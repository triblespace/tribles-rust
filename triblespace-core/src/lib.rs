#![doc = include_str!("../../README.md")]
// Prefer explicit `?` variable bindings in patterns instead of relying on
// parenthesisation. Do not suppress `unused_parens` at the crate level.
#![cfg_attr(nightly, feature(rustc_attrs, decl_macro, file_lock))]

extern crate self as triblespace_core;

#[allow(unused_extern_crates)]
extern crate proc_macro;

#[cfg(not(all(target_pointer_width = "64", target_endian = "little")))]
compile_error!("tribles-rust requires a 64-bit little-endian target");

pub mod attribute;
pub mod blob;
pub mod id;
pub mod import;
pub mod metadata;
pub mod patch;
pub mod prelude;
pub mod query;
pub mod repo;
pub mod trible;
pub mod value;

pub mod debug;
pub mod examples;

// Re-export dependencies used by generated macros so consumers
// don't need to add them explicitly.
pub use arrayvec;
pub use macro_pub;
pub use triblespace_core_macros as macros;
// Re-export proc-macros at the crate root so they are available within the
// crate without requiring explicit `use` statements at every call site.
pub use triblespace_core_macros::attributes;
pub use triblespace_core_macros::entity;
pub use triblespace_core_macros::path;
pub use triblespace_core_macros::pattern;
pub use triblespace_core_macros::pattern_changes;

// Proof harnesses and integration-style documentation tests live in the
// top-level `triblespace` crate so downstream users can depend on
// `triblespace-core` without pulling in additional development-only
// dependencies.
