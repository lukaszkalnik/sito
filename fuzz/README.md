# Fuzzing the file parsers

Fuzz targets for the code of `sito-core` that reads photo files: the TIFF
structure reader, RAW previews and maker notes, metadata, orientation
patching, Takeout date files and decoding. A fuzzer feeds them millions of
mutated inputs and reports every panic, hang and runaway allocation.

This crate is a workspace of its own: the app's build, CI, clippy and license
notices never see it. It needs the nightly toolchain and runs on macOS and
Linux only (`cargo-fuzz` does not support Windows with MSVC).

## What each target does

The targets are one line each; what they do with an input is written once, in
[`crates/core/tests/support/fuzz_harness.rs`](../crates/core/tests/support/fuzz_harness.rs).
The stable test `crates/core/tests/fuzz_regressions.rs` runs the same harness
on every fixture and every saved finding, so it is compiled on every push.

| Target | Parsers |
|---|---|
| `tiff_reader` | `TiffReader`: IFD chains, values, nested maker-note structures |
| `raw_preview` | `read_preview_from` (TIFF-based and RAF), `with_orientation` |
| `metadata` | `read_tiff_metadata_from`, `read_exif_metadata_from` (`nom-exif`), `read_raw_metadata_from` |
| `orientation` | `with_orientation` with every orientation |
| `sidecar` | `parse_sidecar` |
| `decode` | `decode_image` (scaled and full), `decode_tiff_from` |

## Setup

```bash
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --locked
```

The nightly toolchain is picked per command (`cargo +nightly`); no
`rust-toolchain` file switches the repository to it.

## Running

```bash
fuzz/run.sh <target> [seconds]    # default 600 seconds
```

The script passes the fixtures as seeds, the dictionary in `dict/` and the
limits per input: 10 seconds, 64 KB, 2 GB of memory in all and 256 MB per
allocation (1100 MB for `decode`, just above the app's TIFF limit). The corpus
the fuzzer grows is kept in `corpus/<target>/` (git-ignored) and makes the next
run start where the last one stopped.

Which branches a corpus reaches (to find parser code no input gets to):

```bash
cd fuzz && cargo +nightly fuzz coverage <target> corpus/<target>
```

## A finding

The fuzzer stops and saves the input in `artifacts/<target>/` (`crash-…`,
`timeout-…`, `oom-…`).

1. Minimize it: `cd fuzz && cargo +nightly fuzz tmin <target> artifacts/<target>/<file>`.
2. Copy the minimized input to `crates/core/tests/fuzz_regressions/<target>/`
   with a name that says what it is (e.g. `ifd-loop.bin`). From then on the
   stable test runs it on every push.
3. Fix the parser, with a unit test in its module that names the case. A
   finding about time or memory also gets a test that checks the limit
   directly, since the regression test only shows that the input no longer
   crashes.
