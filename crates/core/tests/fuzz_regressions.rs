//! Runs the fuzz harness on stable: every saved fuzz finding through its
//! target, and every fixture through every target, so the harness is tested
//! on real data on every push.

#![allow(clippy::unwrap_used)]

#[path = "support/fuzz_harness.rs"]
mod fuzz_harness;

use std::fs;
use std::path::{Path, PathBuf};

/// What a fuzz target does with one input.
type Harness = fn(&[u8]);

/// Every fuzz target: its name (the folder of its findings) and harness.
const TARGETS: [(&str, Harness); 6] = [
    ("tiff_reader", fuzz_harness::tiff_reader),
    ("raw_preview", fuzz_harness::raw_preview),
    ("metadata", fuzz_harness::metadata),
    ("orientation", fuzz_harness::orientation),
    ("sidecar", fuzz_harness::sidecar),
    ("decode", fuzz_harness::decode),
];

/// Every file under `dir`, in sorted order; none if `dir` does not exist.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .map(|entry| entry.unwrap().path())
        .flat_map(|path| {
            if path.is_dir() {
                files_under(&path)
            } else {
                vec![path]
            }
        })
        .collect();
    files.sort();
    files
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn saved_fuzz_findings_no_longer_fail() {
    let root = manifest_dir().join("tests/fuzz_regressions");
    for (target, harness) in TARGETS {
        for file in files_under(&root.join(target)) {
            println!("{target}: {}", file.display());
            harness(&fs::read(&file).unwrap());
        }
    }
}

#[test]
fn every_fixture_passes_every_harness() {
    let fixtures = files_under(&manifest_dir().join("../../fixtures"));
    assert!(fixtures.len() > 40, "fixtures not found");
    for file in &fixtures {
        let data = fs::read(file).unwrap();
        for (target, harness) in TARGETS {
            println!("{target}: {}", file.display());
            harness(&data);
        }
    }
}

#[test]
fn the_harness_checks_its_invariants_on_edge_cases() {
    let edge_cases: [&[u8]; 5] = [b"", b"\xFF", b"\xFF\xD8", b"II*\0", b"MM\0*\0\0\0\x08"];
    for data in edge_cases {
        for (_, harness) in TARGETS {
            harness(data);
        }
    }
}
