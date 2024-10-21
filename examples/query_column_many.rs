use tribles::prelude::valueschemas::*;
use tribles::prelude::*;

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;

fn main() {
    let mut name: Column<ShortString> = Column::new();
    let mut loves: Column<GenId> = Column::new();

    (0..1000000).for_each(|_| {
        let lover_a = ufoid().raw;
        let lover_b = ufoid().raw;
        name.insert(lover_a, Name(EN).fake::<String>()[..].try_to_value().unwrap());
        name.insert(lover_b, Name(EN).fake::<String>()[..].try_to_value().unwrap());
        loves.insert(lover_a, lover_b.to_value());
        loves.insert(lover_b, lover_a.to_value());
    });

    (0..1000).for_each(|_| {
        let lover_a = ufoid().raw;
        let lover_b = ufoid().raw;
        name.insert(lover_a, "Wameo".try_to_value().unwrap());
        name.insert(lover_b, Name(EN).fake::<String>()[..].try_to_value().unwrap());
        loves.insert(lover_a, lover_b.to_value());
        loves.insert(lover_b, lover_a.to_value());
    });

    let romeo = ufoid().raw;
    let juliet = ufoid().raw;
    name.insert(romeo, "Romeo".try_to_value().unwrap());
    name.insert(juliet, "Juliet".try_to_value().unwrap());
    loves.insert(romeo, juliet.to_value());
    loves.insert(juliet, romeo.to_value());

    loop {
        for _r in find!(
            ctx,
            (juliet, romeo, romeo_name, juliet_name),
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
