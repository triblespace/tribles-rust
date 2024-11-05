use tribles::prelude::valueschemas::*;
use tribles::prelude::*;

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;

fn main() {
    let mut name: Column<ShortString> = Column::new();
    let mut loves: Column<GenId> = Column::new();

    (0..1000000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();
        name.insert(
            &lover_a,
            Name(EN).fake::<String>(),
        );
        name.insert(
            &lover_b,
            Name(EN).fake::<String>(),
        );
        loves.insert(&lover_a, &lover_b);
        loves.insert(&lover_b, &lover_a);
    });

    (0..1000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();
        name.insert(&lover_a, "Wameo");
        name.insert(
            &lover_b,
            Name(EN).fake::<String>(),
        );
        loves.insert(&lover_a, &lover_b);
        loves.insert(&lover_b, &lover_a);
    });

    let romeo = ufoid();
    let juliet = ufoid();
    name.insert(&romeo, "Romeo");
    name.insert(&juliet, "Juliet");
    loves.insert(&romeo, &juliet);
    loves.insert(&juliet, &romeo);

    loop {
        for _r in find!(
            ctx,
            (juliet: Value<_>, romeo: Value<_>, romeo_name: Value<_>, juliet_name: Value<_>),
            and!(
                romeo_name.is("Wameo".try_to_value().unwrap()),
                name.has(romeo, romeo_name),
                name.has(juliet, juliet_name),
                loves.has(romeo, juliet)
            )
        ) {
            coz::progress!();
        }
    }
}
