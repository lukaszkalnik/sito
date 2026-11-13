---
status: 📝 concept drafted
---

# Slice 57 – Fuzz the file parsers

Since 2026-11-13; plan when the slice starts.

## Idea

Asked by the user on 2026-11-13, after the security review of [slice 56](56-security-hardening.md): the parsers that read photo files are tested with a fuzzer, a tool that feeds them millions of random and mutated inputs and reports every panic, hang and runaway allocation.

Photos come from anywhere, and the code that reads them is where a crafted file could harm Sito. Part of that code is Sito's own (the TIFF structure reader, the RAW preview finder, the maker notes); the rest is in crates (`image`, `jpeg-decoder`, `nom-exif`). The review read the hand-written parsers and found them careful, and slice 56 adds lints against panics. A fuzzer finds what reading and lints miss: inputs nobody thought of.

There is nothing to see in the app. The result is a fuzz setup in the repository, the bugs it found fixed with regression tests, and a way to run it again.

## What to fuzz

In rough order of value:

1. **`TiffReader`** (`crates/core/src/tiff/reader.rs`): `open`, `walk`, `numbers`, `text`. Offsets, counts and chains of IFDs from the file.
2. **RAW preview** (`crates/core/src/raw/preview.rs`, `maker_note.rs`): finding the embedded JPEG in TIFF-based RAW files and in Fujifilm RAF.
3. **TIFF metadata** (`crates/core/src/tiff/metadata.rs`) and the EXIF reading in `metadata.rs`, which goes through `nom-exif`.
4. **Orientation patching** (`crates/core/src/raw/orientation.rs`): writes into a JPEG's bytes.
5. **Takeout date files** (`crates/core/src/takeout/sidecar.rs`): JSON through `serde_json`, low risk.
6. **Decoding to pixels** (`crates/core/src/decode.rs`): mostly third-party decoders, which their own projects fuzz. Worth a target for the glue (scaled decode, limits), with a small time budget.

## Candidate approaches

1. **`cargo-fuzz` (libFuzzer).** The usual tool for Rust. It needs the nightly toolchain and runs on Linux and macOS, not on Windows with MSVC. Fuzz targets live in a `fuzz/` crate outside the workspace, so the normal build, clippy and CI do not see nightly code.
2. **Property tests on stable (`proptest` or `bolero`).** Random inputs inside the normal `cargo test`, on every platform and in CI. Far fewer inputs per second and no coverage guidance, so it finds less, but it never needs nightly.
3. **Both:** `cargo-fuzz` for the deep runs, and the inputs it finds become ordinary regression tests on stable.

Likely choice: 3, with `cargo-fuzz` run by hand on the Mac (the Windows machine has 7 GB of RAM and no libFuzzer support).

## To decide when planning

- **Parsers take paths today** (`read_preview(path, ..)`, `read_tiff_metadata(path)`); a fuzzer wants bytes. `TiffReader` is already generic over `Read + Seek`; the others need a variant that takes a reader, with the path version as a thin wrapper. That refactoring is the first step and is useful on its own (tests without temp files).
- **Seed files:** the fixtures in `fixtures/` (JPEG, RAW samples, TIFF) cut down to their headers, so the fuzzer starts from valid structures.
- **Limits per run:** a memory limit and a timeout per input, so "allocates 2 GB" and "loops forever" count as findings, matching the limits of slice 56.
- **How long and how often:** a first long run per target (hours), then a short run when a parser changes. Whether a scheduled CI job on Linux is worth it, or runs by hand are enough.
- **Bugs in third-party crates:** report upstream; in Sito, guard the call (size limit, `catch_unwind` is not an option for aborts) or wait for the fix.
- **Nightly toolchain:** installed only on the Mac, used only for `cargo fuzz`; `rust-toolchain` files must not switch the workspace to it.

## Builds on

- [Slice 56](56-security-hardening.md), step 6: the parsers already return errors instead of panicking on the cases clippy can see, so the fuzzer's first hours are not spent on those. Step 4 there sets the size limits the fuzz runs check against.

## Related

- [Photos](../domain/photos.md): the file formats Sito reads.
