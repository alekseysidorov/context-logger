#[test]
fn log_scope() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/pass/*.rs");
}
