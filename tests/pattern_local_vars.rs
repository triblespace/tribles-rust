use crate::{entity, pattern, pattern_changes};
use tribles::prelude::*;
use trybuild::TestCases;

pub mod names {
    use tribles::prelude::*;

    attributes! {
        "D02189E4C5A74E84B0FCBFDE3C533A0B" as first: valueschemas::ShortString;
        "8F2E5E6A6D9C42F2A4BF6471C5FBF5E0" as last: valueschemas::ShortString;
    }
}

#[test]
fn pattern_local_variables_compile() {
    let t = TestCases::new();
    t.pass("tests/trybuild/pattern_local_variables.rs");
}

#[test]
fn pattern_local_variables_enforce_equality() {
    let mut kb = TribleSet::new();

    let same = ufoid();
    kb += entity! { &same @ names::first: "Same", names::last: "Same" };

    let different = ufoid();
    kb += entity! { &different @ names::first: "Alice", names::last: "Smith" };

    let results: Vec<_> = find!(
        (person: Value<_>),
        pattern!(&kb, [
            { ?person @ names::first: _?name, names::last: _?name }
        ])
    )
    .collect();

    assert_eq!(results, vec![(same.to_value(),)]);
}

#[test]
fn pattern_changes_local_variables_track_deltas() {
    let base = TribleSet::new();
    let mut updated = base.clone();

    let same = ufoid();
    updated += entity! { &same @ names::first: "Same", names::last: "Same" };

    let delta = updated.difference(&base);

    let results: Vec<_> = find!(
        (person: Value<_>),
        pattern_changes!(&updated, &delta, [
            { ?person @ names::first: _?name, names::last: _?name }
        ])
    )
    .collect();

    assert_eq!(results, vec![(same.to_value(),)]);
}
