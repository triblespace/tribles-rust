use tribles::prelude::*;
use tribles::value::schemas::genid::GenId;
use crate::pattern;
use crate::entity;
use crate::pattern_changes;
use crate::path;


NS! {
    namespace social {
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" as follows: GenId;
        "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB" as likes: GenId;
    }
}

fn main() {
    let mut kb = TribleSet::new();
    let a = fucid();
    let b = fucid();
    let c = fucid();
    kb += entity!(&a, { social::follows: &b });
    kb += entity!(&b, { social::likes: &c });

    for (s, e) in
        find!((s: Value<_>, e: Value<_>), path!(kb.clone(), s (social::follows | social::likes)+ e))
    {
        println!("{:?} -> {:?}", s, e);
    }
}
