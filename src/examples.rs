//! This module contains an example namespace for use in the documentation.
//! It is not intended to be used in practice.

use crate::prelude::blobschemas::*;
use crate::prelude::valueschemas::*;
use crate::prelude::*;
use crate::pattern;
use crate::entity;
use crate::pattern_changes;
use crate::path;
pub mod literature {
    #![allow(unused)]
    use crate::prelude::*;
    use crate::value::schemas::hash::Handle;
    use crate::value::schemas::hash::Blake3;
    use crate::blob::schemas::longstring::LongString;
    use crate::value::schemas::shortstring::ShortString;
    use crate::value::schemas::genid::GenId;
    use crate::value::schemas::r256::R256;
    /// The title of a work.
    pub const title: crate::field::Field<ShortString> = crate::field::Field::from(hex_literal::hex!("A74AA63539354CDA47F387A4C3A8D54C"));
    /// A quote from a work.
    pub const quote: crate::field::Field<Handle<Blake3, LongString>> = crate::field::Field::from(hex_literal::hex!("6A03BAF6CFB822F04DA164ADAAEB53F6"));
    /// The author of a work.
    pub const author: crate::field::Field<GenId> = crate::field::Field::from(hex_literal::hex!("8F180883F9FD5F787E9E0AF0DF5866B9"));
    /// The first name of an author.
    pub const firstname: crate::field::Field<ShortString> = crate::field::Field::from(hex_literal::hex!("0DBB530B37B966D137C50B943700EDB2"));
    /// The last name of an author.
    pub const lastname: crate::field::Field<ShortString> = crate::field::Field::from(hex_literal::hex!("6BAA463FD4EAF45F6A103DB9433E4545"));
    /// The number of pages in the work.
    pub const page_count: crate::field::Field<R256> = crate::field::Field::from(hex_literal::hex!("FCCE870BECA333D059D5CD68C43B98F0"));
}

/// Returns a small sample dataset used in the documentation.
pub fn dataset() -> TribleSet {
    let mut set = TribleSet::new();
    let mut blobs = MemoryBlobStore::new();
    let author_id = ufoid();

    set += entity!(&author_id, {
        literature::firstname: "Frank",
        literature::lastname: "Herbert",
    });

    set += entity!({
        literature::title: "Dune",
        literature::author: &author_id,
        literature::quote: blobs
            .put("Deep in the human unconscious is a pervasive need for a logical universe that makes sense. But the real universe is always one step beyond logic.")
            .unwrap(),
        literature::quote: blobs
            .put("I must not fear. Fear is the mind-killer. Fear is the little-death that brings total obliteration. I will face my fear. I will permit it to pass over me and through me. And when it has gone past I will turn the inner eye to see its path. Where the fear has gone there will be nothing. Only I will remain.")
            .unwrap(),
    });

    set
}
