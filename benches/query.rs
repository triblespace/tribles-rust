use crate::entity;
use crate::pattern;
use tribles::prelude::*;

use fake::faker::lorem::en::Sentence;
use fake::faker::lorem::en::Words;
use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;

pub mod literature {
    #![allow(unused)]
    use tribles::prelude::*;
    use tribles::value::schemas::genid::GenId;
    use tribles::value::schemas::hash::Blake3;
    use tribles::value::schemas::hash::Handle;
    use tribles::value::schemas::r256::R256;
    use tribles::value::schemas::shortstring::ShortString;
    use tribles::blob::schemas::longstring::LongString;
    attributes! {
        "8F180883F9FD5F787E9E0AF0DF5866B9" as author: GenId;
        "0DBB530B37B966D137C50B943700EDB2" as firstname: ShortString;
        "6BAA463FD4EAF45F6A103DB9433E4545" as lastname: ShortString;
        "A74AA63539354CDA47F387A4C3A8D54C" as title: ShortString;
        "FCCE870BECA333D059D5CD68C43B98F0" as page_count: R256;
        "6A03BAF6CFB822F04DA164ADAAEB53F6" as quote: Handle<Blake3, LongString>;
    }
}

fn main() {
    let owner = IdOwner::new();
    let mut kb = TribleSet::new();
    (0..1000000).for_each(|_| {
        let author = owner.defer_insert(fucid());
        let book = owner.defer_insert(fucid());
        kb += entity! { &author @
           literature::firstname: FirstName(EN).fake::<String>(),
           literature::lastname: LastName(EN).fake::<String>(),
        };
        kb += entity! { &book @
           literature::author: &author,
           literature::title: Words(1..3).fake::<Vec<String>>().join(" "),
           literature::quote: Sentence(5..25).fake::<String>().to_blob().get_handle()
        };
    });

    let author = owner.defer_insert(fucid());
    let book = owner.defer_insert(fucid());
    kb += entity! { &author @
       literature::firstname: "Frank",
       literature::lastname: "Herbert",
    };
    kb += entity! { &book @
       literature::author: &author,
       literature::title: "Dune",
       literature::quote: "I must not fear. Fear is the \
               mind-killer. Fear is the little-death that brings total \
               obliteration. I will face my fear. I will permit it to \
               pass over me and through me. And when it has gone past I \
               will turn the inner eye to see its path. Where the fear \
               has gone there will be nothing. Only I will remain.".to_blob().get_handle()
    };

    let fanks = find!(
        (author: Value<_>),
        pattern!(&kb, [
        {?author @ literature::firstname: "Frank"}]))
    .count();

    let herberts = find!(
        (author: Value<_>),
        pattern!(&kb, [
        {?author @ literature::lastname: "Herbert"}]))
    .count();

    println!("Found {} authors named Frank", fanks);
    println!("Found {} authors with the last name Herbert", herberts);

    (0..1000000).for_each(|_| {
        let _count = find!(
        (title: Value<_>, quote: Value<_>),
        pattern!(&kb, [
        {_?author @
            literature::firstname: "Frank",
            literature::lastname: "Herbert"},
        { literature::author: _?author,
          literature::title: ?title,
          literature::quote: ?quote
        }]))
        .count();
    });
}
