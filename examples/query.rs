use std::convert::TryInto;

use tribles::namespace::knights;
use tribles::query;

use tribles::patch;
use tribles::tribleset::patchtribleset::PATCHTribleSet;

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;

fn main() {
    patch::init();

    let mut kb = PATCHTribleSet::new();
    (0..1000000).for_each(|_| {
        kb.union(&knights::entities!((lover_a, lover_b),
        [{lover_a @
            name: Name(EN).fake::<String>().try_into().unwrap(),
            loves: lover_b
        },
        {lover_b @
            name: Name(EN).fake::<String>().try_into().unwrap(),
            loves: lover_a
        }]));
    });

    let mut data_kb = knights::entities!((romeo, juliet),
    [{juliet @
        name: "Juliet".try_into().unwrap(),
        loves: romeo
    },
    {romeo @
        name: "Romeo".try_into().unwrap(),
        loves: juliet
    }]);

    (0..999).for_each(|_| {
        data_kb.union(&knights::entities!((lover_a, lover_b),
        [{lover_a @
            name: "Romeo".try_into().unwrap(),
            loves: lover_b
        },
        {lover_b @
            name: Name(EN).fake::<String>().try_into().unwrap(),
            loves: lover_a
        }]));
    });

    kb.union(&data_kb);

    loop {
        for _r in query!(
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
