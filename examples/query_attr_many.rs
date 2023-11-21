use std::convert::TryInto;

use tribles::and;
use tribles::attribute::Attribute;
use tribles::query;

use tribles::patch;

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;
use tribles::types::syntactic::ShortString;
use tribles::types::syntactic::UFOID;

fn main() {
    patch::init();

    let mut name: Attribute<UFOID, ShortString> = Attribute::new();
    let mut loves: Attribute<UFOID, UFOID> = Attribute::new();

    (0..1000000).for_each(|_| {
        let lover_a = UFOID::new();
        let lover_b = UFOID::new();
        name.add(&lover_a, &(Name(EN).fake::<String>().try_into().unwrap()));
        name.add(&lover_b, &(Name(EN).fake::<String>().try_into().unwrap()));
        loves.add(&lover_a, &lover_b);
        loves.add(&lover_b, &lover_a);
    });

    (0..1000).for_each(|_| {
        let lover_a = UFOID::new();
        let lover_b = UFOID::new();
        name.add(&lover_a, &("Wameo".try_into().unwrap()));
        name.add(&lover_b, &(Name(EN).fake::<String>().try_into().unwrap()));
        loves.add(&lover_a, &lover_b);
        loves.add(&lover_b, &lover_a);
    });

    let romeo = UFOID::new();
    let juliet = UFOID::new();
    name.add(&romeo, &("Romeo".try_into().unwrap()));
    name.add(&juliet, &("Juliet".try_into().unwrap()));
    loves.add(&romeo, &juliet);
    loves.add(&juliet, &romeo);

    loop {
        for _r in query!(
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
