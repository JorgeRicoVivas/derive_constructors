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

/// Hi constructor
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

// Hi From
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

// Hi TryFrom
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
            let try_from_info = TryFromInfo::new(&ident, &attrs, &fields_info.fields_names);
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