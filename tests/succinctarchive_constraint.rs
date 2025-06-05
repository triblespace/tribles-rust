use std::collections::HashSet;
use sucds::bit_vectors::Rank9Sel;
use tribles::blob::schemas::succinctarchive::{OrderedUniverse, SuccinctArchive};
use tribles::prelude::*;
use tribles::query::{Binding, Constraint, TriblePattern, VariableContext};
use tribles::value::schemas::{genid::GenId, UnknownValue};

#[test]
fn propose_and_confirm() {
    let e1 = Id::new([1u8; 16]).unwrap();
    let e2 = Id::new([2u8; 16]).unwrap();
    let a1 = Id::new([10u8; 16]).unwrap();
    let a2 = Id::new([20u8; 16]).unwrap();
    let v1 = Value::<UnknownValue>::new([1u8; 32]);
    let v2 = Value::<UnknownValue>::new([2u8; 32]);
    let v3 = Value::<UnknownValue>::new([3u8; 32]);
    let v4 = Value::<UnknownValue>::new([4u8; 32]);
    let v5 = Value::<UnknownValue>::new([5u8; 32]);
    let v6 = Value::<UnknownValue>::new([6u8; 32]);

    let mut set = TribleSet::new();
    set.insert(&Trible::force(&e1, &a1, &v1));
    set.insert(&Trible::force(&e1, &a1, &v2));
    set.insert(&Trible::force(&e1, &a2, &v3));
    set.insert(&Trible::force(&e2, &a1, &v4));
    set.insert(&Trible::force(&e2, &a1, &v5));
    set.insert(&Trible::force(&e2, &a2, &v6));

    let archive: SuccinctArchive<OrderedUniverse, Rank9Sel> = (&set).into();

    let mut ctx = VariableContext::new();
    let e_var = ctx.next_variable::<GenId>();
    let a_var = ctx.next_variable::<GenId>();
    let v_var = ctx.next_variable::<UnknownValue>();
    let constraint = archive.pattern(e_var, a_var, v_var);

    let mut binding = Binding::default();
    binding.set(e_var.index, &e1.to_value().raw);

    let mut proposals = Vec::new();
    constraint.propose(a_var.index, &binding, &mut proposals);
    let attrs: HashSet<_> = proposals.iter().cloned().collect();
    assert_eq!(
        attrs,
        [a1.to_value().raw, a2.to_value().raw].into_iter().collect()
    );

    proposals.push(e1.to_value().raw);
    constraint.confirm(a_var.index, &binding, &mut proposals);
    assert_eq!(proposals.len(), 2);
}
