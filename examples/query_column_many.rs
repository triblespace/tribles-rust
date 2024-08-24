use tribles::query::and;
use tribles::query::find;
use tribles::column::Column;

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;
use tribles::schemas::ShortString;
use tribles::schemas::TryPack;
use tribles::ufoid;
use tribles::schemas::GenId;

fn main() {
    let mut name: Column<ShortString> = Column::new();
    let mut loves: Column<GenId> = Column::new();

    (0..1000000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();
        name.insert(
            lover_a,
            Name(EN).fake::<String>()[..].try_pack().unwrap(),
        );
        name.insert(
            lover_b,
            Name(EN).fake::<String>()[..].try_pack().unwrap(),
        );
        loves.insert(lover_a, lover_b.into());
        loves.insert(lover_b, lover_a.into());
    });

    (0..1000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();
        name.insert(lover_a, "Wameo".try_pack().unwrap());
        name.insert(
            lover_b,
            Name(EN).fake::<String>()[..].try_pack().unwrap(),
        );
        loves.insert(lover_a, lover_b.into());
        loves.insert(lover_b, lover_a.into());
    });

    let romeo = ufoid();
    let juliet = ufoid();
    name.insert(romeo, "Romeo".try_pack().unwrap());
    name.insert(juliet, "Juliet".try_pack().unwrap());
    loves.insert(romeo, juliet.into());
    loves.insert(juliet, romeo.into());

    loop {
        for _r in find!(
            ctx,
            (juliet, romeo, romeo_name, juliet_name),
            and!(
                romeo_name.is("Wameo".try_pack().unwrap()),
                name.has(romeo, romeo_name),
                name.has(juliet, juliet_name),
                loves.has(romeo, juliet)
            )
        ) {
            coz::progress!();
        }
    }
}
