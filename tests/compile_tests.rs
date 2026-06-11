#[test]
fn compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile/surjective_*.rs");
    t.compile_fail("tests/compile/non_surjective_*.rs");
}
