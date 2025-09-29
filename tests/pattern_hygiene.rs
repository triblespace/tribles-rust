use trybuild::TestCases;

#[test]
fn pattern_macros_use_hygienic_idents() {
    let t = TestCases::new();
    t.pass("tests/trybuild/pattern_hygiene.rs");
    t.pass("tests/trybuild/pattern_local_vars.rs");
}
