use tribles::prelude::*;
use tribles::value::schemas::genid::GenId;

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
    kb += social::entity!(&a, { follows: &b });
    kb += social::entity!(&b, { likes: &c });

    for (s, e) in
        find!((s: Value<_>, e: Value<_>), social::path!(kb.clone(), s (follows | likes)+ e))
    {
        println!("{:?} -> {:?}", s, e);
    }
}
