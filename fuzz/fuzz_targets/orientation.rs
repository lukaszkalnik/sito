#![no_main]

#[path = "../../crates/core/tests/support/fuzz_harness.rs"]
#[allow(dead_code, reason = "each target uses one function of the harness")]
mod harness;

libfuzzer_sys::fuzz_target!(|data: &[u8]| harness::orientation(data));
