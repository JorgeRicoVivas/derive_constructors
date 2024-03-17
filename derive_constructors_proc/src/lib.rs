use proc_macro::TokenStream;
use std::collections::HashMap;
use std::str::FromStr;
use convert_case::Casing;
use proc_macro2::Ident;
use quote::{quote, ToTokens};
use syn::{Data, DataEnum, DeriveInput, parse_macro_input};
use syn::parse::Parse;
use parsing_structs::{FieldsInfo, TryFromInfo};
use crate::utils::{ExpectElseOption, ExpectElseResult, print_info};

mod utils;

/// Allows you to define a constructor function, inside the proc attribute you can customize the
/// implementation by giving this information following attributes (Note every attribute is
/// optional):
///
/// - named: Name of the function, constructor functions are usually named like
/// 'with_*name of the fields*', as calling them are quite readable, like
/// ```CharacterInfo::with_name_and_age("Jorge", 23)```. <br> Note: If this field isn't given,
/// instead of implementing a 'with_*' constructor function, it will implement the [From] or
/// [TryFrom] trait.
///
/// - pattern (values: [From, TryFrom], default: From):
///     - When using the From pattern, the function receives fields as parameters and returns this
/// struct with said values, this is what you'll be looking for most of the time.
///     - When using the TryFrom pattern, the functions receives types that implement
/// Into<YourField1>, Into<YourField2>..., returning a [Ok] with your struct if every field could
/// successfully be turned to your field, in case not, it will return [Err] with an enum telling
/// which field couldn't get initialized and the Error why it didn't, see examples below for this.
///
/// - fields (default: All fields not included in the '```defaults```' attribute): Name of the
/// fields you want to create your constructor for, for example: ```fields(age, name)``` could
/// result in a function like: ```fn new(age: u8, name: String) -> CharacterInfo```.
///
/// - defaults: Tells how to initialize fields not covered in the ```fields``` attribute, for
/// example ```defaults(years_studied(4))```. <br>If a field isn't either on the ```fields``` or
/// ```defaults``` attributes, it would count as it was initialized through [Default::default], this
/// means, the ```times_appeared``` field that hasn't been covered will be init as 0 (since
/// u8::default() is 0).
///
/// - error_enum_named (Only for the TryFrom pattern): Specifies the name for the enum error that
/// it's returned the TryFrom function fails.
///
/// - error_enum_metadata (Only for the TryFrom pattern): Declares the metadata for the enum error
/// that it's returned the TryFrom function fails, you will most likely want to write
/// ```error_enum_metadata(#[derive(Debug)])``` in there.
/// <br><br>
///
/// ## 2.1 Example 1: Empty constructor
///
/// If you just apply the [constructor] attribute, it will just implement the [From] trait where it
/// will take a tuple formed out of all your fields, in this case,
/// ```from(value: (String, u8)) -> CharacterInfo```.
///
/// ``` rust
/// #[derive(Debug, PartialEq)]
/// #[derive_constructors_proc::constructor]
/// struct CharacterInfo{
///     name: String,
///     age: u8,
/// }
///
/// let character_using_from = CharacterInfo::from(("Jorge".to_string(), 23));
/// let expected_character = CharacterInfo { name: "Jorge".to_string(), age: 23 };
/// assert_eq!(character_using_from, expected_character);
/// ```
/// <br>
///
/// ## 2.2 Example 2: A 'new' constructor using specific fields
///
/// The following example creates a function named ```new(name: String, age: u8) -> CharacterInfo```
/// .<br><br>
/// Since ```years_studied``` is specified, it will be initialized as 4, and since
/// ```times_appeared``` is not, it will be initialized as u8::default() (which is 0).
///
/// ``` rust
/// #[derive(Debug, PartialEq)]
/// #[derive_constructors_proc::constructor(named(new), fields(name, age), defaults(years_studied(4)))]
/// struct CharacterInfo{
///     name: String,
///     age: u8,
///     times_appeared: u8,
///     years_studied: u8
/// }
///
/// let character_using_from = CharacterInfo::new("Jorge".to_string(), 23);
/// let expected_character = CharacterInfo { name: "Jorge".to_string(), age: 23, times_appeared: 0, years_studied: 4};
/// assert_eq!(character_using_from, expected_character);
/// ```
/// <br>
///
/// ## 2.3 Example 3: A 'new' constructor with the TryFrom pattern
///
/// The following example creates a function named ```new(name: T where String: TryFrom<T>, age: U
/// where String: TryFrom<U>) -> Result<CharacterInfo, MyEnumError>```.<br><br>
/// Since ```years_studied``` is specified, it will be initialized as 4, and since
/// ```times_appeared``` is not, it will be initialized as u8::default() (which is 0).<br><br>
/// In case of an error, it returns a variant of an enum named ```MyEnumError```, this enum is
/// specified to derive [Debug] and [PartialEq].
///
/// ``` rust
/// let character_using_try_from = CharacterInfo::new("Jorge", 23_u16).unwrap();
/// let expected_character = CharacterInfo { name: "Jorge".to_string(), age: 23, times_appeared: 0, years_studied: 4};
/// assert_eq!(character_using_try_from, expected_character);
///
/// let produced_error = u8::try_from(23000_u16).unwrap_err();
/// let forced_error_using_try_from = CharacterInfo::new("Jorge", 23000_u16).unwrap_err();
/// let expected_error_on_try_from = MyEnumError::AgeError(produced_error);
/// assert_eq!(forced_error_using_try_from, expected_error_on_try_from);
///
/// #[derive(Debug, PartialEq)]
/// #[derive_constructors_proc::constructor(
///     named(new),
///     fields(name, age),
///     defaults(years_studied(4)),
///     pattern(TryFrom),
///     error_enum_named(MyEnumError),
///     error_enum_metadata(#[derive(Debug, PartialEq)])
/// )]
/// struct CharacterInfo{
///     name: String,
///     age: u8,
///     times_appeared: u8,
///     years_studied: u8,
/// }
/// ```
#[proc_macro_attribute]
pub fn constructor(attr: TokenStream, mut item: TokenStream) -> TokenStream {
    let item_cloned = item.clone();
    let derive_input = parse_macro_input!(item_cloned as DeriveInput);
    let data = match derive_input.data.clone() {
        Data::Struct(data) => data,
        _ => panic!("This attribute macro is only implemented for structs"),
    };

    let mut attr_contents = utils::idents_and_groups_from(attr.clone())
        .expect_else(|_| "Could not resolve groups and descriptions")
        .into_iter()
        .map(|(ident, group)| (ident.to_string(), group))
        .collect::<HashMap<_, _>>();

    let constructor_fn_name = attr_contents.remove("named")
        .map(|constructor_name| syn::parse::<Ident>(constructor_name.into())
            .expect_else(|_| "Could not get name for constructor's function"));

    let constructor_pattern = attr_contents.remove("pattern")
        .map(|pattern| pattern.to_string().to_lowercase())
        .unwrap_or_else(|| "from".to_string());
    let constructor_pattern = match constructor_pattern.as_str() {
        "from" => Pattern::From,
        "tryfrom" => Pattern::TryFrom,
        wrong_pattern => panic!("This constructor is asking for a pattern by the name of '{wrong_pattern}', the only patterns available are 'From' and 'TryFrom' ")
    };

    let fields_info = FieldsInfo::new_from_macro_attribute_info(&data, &mut attr_contents);

    let ex = match constructor_pattern {
        Pattern::From => {
            tokens_for__from__for_struct(derive_input.ident, fields_info, constructor_fn_name)
        }
        Pattern::TryFrom => {
            let try_from_info = TryFromInfo::new_from_macro_attribute_info(&derive_input, &fields_info, constructor_fn_name.as_ref(), &mut attr_contents);
            tokens_for__try_from__for_struct(derive_input.ident, fields_info, try_from_info, constructor_fn_name)
        }
    };


    item.extend(Into::<TokenStream>::into(ex));
    item
}

