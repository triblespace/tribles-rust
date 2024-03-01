use std::convert::TryInto;

use tribles::transient::Transient;
use tribles::query::and;
use tribles::query::find;

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;
use tribles::types::ShortString;
use tribles::ufoid;
use tribles::Id;

fn main() {
    let mut name: Transient<ShortString> = Transient::new();
    let mut loves: Transient<Id> = Transient::new();

    (0..1000000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();
        name.add(&lover_a, &(Name(EN).fake::<String>().try_into().unwrap()));
        name.add(&lover_b, &(Name(EN).fake::<String>().try_into().unwrap()));
        loves.add(&lover_a, &lover_b);
        loves.add(&lover_b, &lover_a);
    });

    (0..1000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();
        name.add(&lover_a, &("Wameo".try_into().unwrap()));
        name.add(&lover_b, &(Name(EN).fake::<String>().try_into().unwrap()));
        loves.add(&lover_a, &lover_b);
        loves.add(&lover_b, &lover_a);
    });

    let romeo = ufoid();
    let juliet = ufoid();
    name.add(&romeo, &("Romeo".try_into().unwrap()));
    name.add(&juliet, &("Juliet".try_into().unwrap()));
    loves.add(&romeo, &juliet);
    loves.add(&juliet, &romeo);

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
