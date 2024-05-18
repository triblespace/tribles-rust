use std::convert::TryInto;

use tribles::{query::find, types::SmallString, ufoid, Id, TribleSet, NS};

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;

NS! {
    pub namespace knights {
        "39E2D06DBCD9CB96DE5BC46F362CFF31" as loves: Id;
        "7D4F339CC4AE0BBA2765F34BE1D108EF" as name: SmallString;
        "3E0C58AC884072EA6429BB00A1BA1DA4" as title: SmallString;
    }
}

fn main() {
    let mut kb = TribleSet::new();
    (0..1000000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();

        kb.union(&knights::entity!({
            name: Name(EN).fake::<String>()[..].try_into().unwrap(),
            loves: lover_b
        }));
        kb.union(&knights::entity!({
            name: Name(EN).fake::<String>()[..].try_into().unwrap(),
            loves: lover_a
        }));
    });

    let mut data_kb = TribleSet::new();

    let romeo = ufoid();
    let juliet = ufoid();

    data_kb.union(&knights::entity!(juliet, {
        name: "Juliet".try_into().unwrap(),
        loves: romeo
    }));
    data_kb.union(&knights::entity!(romeo, {
        name: "Romeo".try_into().unwrap(),
        loves: juliet
    }));

    kb.union(&data_kb);

    loop {
        for _r in find!(
            ctx,
            (juliet, name),
            knights::pattern!(ctx, kb, [
            {name: ("Romeo".try_into().unwrap()),
             loves: juliet},
            {juliet @
                name: name
            }])
        ) {
            coz::progress!();
        }
    }
}