enum Pattern {
    From,
    TryFrom,
}

/// On structs it allows to Derive the [From] trait where a tuple of the fields are passed to the
/// [From::from], for example:
///
/// ``` rust
/// #[derive(derive_constructors_proc::From, PartialEq, Debug)]
/// struct CharacterInfo{
///     name: String,
///     age: u8,
///     #[no_from]
///     times_appeared: u8,
///     #[no_from(4)]
///     years_studied: u8
/// }
///
/// let character_using_from = CharacterInfo::from(("Jorge".to_string(), 23));
/// let expected_character = CharacterInfo { name: "Jorge".to_string(), age: 23, times_appeared: 0, years_studied: 4};
/// assert_eq!(character_using_from, expected_character);
/// ```
/// <br><br>
/// 
/// On enums it implement the [From] trait by creating a From::from function for each variant taking
/// it's values as parameters, for example:
///
/// ```rust
/// #[derive(derive_constructors_proc::From, Debug, PartialEq)]
/// enum MyValue{
///     StaticString(&'static str),
///     Number(i32),
///     Boolean(bool),
/// }
///
/// let scattered_values = vec![MyValue::from("Age "), MyValue::from(23), MyValue::from(", over age "), MyValue::from(true)];
/// let specified = vec![MyValue::StaticString("Age "), MyValue::Number(23), MyValue::StaticString(", over age "), MyValue::Boolean(true)];
/// assert_eq!(scattered_values, specified);
/// ```
#[proc_macro_derive(From, attributes(no_from))]
pub fn derive_from(input: TokenStream) -> TokenStream {
    /*    let cloned_input = input.clone();
    let p = format!("{:#?}\n", parse_macro_input!(cloned_input as DeriveInput));
    print_info(|| "Derive input info", || p);*/
    let DeriveInput { ident, data, .. } = parse_macro_input!(input as DeriveInput);
    match data {
        Data::Union(_) => panic!("The 'From' derive_constructors_proc macro targets structs and enums, consider removing '#[derive_constructors_proc(From)]' for this type"),
        Data::Struct(data_struct) => return tokens_for__from__for_struct(ident, FieldsInfo::new_from_derive_data_struct(&data_struct), None),
        Data::Enum(data_enum) => return tokens_for__from__for_enum(ident, data_enum),
    };
}

