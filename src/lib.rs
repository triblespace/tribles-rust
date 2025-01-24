#![doc = include_str!("../README.md")]
#![cfg_attr(nightly, feature(rustc_attrs))]
#![cfg_attr(nightly, feature(decl_macro))]

pub mod blob;
pub mod id;
pub mod metadata;
pub mod namespace;
pub mod patch;
pub mod pile;
pub mod prelude;
pub mod query;
pub mod remote;
pub mod trible;
pub mod value;

pub mod examples;
