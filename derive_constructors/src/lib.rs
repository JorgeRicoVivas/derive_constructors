//! [![crates.io](https://img.shields.io/crates/v/derive_constructors.svg)](https://crates.io/crates/derive_constructors)
//! [![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/JorgeRicoVivas/derive_constructors/rust.yml)](https://github.com/JorgeRicoVivas/derive_constructors/actions)
//! [![docs.rs](https://img.shields.io/docsrs/derive_constructors)](https://docs.rs/derive_constructors/latest/derive_constructors/)
//! [![GitHub License](https://img.shields.io/github/license/JorgeRicoVivas/derive_constructors)](https://github.com/JorgeRicoVivas/derive_constructors/blob/main/LICENSE)
//!
//! > *You are reading the documentation for derive_constructors version 1.0.0*
//!
//! Allows to derive multiple constructor functions and implement the [From] and [TryFrom] traits
//! for a struct by giving simple information such as fields.
//!
//! Also allows to derive [From] for enums.
//!
//! ## 1 The Derive macros for structs: [derive_constructors_proc::derive_from] From and TryFrom
//!
//! These allow you to Derive the [From] and [TryFrom] traits where a tuple of the fields are passed
//! to the [From::from] or [TryFrom::try_from] function, for example
//!
//! ``` rust
//! use derive_constructors_proc::From;
//!
//! #[derive(From, PartialEq, Debug)]
//! struct CharacterInfo{
//!     name: String,
//!     age: u8,
//!     #[no_from]
//!     times_appeared: u8,
//! }
//!
//! fn test(){
//!     let character_using_from = CharacterInfo::from(("Jorge".to_string(), 23));
//!     let expected_character = CharacterInfo { name: "Jorge".to_string(), age: 23, times_appeared: 0 };
//!     assert_eq!(character_using_from, expected_character);
//! }
//! ```
//! ## 2 The Attribute Macro for structs: [constructor]
//!
//! the []
//!
//!

extern crate derive_constructors_proc;

pub use derive_constructors_proc::*;