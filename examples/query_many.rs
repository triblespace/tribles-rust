use std::convert::TryInto;

use tribles::query::find;
use tribles::NS;

use tribles::patch;
use tribles::TribleSet;

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;

NS! {
    pub namespace knights {
        @ tribles::types::syntactic::UFOID;
        loves: "328edd7583de04e2bedd6bd4fd50e651" as tribles::types::syntactic::UFOID;
        name: "328147856cc1984f0806dbb824d2b4cb" as tribles::types::syntactic::ShortString;
        title: "328f2c33d2fdd675e733388770b2d6c4" as tribles::types::syntactic::ShortString;
    }
}

fn main() {
    patch::init();

    let mut kb = TribleSet::new();
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
