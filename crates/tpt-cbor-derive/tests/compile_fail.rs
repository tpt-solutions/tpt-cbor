//! Compile-fail tests for the derive macros. Each file in `tests/ui` must fail
//! to compile, and its error must match the `.stderr` file beside it.

#[test]
fn ui() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
