---
status: 🚧 in progress
---

# Slice 57 – Fuzz the file parsers

Since 2026-11-13; plan written and approved by the user on 2026-11-14 (with the weekly CI job and one hour per target for the first run). Steps 1–3 done (2026-11-14). Step 4: the first long run (one hour per target, one after another) started on the Mac on 2026-11-14 with nightly 2026-11-11 and cargo-fuzz 0.13.2; next: its findings and coverage, then step 5.

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

## Plan

Approach 3: `cargo-fuzz` for the deep runs on the Mac, and every input it finds becomes an ordinary test on stable, in CI on Windows and macOS. Five steps, one commit each (step 4 adds one `fix:` commit per finding).

### Step 1 – Entry points that take bytes (`refactor:`)

The parsers open files themselves today. Each gets a variant over a reader or bytes, with the path function as a thin wrapper; behaviour stays as it is.

| Today (path) | New (public, in `core`) |
|---|---|
| `raw::read_preview(path, format)` | `raw::read_preview_from(reader: impl Read + Seek, format)` |
| `raw::read_raw_metadata(path, format)` | `raw::read_raw_metadata_from(reader, format)` |
| `tiff::read_tiff_metadata(path)` | `tiff::read_tiff_metadata_from(reader)` (today's private `from_reader`) |
| `metadata::read_exif_metadata(path)` | `metadata::read_exif_metadata_from(reader)` (`nom_exif::MediaSource::seekable`, which is what `nom_exif::read_exif` does with a file) |
| `takeout::read_sidecar(path)` | `takeout::parse_sidecar(json: &[u8])` (exists, becomes public) |
| `decode::decode_tiff(path, limit)` | `decode::decode_tiff_from(reader, limit)` |

Details:

- `preview.rs`: `tiff_candidates`, `raf_candidates`, `pixels` and `read_at` become generic over `Read + Seek` instead of `BufReader<File>`. `read_at` takes the length found once by seeking to the end, instead of asking the file's metadata on every call.
- A JPEG under a RAW name is read whole from the reader with the same limit: `limited_read` gets `read_limited_from(reader, max)`, and `read_limited(path, max)` keeps its size check before reading and then calls it.
- `decode_image(ImageSource::Bytes { .. }, ..)` already takes bytes; it needs nothing.
- The new functions get unit tests on in-memory data (`Cursor`), the existing path tests stay. No test churn beyond that.

### Step 2 – Fuzz crate and harness (`test:`)

- **`fuzz/`**, generated by `cargo fuzz init` and trimmed: its own `[workspace]`, so the root workspace, CI, clippy, `cargo-about` and the release never see it. The root `Cargo.toml` gets `exclude = ["fuzz"]`. Dependencies: `libfuzzer-sys` and `sito-core` (path). `fuzz/Cargo.lock` is committed. Nothing in it ships, so it needs no notices entry. No `rust-toolchain` file: the nightly toolchain is picked per command (`cargo +nightly fuzz …`).
- **One harness, two users.** What each target does with an input is written once, in `crates/core/tests/support/fuzz_harness.rs` (in a subfolder, since Cargo would make a file directly in `tests/` a test crate of its own) (a plain module of `pub fn tiff_reader(data: &[u8])`, …). The regression test in step 3 uses it on stable, so CI compiles it on every push and it cannot rot; each fuzz target is a one-liner that includes it with `#[path = "../../crates/core/tests/support/fuzz_harness.rs"]`. That way `core` needs no fuzzing feature or extra public API.
- **Targets**, in the order of the concept:

  | Target | Calls | Checks besides "no panic" |
  |---|---|---|
  | `tiff_reader` | `TiffReader::open`, `walk`, then on every entry `numbers`, `text`, `value_location`, and `bytes` on that location; a maker-note style `nested` reader at each IFD offset | `bytes` returns exactly the requested length; `walk` returns at most 64 IFDs |
  | `raw_preview` | `read_preview_from` once as a TIFF-based format (DNG) and once as RAF, then `with_orientation(preview, 6)` | a preview starts with `FF D8`, is at most `MAX_BLOCK` (or `MAX_JPEG_BYTES` for a whole JPEG) |
  | `metadata` | `read_tiff_metadata_from`, `read_exif_metadata_from` (`nom-exif`), `read_raw_metadata_from` for RAF | a capture time is from 1990 on (`MIN_PLAUSIBLE_YEAR`) |
  | `orientation` | `with_orientation` with every orientation 1–8 | the result is the input unchanged, or the input with exactly one EXIF segment inserted after `FF D8` |
  | `sidecar` | `parse_sidecar` | – |
  | `decode` | `decode_image` with `max_edge` 256 and `None`; `decode_tiff_from` with a small limit (64 MB) | – |

- **Seeds:** the fixtures are already tiny (RAW samples under 2 KB, TIFFs under 8 KB, JPEGs small), so they are passed to the fuzzer as they are (`fixtures/raw`, `fixtures/tiff`, `fixtures/exif`, `fixtures/cameras`, the `.json` files of `fixtures/takeout`); nothing is copied. A dictionary `fuzz/dict/tiff.dict` (and `json.dict` for `sidecar`) (byte order marks, the tags Sito reads, `FUJIFILMCCD-RAW`, `OLYMPUS\0`, `Nikon\0`, JPEG markers) helps the fuzzer past the magic numbers. The corpus the fuzzer grows lives in `fuzz/corpus/` and is git-ignored, like `fuzz/artifacts/` and `fuzz/target/`.
- **Limits per input**, so runaway memory and hangs count as findings: `-timeout=10` (seconds), `-max_len=65536`, `-rss_limit_mb=2048`, and `-malloc_limit_mb=256` for the parser targets (from an input of 64 KB, any single allocation over 256 MB is a bug; the largest legitimate one is a 64 MB preview block, which needs a file that large). The `decode` target gets `-malloc_limit_mb=1100`, just above `TIFF_MEMORY_LIMIT`. A script `fuzz/run.sh <target> [seconds]` passes the right seeds, dictionary and limits, so nobody has to remember them.
- **`fuzz/README.md`:** installing (`rustup toolchain install nightly --profile minimal`, `cargo install cargo-fuzz`), running, minimizing a crash (`cargo +nightly fuzz tmin`), turning it into a regression file (step 3). Only on macOS or Linux; Windows (MSVC) is not supported by `cargo-fuzz`.

### Step 3 – Regression inputs on stable (`test:`)

- `crates/core/tests/fuzz_regressions/<target>/` holds the minimized inputs that once crashed, hung or allocated too much (empty at first, with a `README` line on what goes there).
- `crates/core/tests/fuzz_regressions.rs` runs every file there through the harness of its target, plus every fixture through every harness, so the harness itself is tested on real data on every push.
- A finding that is about time or memory, not a panic, also gets a regression test that checks the limit directly (e.g. that a crafted header is refused before the allocation), since the stable test would otherwise only show that the input does not crash.

### Step 4 – The first long run, and the fixes (`fix:` per finding)

- On the Mac: each target for one hour (`fuzz/run.sh <target> 3600`), one target at a time; then `cargo fuzz coverage` once per target to see which branches of the parsers were never reached, and add seeds or dictionary entries for those (e.g. an Olympus maker note with `CameraSettings`).
- **A finding in Sito's code:** minimized input into `fuzz_regressions/`, the fix, and a unit test in the parser module that names the case, in one `fix:` commit per finding.
- **A finding in a crate** (`image`, `jpeg-decoder`, `zune-jpeg`, `nom-exif`, `tiff`): I check whether a newer version fixes it; if not, I write an issue text with the minimized input for you to file upstream (filing it is your call). In Sito, the call is guarded where possible (a size or dimension check before the call, e.g. `jpeg_decoder::Decoder::set_max_decoding_buffer_size`, which is unlimited by default and not set in `decode_scaled` today). An abort or an unbounded loop inside a crate cannot be caught; such a case is listed under "Known gaps" in [Security](../domain/security.md) until the crate is fixed.
- The results go into this slice ("Findings of the first run": target, hours, executions, what was found, what was fixed).

### Step 5 – Running it again (`ci:` and `docs:`)

- **Scheduled job:** a new workflow `.github/workflows/fuzz.yml` on `ubuntu-latest`, weekly and by hand (`workflow_dispatch`), not on push: nightly toolchain, `cargo install cargo-fuzz --locked` (pinned version), each target for 10 minutes. It keeps the grown corpus between runs in the Actions cache (no secrets in this job, read-only token, nothing it produces ships). On a finding it fails and uploads the crashing input as an artifact. It also runs `cargo fmt --check` and `cargo clippy` on the fuzz crate, which the main CI does not see. Pinned actions, `permissions: contents: read`, passes `zizmor`. Free for a public repository; on the Mac nothing has to be remembered.
- **When a parser changes:** a short run of its target by hand (`fuzz/run.sh <target> 600`), noted in `CLAUDE.md` next to the existing rule for parser modules, together with: "a new parser module gets a fuzz target".
- `docs/domain/security.md`: "Parsers are fuzzed" under Photo files (what, how often), the known gap removed (or replaced by any crate finding that stays open). `CLAUDE.md` Commands: one line for `fuzz/run.sh`. Repository layout: `fuzz/`.

### Not in this slice

- Fuzzing the scan (folder walk) or the Tauri protocols: their input is not file content.
- Property tests with `proptest`/`bolero`: the regression tests of step 3 give the stable half of approach 3 without a new dependency in `core`.
- Fuzzing on the Windows machine.

### Decisions

Made by the user on 2026-11-14:

1. The weekly CI job of step 5 is set up (not only runs by hand).
2. The first run gives each target one hour (about six hours of the Mac, overnight).

## Builds on

- [Slice 56](56-security-hardening.md), step 6: the parsers already return errors instead of panicking on the cases clippy can see, so the fuzzer's first hours are not spent on those. Step 4 there sets the size limits the fuzz runs check against.

## Related

- [Photos](../domain/photos.md): the file formats Sito reads.
