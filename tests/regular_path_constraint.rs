use tribles::prelude::*;
use tribles::value::schemas::genid::GenId;

NS! {
    namespace social {
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" as follows: GenId;
        "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB" as likes: GenId;
    }
}

#[test]
fn simple_path() {
    let mut kb = TribleSet::new();
    let a = fucid();
    let b = fucid();
    kb += crate::entity!(&a, { social::follows: &b });
    let a_val = a.to_value();
    let b_val = b.to_value();
    let results: Vec<_> =
        find!((s: Value<_>, e: Value<_>), social::path!(kb.clone(), s follows e)).collect();
    assert!(results.contains(&(a_val, b_val)));
}

#[test]
fn alternation() {
    let mut kb = TribleSet::new();
    let a = fucid();
    let b = fucid();
    let c = fucid();
    kb += crate::entity!(&a, { social::follows: &b });
    kb += crate::entity!(&a, { social::likes: &c });
    let a_val = a.to_value();
    let b_val = b.to_value();
    let c_val = c.to_value();

    let results: Vec<_> =
        find!((s: Value<_>, e: Value<_>), social::path!(kb.clone(), s (follows | likes) e))
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
    kb += crate::entity!(&a, { social::follows: &b });
    kb += crate::entity!(&b, { social::follows: &c });

    let start_val = a.to_value();
    let end_val = c.to_value();
    let results: Vec<_> = find!((s: Value<_>, e: Value<_>),
        and!(s.is(start_val), e.is(end_val), social::path!(kb.clone(), s follows+ e)))
    .collect();
    assert!(results.contains(&(start_val, end_val)));
}
