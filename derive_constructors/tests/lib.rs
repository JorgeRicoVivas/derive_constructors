#![allow(warnings)]

use std::io;
use std::convert::Infallible;
use std::net::{AddrParseError, IpAddr};
use std::num::TryFromIntError;

extern crate derive_constructors_proc;

pub use derive_constructors_proc::*;

// Creates a constructor function named 'with_age_and_name' with parameters (age: u8,
// name: &'static str), the other fields are initialized as {id: "Jorge", family_names: Vec("Rico",
// "Vivas"), appeared_in_movies: 0}, notice 'appeared_in_movies' get's value 0 since u32::default()
// is 0
#[constructor(
named(with_age_and_name),
pattern(From),
fields(age, name),
defaults(
id("Jorge".to_string()),
family_names(vec ! ["Rico", "Vivas"])
)
)]

// Similar to previous constructor, but it uses TryFrom pattern instead of From, this mean the
// parameters now are (age: TryAge, name: TryName) where TryAge and TryFrom are types
// implemententing TryFrom -> u8/&'static str, in other words, types that can turn into u8
// (the type of age) and into &'static str (the type of name).
//
// Note 1: the result is Result<CharacterInfo, EnumError>, where EnumError is an enum indicating
// which value failed and it's error, for example, if introduced age as 500 (A value from which u8
// cannot result), the error would be: EnumError(AgeError(TryFromIntError())).
//
// Note 2:
#[constructor(
named(try_with_age_and_name),
pattern(TryFrom),
fields(age, name),
defaults(
id("Jorge".to_string()),
family_names(vec ! ["Rico", "Vivas"])
),
error_enum_metadata(# [derive(Debug)]),
error_enum_named(GetWithAgeAndNameError),
)]

#[derive(From, Debug)]
pub struct CharacterInfo {
    age: u8,
    name: &'static str,
    #[no_from("Jorge".to_string())]
    id: String,
    #[no_from(vec!["Rico", "Vivas"])]
    family_names: Vec<&'static str>,
    #[no_from]
    appeared_in_movies: u8,
}


#[derive(From, Debug)]
pub enum MyError {
    IO(std::io::Error),
    #[no_from]
    CustomIOError(io::Error),
    MyTwoIo { io_err: io::Error, other_io_err: io::Error },
}

#[derive(Debug, From)]
enum MyNumErrors {
    IO(std::io::Error),
    Infallible(Infallible),
    TryFromIntError(TryFromIntError),
}

pub trait FlattenError<Ok, InternalError, ExternalError> {
    fn flatten_err<Error>(self) -> Result<Ok, Error> where Error: From<InternalError> + From<ExternalError>;

    fn flatten_err_ext(self) -> Result<Ok, ExternalError> where InternalError: Into<ExternalError>;

    fn flatten_err_int(self) -> Result<Ok, InternalError> where ExternalError: Into<InternalError>;
}

impl<Ok, InternalError, ExternalError> FlattenError<Ok, InternalError, ExternalError> for Result<Result<Ok, InternalError>, ExternalError> {
    fn flatten_err<ResError>(self) -> Result<Ok, ResError> where ResError: From<InternalError> + From<ExternalError> {
        match self {
            Err(external_error) => Result::Err(external_error.into()),
            Ok(internal_result) => {
                match internal_result {
                    Err(internal_error) => Result::Err(internal_error.into()),
                    Ok(value) => Result::Ok(value),
                }
            }
        }
    }

    fn flatten_err_ext(self) -> Result<Ok, ExternalError> where InternalError: Into<ExternalError> {
        match self {
            Err(external_error) => Result::Err(external_error),
            Ok(internal_result) => {
                match internal_result {
                    Err(internal_error) => Result::Err(internal_error.into()),
                    Ok(value) => Result::Ok(value),
                }
            }
        }
    }

    fn flatten_err_int(self) -> Result<Ok, InternalError> where ExternalError: Into<InternalError> {
        match self {
            Err(external_error) => Result::Err(external_error.into()),
            Ok(internal_result) => {
                match internal_result {
                    Err(internal_error) => Result::Err(internal_error),
                    Ok(value) => Result::Ok(value),
                }
            }
        }
    }
}


#[test]
fn test() {
    let error =
        std::fs::read_to_string("").map(|_| std::fs::read_to_string("").map(|_| std::fs::read_to_string("")))
            .flatten_err::<MyNumErrors>().flatten_err_ext();
    let error_2 = std::fs::read_to_string("").expect_err("");
    let error_3 = std::fs::read_to_string("").expect_err("");
    println!("{error:#?}");
    println!("{:#?}", MyError::from((error_2, error_3)));
    println!("{:?}", CharacterInfo::from((23, "Jorge")));
    println!("{:?}", CharacterInfo::try_with_age_and_name(2003, "Jorge"));
}
