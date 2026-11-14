# Fuzz regressions

Minimized inputs that once made a fuzz target (`fuzz/`) crash, hang or
allocate too much, one folder per target (`tiff_reader/`, `raw_preview/`,
`metadata/`, `orientation/`, `sidecar/`, `decode/`). `fuzz_regressions.rs` runs
each file through its target's harness on every `cargo test`. How a finding
gets here: [fuzz/README.md](../../../../fuzz/README.md), "A finding".
