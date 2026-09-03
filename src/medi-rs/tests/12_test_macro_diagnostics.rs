#[test]
fn invalid_module_compositions_fail_to_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/duplicate_command.rs");
    cases.compile_fail("tests/ui/duplicate_resource.rs");
    cases.compile_fail("tests/ui/missing_resource.rs");
}
