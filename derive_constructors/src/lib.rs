//! [![crates.io](https://img.shields.io/crates/v/derive_constructors.svg)](https://crates.io/crates/derive_constructors)
//! [![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/JorgeRicoVivas/derive_constructors/rust.yml)](https://github.com/JorgeRicoVivas/derive_constructors/actions)
//! [![docs.rs](https://img.shields.io/docsrs/derive_constructors)](https://docs.rs/derive_constructors/latest/derive_constructors/)
//! [![GitHub License](https://img.shields.io/github/license/JorgeRicoVivas/derive_constructors)](https://github.com/JorgeRicoVivas/derive_constructors/blob/main/LICENSE)
//!
//! > *You are reading the documentation for derive_constructors version 1.0.0*
//!
//! Allows to derive multiple constructor functions and implement the [From] and [TryFrom] traits
//! for a struct by giving simple information such as their field's names.
//!
//! Also allows to derive [From] for enums.
//!
//! ## 1 The Derive macros for structs: From and TryFrom
//! > Ref: [derive_constructors_proc::From], [derive_constructors_proc::TryFrom]
//!
//! These allow you to Derive the [From] and [TryFrom] traits where a tuple of the fields are passed
//! to the [From::from] or [TryFrom::try_from] function, for example
//!
//! ``` rust
//! #[derive(derive_constructors::From, PartialEq, Debug)]
//! struct CharacterInfo{
//!     name: String,
//!     age: u8,
//!     #[no_from]
//!     times_appeared: u8,
//!     #[no_from(4)]
//!     years_studied: u8
//! }
//!
//! let character_using_from = CharacterInfo::from(("Jorge".to_string(), 23));
//! let expected_character = CharacterInfo { name: "Jorge".to_string(), age: 23, times_appeared: 0, years_studied: 4};
//! assert_eq!(character_using_from, expected_character);
//! ```
//! ## 2 The Attribute Macro for structs:
//! > Ref: [derive_constructors_proc::constructor]
//!
//! Allows you to define a constructor function, inside the proc attribute you can customize the
//! implementation by giving this information following attributes (Note every attribute is
//! optional):
//!
//! - named: Name of the function, constructor functions are usually named like
//! 'with_*name of the fields*', as calling them are quite readable, like
//! ```CharacterInfo::with_name_and_age("Jorge", 23)```. <br> Note: If this field isn't given,
//! instead of implementing a 'with_*' constructor function, it will implement the [From] or
//! [TryFrom] trait.
//!
//! - pattern (values: [From, TryFrom], default: From):
//!     - When using the From pattern, the function receives fields as parameters and returns this
//! struct with said values, this is what you'll be looking for most of the time.
//!     - When using the TryFrom pattern, the functions receives types that implement
//! Into<YourField1>, Into<YourField2>..., returning a [Ok] with your struct if every field could
//! successfully be turned to your field, in case not, it will return [Err] with an enum telling
//! which field couldn't get initialized and the Error why it didn't, see examples below for this.
//!
//! - fields (default: All fields not included in the '```defaults```' attribute): Name of the
//! fields you want to create your constructor for, for example: ```fields(age, name)``` could
//! result in a function like: ```fn new(age: u8, name: String) -> CharacterInfo```.
//!
//! - defaults: Tells how to initialize fields not covered in the ```fields``` attribute, for
//! example ```defaults(years_studied(4))```. <br>If a field isn't either on the ```fields``` or
//! ```defaults``` attributes, it would count as it was initialized through [Default::default], this
//! means, the ```times_appeared``` field that hasn't been covered will be init as 0 (since
//! u8::default() is 0).
//!
//! - error_enum_named (Only for the TryFrom pattern): Specifies the name for the enum error that
//! it's returned the TryFrom function fails.
//!
//! - error_enum_metadata (Only for the TryFrom pattern): Declares the metadata for the enum error
//! that it's returned the TryFrom function fails, you will most likely want to write
//! ```error_enum_metadata(#[derive(Debug)])``` in there.
//! <br><br>
//!
//! ## 2.1 Example 1: Empty constructor
//!
//! If you just apply the [constructor] attribute, it will just implement the [From] trait where it
//! will take a tuple formed out of all your fields, in this case,
//! ```from(value: (String, u8)) -> CharacterInfo```.
//!
//! ``` rust
//! #[derive(Debug, PartialEq)]
//! #[derive_constructors::constructor]
//! struct CharacterInfo{
//!     name: String,
//!     age: u8,
//! }
//!
//! let character_using_from = CharacterInfo::from(("Jorge".to_string(), 23));
//! let expected_character = CharacterInfo { name: "Jorge".to_string(), age: 23 };
//! assert_eq!(character_using_from, expected_character);
//! ```
//! <br>
//!
//! ## 2.2 Example 2: A 'new' constructor using specific fields
//!
//! The following example creates a function named ```new(name: String, age: u8) -> CharacterInfo```
//! .<br><br>
//! Since ```years_studied``` is specified, it will be initialized as 4, and since
//! ```times_appeared``` is not, it will be initialized as u8::default() (which is 0).
//!
//! ``` rust
//! #[derive(Debug, PartialEq)]
//! #[derive_constructors::constructor(named(new), fields(name, age), defaults(years_studied(4)))]
//! struct CharacterInfo{
//!     name: String,
//!     age: u8,
//!     times_appeared: u8,
//!     years_studied: u8
//! }
//!
//! let character_using_from = CharacterInfo::new("Jorge".to_string(), 23);
//! let expected_character = CharacterInfo { name: "Jorge".to_string(), age: 23, times_appeared: 0, years_studied: 4};
//! assert_eq!(character_using_from, expected_character);
//! ```
//! <br>
//!
//! ## 2.3 Example 3: A 'new' constructor with the TryFrom pattern
//!
//! The following example creates a function named ```new(name: T where String: TryFrom<T>, age: U
//! where String: TryFrom<U>) -> Result<CharacterInfo, MyEnumError>```.<br><br>
//! Since ```years_studied``` is specified, it will be initialized as 4, and since
//! ```times_appeared``` is not, it will be initialized as u8::default() (which is 0).<br><br>
//! In case of an error, it returns a variant of an enum named ```MyEnumError```, this enum is
//! specified to derive [Debug] and [PartialEq].
//!
//! ``` rust
//! let character_using_try_from = CharacterInfo::new("Jorge", 23_u16).unwrap();
//! let expected_character = CharacterInfo { name: "Jorge".to_string(), age: 23, times_appeared: 0, years_studied: 4};
//! assert_eq!(character_using_try_from, expected_character);
//!
//! let produced_error = u8::try_from(23000_u16).unwrap_err();
//! let forced_error_using_try_from = CharacterInfo::new("Jorge", 23000_u16).unwrap_err();
//! let expected_error_on_try_from = MyEnumError::AgeError(produced_error);
//! assert_eq!(forced_error_using_try_from, expected_error_on_try_from);
//!
//! #[derive(Debug, PartialEq)]
//! #[derive_constructors::constructor(
//!     named(new),
//!     fields(name, age),
//!     defaults(years_studied(4)),
//!     pattern(TryFrom),
//!     error_enum_named(MyEnumError),
//!     error_enum_metadata(#[derive(Debug, PartialEq)])
//! )]
//! struct CharacterInfo{
//!     name: String,
//!     age: u8,
//!     times_appeared: u8,
//!     years_studied: u8,
//! }
//! ```
//!
//! ## 3 The Derive macro for enums: From
//!
//! > Ref: [derive_constructors_proc::From]
//!
//! This implement the From trait for each enum by creating a From::from function on each taking
//! every field as value, for example:
//!
//! ```rust
//! #[derive(derive_constructors::From, Debug, PartialEq)]
//! enum MyValue{
//!     StaticString(&'static str),
//!     Number(i32),
//!     Boolean(bool),
//! }
//!
//! let scattered_values = vec![MyValue::from("Age "), MyValue::from(23), MyValue::from(", over age "), MyValue::from(true)];
//! let specified = vec![MyValue::StaticString("Age "), MyValue::Number(23), MyValue::StaticString(", over age "), MyValue::Boolean(true)];
//! assert_eq!(scattered_values, specified);
//! ```

extern crate derive_constructors_proc;

pub use derive_constructors_proc::*;