//! Compile-fail tests for bad `#[observe]` usage (trybuild).
#![cfg(all(feature = "macros", not(target_family = "wasm")))]

#[test]
fn observe_attribute_errors() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
