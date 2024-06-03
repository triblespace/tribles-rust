use std::convert::TryInto;

use tribles::query::and;
use tribles::query::find;
use tribles::column::Column;

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;
use tribles::types::SmallString;
use tribles::ufoid;
use tribles::Id;

fn main() {
    let mut name: Column<SmallString> = Column::new();
    let mut loves: Column<Id> = Column::new();

    (0..1000000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();
        name.insert(
            &lover_a,
            &(Name(EN).fake::<String>()[..].try_into().unwrap()),
        );
        name.insert(
            &lover_b,
            &(Name(EN).fake::<String>()[..].try_into().unwrap()),
        );
        loves.insert(&lover_a, &lover_b);
        loves.insert(&lover_b, &lover_a);
    });

    (0..1000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();
        name.insert(&lover_a, &("Wameo".try_into().unwrap()));
        name.insert(
            &lover_b,
            &(Name(EN).fake::<String>()[..].try_into().unwrap()),
        );
        loves.insert(&lover_a, &lover_b);
        loves.insert(&lover_b, &lover_a);
    });

    let romeo = ufoid();
    let juliet = ufoid();
    name.insert(&romeo, &("Romeo".try_into().unwrap()));
    name.insert(&juliet, &("Juliet".try_into().unwrap()));
    loves.insert(&romeo, &juliet);
    loves.insert(&juliet, &romeo);

    loop {
        for _r in find!(
            ctx,
            (juliet, romeo, romeo_name, juliet_name),
            and!(
                romeo_name.is("Wameo".try_into().unwrap()),
                name.has(romeo, romeo_name),
                name.has(juliet, juliet_name),
                loves.has(romeo, juliet)
            )
        ) {
            coz::progress!();
        }
    }
}
