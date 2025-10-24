#![doc = include_str!("../README.md")]

pub use triblespace_core::attribute;
pub use triblespace_core::blob;
pub use triblespace_core::debug;
pub use triblespace_core::examples;
pub use triblespace_core::id;
pub use triblespace_core::metadata;
pub use triblespace_core::patch;
pub use triblespace_core::query;
pub use triblespace_core::repo;
pub use triblespace_core::trible;
pub use triblespace_core::value;

pub mod prelude {
    pub use triblespace_core::prelude::*;
}

pub use triblespace_core::arrayvec;
pub use triblespace_core::macro_pub;
pub use triblespace_core::macros;

pub use triblespace_core::attributes;
pub use triblespace_core::entity;
pub use triblespace_core::path;
pub use triblespace_core::pattern;
pub use triblespace_core::pattern_changes;
