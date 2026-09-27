#[test]
fn log_scope() {
    let tests = trybuild::TestCases::new();
    // Compile-pass coverage for supported syntax and generated function shapes.
    tests.pass("tests/ui/pass/*.rs");
    // Diagnostic coverage for rejected syntax and unsupported expansion cases.
    tests.compile_fail("tests/ui/fail/*.rs");
}