/// It derives [TryFrom] trait where a tuple of this struct's fields are passed to the
/// [TryFrom::try_from] function, for example
///
/// ``` rust
/// #[derive(derive_constructors_proc::TryFrom, PartialEq, Debug)]
/// #[enum_error_meta(#[derive(Debug, PartialEq)])]
/// struct CharacterInfo{
///     name: String,
///     age: u8,
///     #[no_from]
///     times_appeared: u8,
///     #[no_from(4)]
///     years_studied: u8
/// }
///
/// let character_using_try_from = CharacterInfo::try_from(("Jorge", 23_u16)).unwrap();
/// let expected_character = CharacterInfo { name: "Jorge".to_string(), age: 23, times_appeared: 0, years_studied: 4};
/// assert_eq!(character_using_try_from, expected_character);
///
/// let produced_error = u8::try_from(23000_u16).unwrap_err();
/// let forced_error_using_try_from = CharacterInfo::try_from(("Jorge", 23000_u16)).unwrap_err();
/// let expected_error_on_try_from = CharacterInfoTryFromError::AgeError(produced_error);
/// assert_eq!(forced_error_using_try_from, expected_error_on_try_from);
/// ```
#[proc_macro_derive(TryFrom, attributes(no_from, enum_error_meta))]
pub fn derive_try_from(input: TokenStream) -> TokenStream {
    let cloned_input = input.clone();
    let p = format!("{:#?}\n", parse_macro_input!(cloned_input as DeriveInput));
    print_info(|| "Derive input info", || p);

    let DeriveInput { ident, data, attrs, .. } = parse_macro_input!(input as DeriveInput);
    match data {
        Data::Union(_) | Data::Enum(_) => panic!("The 'From' derive_constructors_proc macro targets structs, consider removing '#[derive_constructors_proc(From)]' for this type"),
        Data::Struct(data_struct) => {
            let fields_info = FieldsInfo::new_from_derive_data_struct(&data_struct);
            let try_from_info = TryFromInfo::new_from_derive_data_struct(&ident, &attrs, &fields_info.fields_names);
            return tokens_for__try_from__for_struct(ident, fields_info, try_from_info, None);
        }
    };
}


