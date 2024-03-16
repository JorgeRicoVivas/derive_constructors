use syn::{Attribute, DataStruct, DeriveInput, parse_str, Type};
use quote::{quote, ToTokens};
use proc_macro2::Ident;
use proc_macro::TokenStream;
use std::collections::HashMap;
use convert_case::{Case, Casing};
use crate::utils::idents_and_groups_from;
use crate::utils::{ExpectElseOption, ExpectElseResult, extract_token_stream_of_attribute, find_attribute, print_info};

pub(crate) struct FieldsInfo {
    pub(crate) fields_names: Vec<Ident>,
    pub(crate) fields_types: Vec<Type>,
    pub(crate) no_from_fields: Vec<Ident>,
    pub(crate) no_from_fields_initializers: Vec<proc_macro2::TokenStream>,
}

impl FieldsInfo {
    pub(crate) fn new_from_derive_data_struct(data: &DataStruct) -> FieldsInfo {
        let (fields_names, fields_types) = data.fields.iter()
            .filter(|field| find_attribute(&field.attrs, "no_from").is_none())
            .map(|field| (field.ident.clone().unwrap(), field.ty.clone()))
            .unzip::<_, _, Vec<_>, Vec<_>>();
        print_info(|| "Fields", || format!("{fields_names:#?}"));

        let (no_from_fields, no_from_fields_initializers) = data.fields.iter()
            .map(|field| (field, find_attribute(&field.attrs, "no_from")))
            .filter(|(_, attribute_opt)| attribute_opt.is_some())
            .map(|(field, attribute_opt)| (field, attribute_opt.unwrap()))
            .map(|(field, attribute)| {
                let initializer = extract_token_stream_of_attribute(attribute).map(Into::into).unwrap_or_else(|| quote!(core::default::Default::default()));
                print_info(|| "Ident", || format!("{:#?}", field.ident.as_ref()));
                (field.ident.clone().unwrap(), initializer)
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();
        print_info(|| "No from fields", || format!("{no_from_fields:#?}"));

        let info = FieldsInfo { fields_names, fields_types, no_from_fields, no_from_fields_initializers };
        info
    }

    pub(crate) fn new_from_macro_attribute_info(data: &DataStruct, attr_contents: &mut HashMap<String, proc_macro2::TokenStream>) -> FieldsInfo {
        let mut idents_and_groups = attr_contents;
        print_info(|| "Info", || format!("{idents_and_groups:#?}"));

        let (mut no_from_fields, mut no_from_initializers) = idents_and_groups.remove("defaults")
            .map(|token| idents_and_groups_from(token.to_token_stream())
                .expect_else(|_| "Could not resolve groups and descriptions inside attribute 'defaults'")
                .into_iter()
                .map(|(ident, group)| (ident, group))
                .unzip::<_, _, Vec<_>, Vec<_>>())
            .unwrap_or_else(|| Default::default());

        let fields_in_use = idents_and_groups.remove("fields")
            .map(|fields_token| fields_token.to_string()
                .split(",")
                .map(|field_name| parse_str::<Ident>(field_name.trim()).unwrap())
                .collect::<Vec<_>>()
            )
            .unwrap_or_else(|| data.fields.iter()
                .filter(|field| !no_from_fields.contains(field.ident.as_ref().unwrap()))
                .map(|field| field.ident.clone().unwrap()).collect());

        let fields_in_use_types = fields_in_use.iter()
            .map(|constructor_field| {
                data.fields.iter().filter(|field| field.ident.as_ref().is_some_and(|ident| ident.eq(constructor_field))).next().unwrap()
                    .ty.clone()
            })
            .collect::<Vec<_>>();

        let (unreached_field, unreached_initializers) =
            data.fields.iter()
                .map(|field| field.ident.clone().unwrap())
                .filter(|name| !fields_in_use.contains(name) && !no_from_fields.contains(name))
                .map(|name| (name, quote! {core::default::Default::default()}))
                .unzip::<_, _, Vec<_>, Vec<_>>();

        no_from_fields.extend(unreached_field.into_iter());
        no_from_initializers.extend(unreached_initializers.into_iter());

        FieldsInfo {
            fields_names: fields_in_use,
            fields_types: fields_in_use_types,
            no_from_fields: no_from_fields,
            no_from_fields_initializers: no_from_initializers,
        }
    }
}

pub(crate) struct TryFromInfo {
    pub(crate) error_enum_metadata: proc_macro2::TokenStream,
    pub(crate) error_enum_name: Ident,
    pub(crate) error_types: Vec<Ident>,
    pub(crate) try_from_types: Vec<Ident>,
}

impl TryFromInfo {
    fn error_types_and_try_from_types(fields_names: &Vec<Ident>) -> (Vec<Ident>, Vec<Ident>) {
        let error_types = fields_names.iter()
            .map(|field_name|
                syn::parse_str::<Ident>(
                    &format!("{}Error", field_name.to_string().to_case(Case::Pascal))
                ).expect_else(|_| format!("Could not create enum error's identifier name for field {field_name}")))
            .collect::<Vec<_>>();

        let try_from_types = fields_names.iter()
            .map(|field_name|
                syn::parse_str::<Ident>(
                    &format!("{}From", field_name.to_string().to_case(Case::Pascal))
                ).expect_else(|_| format!("Could not create enum error's identifier name for field {field_name}")))
            .collect::<Vec<_>>();
        (error_types, try_from_types)
    }

    pub(crate) fn new(name: &Ident, attrs: &Vec<Attribute>, fields_names: &Vec<Ident>) -> TryFromInfo {
        let error_enum_metadata: proc_macro2::TokenStream = find_attribute(&attrs, "enum_error_meta")
            .map(|attribute| extract_token_stream_of_attribute(attribute)
                .expect_else(|| "Could not parse content of the #[enum_error_meta] attribute"))
            .unwrap_or_else(|| TokenStream::new()).into();

        let error_enum_name = syn::parse_str::<Ident>
            (&format!("{}TryFromError", name.to_string().to_case(Case::Pascal)))
            .expect_else(|_| format!("Could not create enum error's identifier name"));

        let (error_types, try_from_types) = Self::error_types_and_try_from_types(fields_names);

        Self {
            error_enum_metadata,
            error_enum_name,
            error_types,
            try_from_types,
        }
    }


    pub(crate) fn new_from_macro_attribute_info(derive_input: &DeriveInput, fields_info: &FieldsInfo, constructor_fn_name: Option<&Ident>, attr_contents: &mut HashMap<String, proc_macro2::TokenStream>) -> Self {
        let error_enum_metadata = attr_contents.remove("error_enum_metadata")
            .unwrap_or_else(proc_macro2::TokenStream::new);
        let error_enum_name = attr_contents.remove("error_enum_named")
            .map(|name| syn::parse::<Ident>(name.into()).unwrap())
            .unwrap_or_else(|| {
                let constructor_fn_name = constructor_fn_name.map(|constructor_name| constructor_name.to_string()).unwrap_or_else(|| "TryFrom".to_string());
                syn::parse_str::<Ident>
                    (&format!("{}_{}_error", derive_input.ident, constructor_fn_name).to_case(Case::Pascal))
                    .expect_else(|_| format!("Could not create enum error's identifier name"))
            });
        let (error_types, try_from_types) = Self::error_types_and_try_from_types(&fields_info.fields_names);

        Self {
            error_enum_metadata,
            error_enum_name,
            error_types,
            try_from_types,
        }
    }
}

