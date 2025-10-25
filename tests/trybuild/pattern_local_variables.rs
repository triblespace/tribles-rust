use triblespace::prelude::*;

mod names {
    use triblespace::prelude::*;

    attributes! {
        "D02189E4C5A74E84B0FCBFDE3C533A0B" as first: valueschemas::ShortString;
        "8F2E5E6A6D9C42F2A4BF6471C5FBF5E0" as last: valueschemas::ShortString;
    }
}

fn main() {
    let base = TribleSet::new();
    let mut kb = base.clone();

    let same = ufoid();
    kb += entity! { &same @ names::first: "Same", names::last: "Same" };

    let delta = kb.difference(&base);

    let _: Vec<_> = find!(
        (person: Value<_>),
        pattern!(&kb, [
            { ?person @ names::first: _?name, names::last: _?name }
        ])
    )
    .collect();

    let _: Vec<_> = find!(
        (person: Value<_>),
        pattern_changes!(&kb, &delta, [
            { ?person @ names::first: _?name, names::last: _?name }
        ])
    )
    .collect();
}
