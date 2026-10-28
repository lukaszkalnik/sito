//! Core pipeline of Sito.
//!
//! This crate contains the pure logic (scanning, grouping, analysis, scoring,
//! storage and actions) and must not depend on Tauri or any UI code.

#![warn(missing_docs)]

pub mod actions;
pub mod metadata;
pub mod photo_cache;
pub mod reject;
pub mod scan;
pub mod series;
pub mod takeout;
pub mod thumbnail;
pub mod timeline;
