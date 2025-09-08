use crate::entity;
use crate::path;
use crate::pattern;
use crate::pattern_changes;
use tribles::prelude::*;
use tribles::value::schemas::genid::GenId;

pub mod social {
    #![allow(unused)]
    use super::*;
    use tribles::prelude::*;
    crate::fields! {
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" as follows: valueschemas::GenId;
        "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB" as likes: valueschemas::GenId;
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
