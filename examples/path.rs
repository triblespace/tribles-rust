use tribles::prelude::*;
use tribles::value::schemas::genid::GenId;
use crate::pattern;
use crate::entity;
use crate::pattern_changes;
use crate::path;


pub mod social {
    #![allow(unused)]
    use crate::prelude::*;
    pub const follows: crate::field::Field<GenId> = crate::field::Field::from(hex_literal::hex!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"));
    pub const likes: crate::field::Field<GenId> = crate::field::Field::from(hex_literal::hex!("BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"));
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
