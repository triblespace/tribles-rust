#![doc = include_str!("../README.md")]
#![cfg_attr(nightly, feature(rustc_attrs, decl_macro, file_lock))]

extern crate self as tribles;

#[cfg(not(all(target_pointer_width = "64", target_endian = "little")))]
compile_error!("tribles-rust requires a 64-bit little-endian target");

pub mod blob;
pub mod id;
pub mod metadata;
pub mod namespace;
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
pub use tribles_macros as macros;
// Re-export proc-macros at the crate root so they are available within the
// crate without requiring explicit `use` statements at every call site.
pub use tribles_macros::pattern;
pub use tribles_macros::entity;
pub use tribles_macros::pattern_changes;
pub use tribles_macros::path;

#[cfg(kani)]
#[path = "../proofs/mod.rs"]
mod proofs;

// Let's add the readme example as a test
#[cfg(test)]
mod readme_example {
    use crate::examples::literature;
    use crate::prelude::*;

    #[test]
    fn readme_example() -> Result<(), Box<dyn std::error::Error>> {
        let mut blobs = MemoryBlobStore::new();
        let mut set = TribleSet::new();

        let author_id = ufoid();

        // Note how the entity macro returns TribleSets that can be cheaply merged
        // into our existing dataset.
        set += entity!(&author_id, {
            literature::firstname: "Frank",
            literature::lastname: "Herbert",
        });

        set += entity!({
            literature::title: "Dune",
            literature::author: &author_id,
            literature::quote: blobs.put("Deep in the human unconscious is a \
            pervasive need for a logical universe that makes sense. \
            But the real universe is always one step beyond logic.").unwrap(),
            literature::quote: blobs.put("I must not fear. Fear is the \
            mind-killer. Fear is the little-death that brings total \
            obliteration. I will face my fear. I will permit it to \
            pass over me and through me. And when it has gone past I \
            will turn the inner eye to see its path. Where the fear \
            has gone there will be nothing. Only I will remain.").unwrap(),
        });

        let title = "Dune";

        // We can then find all entities matching a certain pattern in our dataset.
        for (_, f, l, q) in find!(
        (author: (), first: String, last: Value<_>, quote),
        pattern!(&set, [
            { author @
                literature::firstname: first,
                literature::lastname: last
            },
            {
                literature::title: (title),
                literature::author: author,
                literature::quote: quote
            }]))
        {
            let q: View<str> = blobs.reader().unwrap().get(q).unwrap();
            let q = q.as_ref();

            println!("'{q}'\n - from {title} by {f} {}.", l.from_value::<&str>())
        }
        Ok(())
    }
}
