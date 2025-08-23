#![doc = include_str!("../README.md")]
#![cfg_attr(nightly, feature(rustc_attrs, decl_macro, file_lock))]

extern crate self as tribles;

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
        set += literature::entity!(&author_id, {
            firstname: "Frank",
            lastname: "Herbert",
        });

        set += literature::entity!({
            title: "Dune",
            author: &author_id,
            quote: blobs.put("Deep in the human unconscious is a \
            pervasive need for a logical universe that makes sense. \
            But the real universe is always one step beyond logic.").unwrap(),
            quote: blobs.put("I must not fear. Fear is the \
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
        literature::pattern!(&set, [
            { author @
                firstname: first,
                lastname: last
            },
            {
                title: (title),
                author: author,
                quote: quote
            }]))
        {
            let q: View<str> = blobs.reader().unwrap().get(q).unwrap();
            let q = q.as_ref();

            println!("'{q}'\n - from {title} by {f} {}.", l.from_value::<&str>())
        }
        Ok(())
    }
}
