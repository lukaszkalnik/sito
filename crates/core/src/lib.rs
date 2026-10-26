//! Core pipeline of Sito.
//!
//! This crate contains the pure logic (scanning, grouping, analysis, scoring,
//! storage and actions) and must not depend on Tauri or any UI code.

/// Version of the core library, taken from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver() {
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "unexpected version: {VERSION}");
        assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()));
    }
}
