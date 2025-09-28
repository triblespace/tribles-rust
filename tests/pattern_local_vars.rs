use tribles::prelude::*;

mod social {
    use tribles::prelude::*;

    attributes! {
        "C2C8D4D6E3E5479EA6F4D71D979CD3CE" as friend: valueschemas::GenId;
        "E2175D85AC9F4A09BB52A0F7971D7569" as best_friend: valueschemas::GenId;
    }
}

mod library {
    use tribles::prelude::*;

    attributes! {
        "6E7843FC4D9C428EBF5C9C86CB8C33C4" as title: valueschemas::ShortString;
        "3E51B9E2E4C14D2DA0DC6B0ACB5CBF56" as subtitle: valueschemas::ShortString;
    }
}

#[test]
fn pattern_local_vars_require_no_external_binding() {
    let mut set = TribleSet::new();
    let alice = ufoid();
    let bob = ufoid();
    let carol = ufoid();

    set += entity! {
        &alice @
        social::friend: &bob,
        social::best_friend: &bob
    };

    set += entity! {
        &carol @
        social::friend: &bob,
        social::best_friend: &alice
    };

    let results: Vec<_> = find!((person: Value<_>), pattern!(&set, [
        { ?person @ social::friend: _?buddy },
        { ?person @ social::best_friend: _?buddy }
    ]))
    .collect();

    assert_eq!(results, vec![(alice.to_value(),)]);
}

#[test]
fn pattern_changes_local_vars_are_scoped_to_invocation() {
    let base = TribleSet::new();
    let mut updated = base.clone();
    let alice = ufoid();
    let bob = ufoid();
    let delta_friend = ufoid();

    updated += entity! {
        &alice @
        social::friend: &bob,
        social::best_friend: &bob
    };

    updated += entity! {
        &delta_friend @
        social::friend: &alice,
        social::best_friend: &bob
    };

    let delta = updated.difference(&base);

    let results: Vec<_> = find!((person: Value<_>), pattern_changes!(&updated, &delta, [
        { ?person @ social::friend: _?buddy },
        { ?person @ social::best_friend: _?buddy }
    ]))
    .collect();

    assert_eq!(results, vec![(alice.to_value(),)]);
}

#[test]
fn pattern_local_vars_infer_value_schema_from_usage() {
    let mut set = TribleSet::new();
    let highlighted = ufoid();
    let ignored = ufoid();

    set += entity! {
        &highlighted @
        library::title: "Rust Patterns",
        library::subtitle: "Rust Patterns"
    };

    set += entity! {
        &ignored @
        library::title: "Query Guide",
        library::subtitle: "Second Edition"
    };

    let results: Vec<_> = find!((book: Value<_>), pattern!(&set, [
        { ?book @ library::title: _?label, library::subtitle: _?label }
    ]))
    .collect();

    assert_eq!(results, vec![(highlighted.to_value(),)]);
}