fn tokens_for__try_from__for_struct(name: Ident, fields_info: FieldsInfo, try_from_info: TryFromInfo, constructor_fn_name: Option<Ident>) -> TokenStream {
    let FieldsInfo {
        fields_names,
        fields_types,
        no_from_fields,
        no_from_fields_initializers
    }
        = fields_info;

    let TryFromInfo {
        error_enum_metadata,
        error_enum_name,
        error_types,
        try_from_types
    }
        = try_from_info;


    if constructor_fn_name.is_none() {
        let res = quote! {
            #error_enum_metadata
            pub enum #error_enum_name <#(#error_types),*>{
                #(#error_types (#error_types)),*
            }

            impl <#(#try_from_types , #error_types),*>
                TryFrom<(#(#try_from_types),*)> for #name
                where
                    #(#fields_types : TryFrom< #try_from_types, Error=#error_types > ),*
            {

                type Error = #error_enum_name<#(#error_types),*>;

                fn try_from(value: (#(#try_from_types),*)) -> Result<Self, Self::Error> {

                    let (#(#fields_names),*) = value;
                    #(let #fields_names = <#fields_types>::try_from(#fields_names)
                        .map_err(|error| #error_enum_name::#error_types(error)  )?; )*
                    Ok(
                        #name{
                            #(#fields_names,)*
                            #(#no_from_fields: #no_from_fields_initializers,)*
                        }
                    )
                }
            }
        };
        print_info(|| "Derive input res", || format!("{res}"));
        return res.into();
    }

    let constructor_fn_name = constructor_fn_name.unwrap();
    let res = quote! {
        #error_enum_metadata
        pub enum #error_enum_name <#(#error_types),*>{
            #(#error_types (#error_types)),*
        }

        impl #name {
            pub fn #constructor_fn_name<#(#try_from_types , #error_types),*>(#(#fields_names: #try_from_types),*) -> Result<Self, #error_enum_name<#(#error_types),*>>
                where
                    #(#fields_types : TryFrom< #try_from_types, Error=#error_types > ),*
            {
                    #(let #fields_names = <#fields_types>::try_from(#fields_names)
                        .map_err(|error| #error_enum_name::#error_types(error)  )?; )*
                    Ok(
                        #name{
                            #(#fields_names,)*
                            #(#no_from_fields: #no_from_fields_initializers,)*
                        }
                    )
            }
        }

    };

    print_info(|| "Output", || format!("{res}"));
    res.into()
}

fn tokens_for__from__for_struct(name: Ident, fields_info: FieldsInfo, constructor_fn_name: Option<Ident>) -> TokenStream {
    let FieldsInfo {
        fields_names, fields_types,
        no_from_fields, no_from_fields_initializers
    } = fields_info;

    if constructor_fn_name.is_none() {
        let res = quote! {
            impl core::convert::From<(#(#fields_types),*)> for #name {
                fn from(value: (#(#fields_types),* )) -> Self {
                    let (#(#fields_names),*) = value;
                    Self {
                        #(#fields_names,)*
                        #(#no_from_fields : #no_from_fields_initializers),*
                    }
                }
            }
        };
        print_info(|| "Derive input res", || format!("{res}"));
        return res.into();
    }

    let constructor_fn_name = constructor_fn_name.unwrap();
    let res = quote! {
            impl #name{
                pub fn #constructor_fn_name( #(#fields_names: #fields_types),*  ) -> Self{
                    Self {
                        #(#fields_names,)*
                        #(#no_from_fields : #no_from_fields_initializers),*
                    }
                }
            }
        };
    print_info(|| "Derive input res", || format!("{res}"));
    res.into()
}

fn tokens_for__from__for_enum(name: Ident, enum_data: DataEnum) -> TokenStream {
    let impls = enum_data.variants.iter()
        .filter(|variant| utils::find_attribute(&variant.attrs, "no_from").is_none())
        .map(|variant| {
            let variant_name = &variant.ident;
            let (fieldnames, types) = variant.fields.iter()
                .map(|fields| (fields.ident.as_ref().map(ToString::to_string), fields.ty.to_token_stream()))
                .unzip::<_, _, Vec<_>, Vec<_>>();
            let is_named = fieldnames.get(0).is_some_and(|name| name.is_some());

            let fieldnames = fieldnames
                .into_iter()
                .enumerate()
                .map(|(index, name)| name.unwrap_or_else(|| index.to_string()))
                .map(|name| name.parse::<proc_macro2::TokenStream>().unwrap())
                .collect::<Vec<_>>();
            print_info(|| format!("Variant {variant:?}"),
                       || format!("Is named: {is_named}\n\
                        fields names :{fieldnames:#?}\
                        \n fields types :{types:#?}\n"));
            let indexes =
                match fieldnames.len() {
                    0 => Vec::new(),
                    1 => vec!["".parse::<proc_macro2::TokenStream>().unwrap()],
                    _ => {
                        (0..fieldnames.len())
                            .map(|index| format!(".{index}").parse::<proc_macro2::TokenStream>().unwrap())
                            .collect::<Vec<_>>()
                    }
                };

            let res = quote! {
                impl core::convert::From<(#(#types),*)> for #name {
                    fn from(value: (#(#types),* )) -> Self {
                        Self:: #variant_name { #(#fieldnames : value #indexes),* }
                    }
                }
            };
            print_info(|| "Possible result", || format!("{res}"));
            res
        })
        .collect::<Vec<_>>();
    let res = impls.into_iter().reduce(|token1, token2| {
        quote! { #token1 #token2}
    }).unwrap();
    print_info(|| "Output", || format!("{res}"));
    TokenStream::from(res)
}

pub(crate) mod parsing_structs;