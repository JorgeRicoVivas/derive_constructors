use proc_macro::{TokenStream, TokenTree};
use proc_macro2::{Ident, Punct};
use quote::quote;
use syn::{Attribute, Error, Variant};
use syn::parse::ParseStream;
use std::iter::zip;

pub(crate) fn print_info<TNameRet, TInfoRet, TName, TInfo>(_name: TName, _info: TInfo)
    where TNameRet: ToString,
          TInfoRet: ToString,
          TName: FnOnce() -> TNameRet,
          TInfo: FnOnce() -> TInfoRet
{
    //eprintln!("--------------------- {} ---------------------\n", (_name()).to_string());
    //eprintln!("{}\n", (_info()).to_string());
    //eprintln!("-------------------------------------------------------------\n");
}

pub(crate) trait ExpectElseResult<T, E> {
    fn expect_else<TInfoRet: ToString, TInfo: FnOnce(&E) -> TInfoRet>(self, info: TInfo) -> T;
}

impl<T, E: core::fmt::Debug> ExpectElseResult<T, E> for Result<T, E> {
    fn expect_else<TInfoRet: ToString, TInfo: FnOnce(&E) -> TInfoRet>(self, info: TInfo) -> T {
        if self.is_ok() {
            self.expect("")
        } else {
            let error_info = match &self {
                Err(error) => info(error).to_string(),
                _ => { panic!("Unreachable point"); }
            };
            self.expect(&error_info)
        }
    }
}


pub(crate) trait ExpectElseOption<T> {
    fn expect_else<TInfoRet: ToString, TInfo: FnOnce() -> TInfoRet>(self, info: TInfo) -> T;
}

impl<T> ExpectElseOption<T> for Option<T> {
    fn expect_else<TInfoRet: ToString, TInfo: FnOnce() -> TInfoRet>(self, info: TInfo) -> T {
        if self.is_some() {
            self.expect("")
        } else {
            self.expect(&info().to_string())
        }
    }
}


pub(crate) fn extract_token_stream_of_attribute(variants_value_attr: &Attribute) -> Option<TokenStream> {
    let mut token_stream = None;
    let _ = variants_value_attr.parse_args_with(|input: ParseStream| {
        token_stream = Some(TokenStream::from(input.cursor().token_stream()));
        Ok(())
    });
    token_stream
}

pub(crate) fn fields_as_const_defaults_tokens(variant: &Variant) -> Option<proc_macro2::TokenStream> {
    let internal_fields_as_default = variant.fields
        .iter()
        .map(|field| {
            field.ident.as_ref()
                .map(|field_name| quote!(#field_name (const_default::ConstDefault::DEFAULT)))
                .unwrap_or_else(|| quote!((const_default::ConstDefault::DEFAULT)))
        })
        .reduce(|prev_token, next_token| quote!(#prev_token, #next_token));
    internal_fields_as_default
}

pub(crate) fn parse_separated_idents(input: ParseStream) -> Result<Vec<Ident>, Error> {
    let mut idents = Vec::new();
    while !input.is_empty() {
        match input.parse::<Ident>() {
            Ok(ident) => idents.push(ident),
            Err(_) => {
                if input.parse::<Punct>().is_err() {
                    return Err(Error::new(input.span(), "Not a feature or a punctuation sign"));
                }
            }
        }
    }
    Ok(idents)
}

pub(crate) fn find_attribute_last_in_path<'attr>(attrs: &'attr Vec<Attribute>, attribute_ident: &str) -> Option<&'attr Attribute> {
    attrs.iter()
        .filter(|attribute| attribute.path.segments.iter().last().is_some_and(|segment| segment.ident.to_string().eq(attribute_ident)))
        .next()
}

pub(crate) fn find_attribute<'attr>(attrs: &'attr Vec<Attribute>, attribute_ident: &str) -> Option<&'attr Attribute> {
    attrs.iter()
        .filter(|attribute| attribute.path.is_ident(attribute_ident))
        .next()
}

pub fn idents_and_groups_from<TTokenStream: Into<TokenStream>>(token_stream: TTokenStream) -> Result<Vec<(Ident, proc_macro2::TokenStream)>, syn::Error> {
    let token_stream = token_stream.into();
    let is_start = true;
    let mut parse_phase = ParsePhase::Ident;
    let mut idents = Vec::new();
    let mut groups = Vec::new();
    for token in token_stream.into_iter() {
        match &parse_phase {
            ParsePhase::Ident => match token {
                TokenTree::Ident(ref ident) => idents.push(syn::parse::<Ident>(TokenStream::from(token)).unwrap()),
                _ => Err(syn::Error::new(token.span().into(),
                                         format!("Expected a name{}, for example, 'group_name' in 'group_name(my group's description)'",
                                                 idents.last().map(|ident| format!(" after group named '{ident}'")).unwrap_or_else(|| String::new())
                                         ),
                ))?,
            },
            ParsePhase::Group => match token {
                TokenTree::Group(group) => groups.push(group.stream().into()),
                _ => Err(syn::Error::new(token.span().into(),
                                         format!("Expected a group after group named '{}', for example, '(my group's description)' in 'group_name(my group's description)'",
                                                 idents.last().unwrap()
                                         ),
                ))?,
            },
            ParsePhase::Punct => match token {
                TokenTree::Punct(_) => {}
                _ => Err(syn::Error::new(token.span().into(),
                                         format!("Expected a separator after group named '{}', for example, the comma (',') in 'group1(desc), group2(desc)'",
                                                 idents.last().unwrap()
                                         ),
                ))?,
            },
        };
        parse_phase.advance_to_next();
    }
    if idents.len() != groups.len() {
        let last_ident = idents.last().unwrap();
        Err(syn::Error::new(last_ident.span().into(),
                            "Expected a group, for example, '(my group's description)' in 'group_name(my group's description)'"))?
    }
    let res = zip(idents, groups)
        .into_iter()
        .collect::<Vec<_>>();
    Ok(res)
}


enum ParsePhase {
    Ident,
    Group,
    Punct,
}

impl ParsePhase {
    fn advance_to_next(&mut self) {
        let next = match self {
            ParsePhase::Ident => ParsePhase::Group,
            ParsePhase::Group => ParsePhase::Punct,
            ParsePhase::Punct => ParsePhase::Ident,
        };
        *self = next;
    }
}
