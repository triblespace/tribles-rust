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

#[test]
fn simple_path() {
    let mut kb = TribleSet::new();
    let a = fucid();
    let b = fucid();
    kb += entity!(&a, { social::follows: &b });
    let a_val = a.to_value();
    let b_val = b.to_value();
    let results: Vec<_> =
        find!((s: Value<_>, e: Value<_>), path!(kb.clone(), s social::follows e)).collect();
    assert!(results.contains(&(a_val, b_val)));
}

#[test]
fn alternation() {
    let mut kb = TribleSet::new();
    let a = fucid();
    let b = fucid();
    let c = fucid();
    kb += entity!(&a, { social::follows: &b });
    kb += entity!(&a, { social::likes: &c });
    let a_val = a.to_value();
    let b_val = b.to_value();
    let c_val = c.to_value();

    let results: Vec<_> =
        find!((s: Value<_>, e: Value<_>), path!(kb.clone(), s (social::follows | social::likes) e))
            .collect();
    assert!(results.contains(&(a_val, b_val)));
    assert!(results.contains(&(a_val, c_val)));
}

#[test]
fn repetition() {
    let mut kb = TribleSet::new();
    let a = fucid();
    let b = fucid();
    let c = fucid();
    kb += entity!(&a, { social::follows: &b });
    kb += entity!(&b, { social::follows: &c });

    let start_val = a.to_value();
    let end_val = c.to_value();
    let results: Vec<_> = find!((s: Value<_>, e: Value<_>),
        and!(s.is(start_val), e.is(end_val), path!(kb.clone(), s social::follows+ e)))
    .collect();
    assert!(results.contains(&(start_val, end_val)));
}
