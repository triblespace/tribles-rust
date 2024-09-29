use tribles::valueschemas::TryPackValue;
use tribles::{
    query::find,
    ufoid,
    valueschemas::{GenId, ShortString},
    TribleSet, NS,
};

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;

NS! {
    pub namespace knights {
        "39E2D06DBCD9CB96DE5BC46F362CFF31" as loves: GenId;
        "7D4F339CC4AE0BBA2765F34BE1D108EF" as name: ShortString;
        "3E0C58AC884072EA6429BB00A1BA1DA4" as title: ShortString;
    }
}

fn main() {
    let mut kb = TribleSet::new();
    (0..1000000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();

        kb.union(knights::entity!(lover_a,
        {
            name: Name(EN).fake::<String>()[..].try_pack().unwrap(),
            loves: lover_b.into()
        }));
        kb.union(knights::entity!(lover_b, {
            name: Name(EN).fake::<String>()[..].try_pack().unwrap(),
            loves: lover_a.into()
        }));
    });

    let mut data_kb = TribleSet::new();

    let romeo = ufoid();
    let juliet = ufoid();

    kb.union(knights::entity!(juliet, {
        name: "Juliet".try_pack().unwrap(),
        loves: romeo.into()
    }));
    kb.union(knights::entity!(romeo, {
        name: "Romeo".try_pack().unwrap(),
        loves: juliet.into()
    }));

    (0..999).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();

        data_kb.union(knights::entity!(lover_a, {
            name: "Romeo".try_pack().unwrap(),
            loves: lover_b.into()
        }));
        data_kb.union(knights::entity!(lover_b, {
            name: Name(EN).fake::<String>()[..].try_pack().unwrap(),
            loves: lover_a.into()
        }));
    });

    kb.union(data_kb);

    loop {
        for _r in find!(
            ctx,
            (juliet, name),
            knights::pattern!(ctx, &kb, [
            {name: ("Romeo".try_pack().unwrap()),
             loves: juliet},
            {juliet @
                name: name
            }])
        ) {
            coz::progress!();
        }
    }
}
