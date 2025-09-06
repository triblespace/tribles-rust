use std::collections::HashSet;
use crate::pattern;
use crate::entity;
use crate::pattern_changes;
use crate::path;


use fake::faker::lorem::en::Sentence;
use fake::faker::lorem::en::Words;
use tribles::prelude::blobschemas::*;
use tribles::prelude::valueschemas::*;
use tribles::prelude::*;

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;
use tribles::query::ContainsConstraint;
use tribles::repo::BlobStorePut;

pub mod literature {
    #![allow(unused)]
    use crate::prelude::*;
    use crate::value::schemas::hash::Handle;
    use crate::value::schemas::hash::Blake3;
    use crate::blob::schemas::longstring::LongString;
    use crate::value::schemas::shortstring::ShortString;
    use crate::value::schemas::genid::GenId;
    use crate::value::schemas::r256::R256;
    pub const author: crate::field::Field<GenId> = crate::field::Field::from(hex_literal::hex!("8F180883F9FD5F787E9E0AF0DF5866B9"));
    pub const firstname: crate::field::Field<ShortString> = crate::field::Field::from(hex_literal::hex!("0DBB530B37B966D137C50B943700EDB2"));
    pub const lastname: crate::field::Field<ShortString> = crate::field::Field::from(hex_literal::hex!("6BAA463FD4EAF45F6A103DB9433E4545"));
    pub const title: crate::field::Field<ShortString> = crate::field::Field::from(hex_literal::hex!("A74AA63539354CDA47F387A4C3A8D54C"));
    pub const page_count: crate::field::Field<R256> = crate::field::Field::from(hex_literal::hex!("FCCE870BECA333D059D5CD68C43B98F0"));
    pub const quote: crate::field::Field<Handle<Blake3, LongString>> = crate::field::Field::from(hex_literal::hex!("6A03BAF6CFB822F04DA164ADAAEB53F6"));
}

fn main() {
    let mut kb = TribleSet::new();
    let mut mem_store = MemoryBlobStore::new();
    (0..1000000).for_each(|_| {
        let author = fucid();
        let book = fucid();
        kb += entity!(&author, {
            literature::firstname: FirstName(EN).fake::<String>(),
            literature::lastname: LastName(EN).fake::<String>(),
        });
        kb += entity!(&book, {
            literature::author: &author,
            literature::title: Words(1..3).fake::<Vec<String>>().join(" "),
            literature::quote: mem_store.put(Sentence(5..25).fake::<String>()).unwrap()
        });
    });

    let author_names: HashSet<String, _> =
        HashSet::from_iter(["Frank", "Bob"].iter().map(|s| s.to_string()));

    let _result: Vec<_> = find!(
    (firstname: Value<_>, title: Value<_>, author: Value<_>, quote: Value<_>),
    and!(
        author_names.has(firstname),
        pattern!(&kb, [
        {author @
            literature::firstname: firstname,
            literature::lastname: ("Herbert")},
        { literature::author: author,
            literature::title: title,
            literature::quote: quote
        }]))
    )
    .collect();
}
