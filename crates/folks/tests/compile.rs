#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
    // The error for using an integration whose feature is off.
    #[cfg(not(feature = "schemars"))]
    t.compile_fail("tests/ui_features/*.rs");
    #[cfg(not(feature = "email"))]
    t.compile_fail("tests/ui_email/*.rs");
}
