//! Core pipeline of Sito.
//!
//! This crate contains the pure logic (scanning, grouping, analysis, scoring,
//! storage and actions) and must not depend on Tauri or any UI code.

pub mod scan;
