# Sito – Project Context

> "Sito" is Polish for *sieve*: the app sifts through photo series and keeps the best shot.

Desktop app that finds **series of similar shots** in a large photo gallery (photos taken with Android phones) and suggests the best photo of each series.

Typical cases: photos taken seconds apart that differ in exposure, slightly in angle, or in facial expression. This is **not** primarily about byte-identical duplicates. There are **no** burst or motion photos.

Platforms: developed on Windows, macOS later.

## Language

- **UI language: Polish.** All user-facing strings are in Polish.
- **Everything else is English:** code, identifiers, comments, commit messages, PR descriptions, docs and this file.
- Never hardcode UI strings in components. Keep them in a translation file (e.g. `ui/src/i18n/pl.json`) via an i18n library such as i18next, so other languages can be added later.
- **Polish plurals have three forms** (1 zdjęcie, 2 zdjęcia, 5 zdjęć). Always use proper plural rules (ICU / `Intl.PluralRules`), never `count === 1 ? a : b`.
- Format dates and numbers with the `pl-PL` locale.
- Use correct Polish diacritics (ą, ć, ę, ł, ń, ó, ś, ź, ż) in UI text and make sure the chosen font renders them.

## Hard rules

- **Never delete files.** Rejected photos are only moved to a quarantine folder or the system trash. Every action is logged in SQLite so it can be undone.
- Everything runs **locally and offline**. No photos, embeddings or face data leave the machine.
- The app **only suggests**. The user always makes the final decision.
- `crates/core` has **no dependency on Tauri or UI code**.
- Small, focused changes. New logic in `core` comes with tests.

## Development workflow

- **GitHub repo:** https://github.com/lukaszkalnik/sito (branch `main`).
- **Unit tests:** every new piece of logic gets unit tests (Rust: `#[cfg(test)]` modules / `tests/`; frontend: Vitest + Testing Library). Bug fixes come with a regression test.
- **Linters and static checks** must pass before every commit:
  - Rust: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (clippy pedantic is enabled workspace-wide)
  - Frontend: oxlint (`--deny-warnings`), Prettier, `tsc -b`
- **Commit and push every self-contained change.** One logical change per commit, Conventional Commits style (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`, `ci:`), with tests and checks green. Push right after committing.
- **Code style and architecture:** follow the official/recommended style of each language and framework: Rust API Guidelines and idiomatic error handling (`thiserror` in libraries, no `unwrap` outside tests); React function components and hooks, strict TypeScript, no `any`; PEP 8 with type hints. Keep the layering: `core` (pure logic) ← `src-tauri` (thin adapter) ← `ui` (presentation only).
- **End-to-end tests** where possible: integration tests in `crates/core/tests/` run the full pipeline on `fixtures/`; UI flows are tested with Playwright against the Vite dev server with mocked Tauri commands. Run them before pushing changes that touch the affected flow.
- **Local tool locations (Windows dev machine):** all toolchains live under `D:\Lukasz\tools` (C: is nearly full): Rust (`RUSTUP_HOME`, `CARGO_HOME`), Node, GitHub CLI, uv, npm cache/global prefix (`npm_config_cache`, `npm_config_prefix`), Playwright browsers (`PLAYWRIGHT_BROWSERS_PATH`), VS Build Tools. Never install large tools or caches on C:.
- **Low memory on the dev machine (7 GB RAM):** never run several heavy jobs at once (installers, Rust builds, browser tests). The user env var `CARGO_BUILD_JOBS=2` limits parallel rustc jobs locally (CI uses all cores).

## Tech stack

- **Tauri** (Rust backend + web frontend)
- **Frontend:** React + TypeScript (Vite)
- **Rust crates:** `walkdir` (scanning), `nom-exif` (EXIF), `image` (decoding), `rayon` (parallelism), `ort` (ONNX Runtime; DirectML on Windows, CoreML on macOS, CPU fallback), `rusqlite` (cache)
- **Parameter tuning** happens in the app itself (no separate prototype): thresholds and score weights are settings, and a debug panel shows each photo's individual scores.

## Repository layout

```
sito/
├─ CLAUDE.md
├─ Cargo.toml         # Rust workspace (crates/core, src-tauri)
├─ package.json       # npm workspace (ui) + Tauri CLI
├─ crates/core/       # pure logic, no UI code
├─ src-tauri/         # thin layer: commands and progress events
├─ ui/                # React + TypeScript
│  ├─ src/i18n/pl.json
│  └─ e2e/            # Playwright tests
├─ docs/ROADMAP.md    # work plan (vertical slices)
├─ docs/slices/       # one plan per slice (NN-name.md)
├─ docs/RELEASING.md  # how to build, release and install
├─ scripts/           # dev helper scripts (icon rendering)
├─ fixtures/          # small test photos with known groups
└─ .github/workflows/
```

## Architecture (pipeline in `crates/core`)

1. **Scanner:** walk folders recursively, read EXIF (capture time, orientation, camera). Fallback without EXIF: file modification time.
2. **Grouping:**
   - Sort photos by capture time. A new group starts when the gap exceeds a threshold (default ~5–10 s, adjustable via a slider in the UI).
   - Then split groups by embedding similarity (cosine) when they contain different subjects.
3. **Analysis** (parallel; decode downscaled early; respect EXIF orientation):
   - Image embedding via a small ONNX model (e.g. DINOv2-small or CLIP)
   - Sharpness: variance of the Laplacian
   - Exposure: share of clipped highlights and crushed shadows in the histogram
   - Faces and eye state (open/closed) via a small model (e.g. YuNet + landmarks). Check the model license before adding it.
4. **Scoring:** weighted score from sharpness, exposure and faces; suggest the best photo per group.
5. **Storage:** SQLite keyed by path + size + mtime. Stores embeddings, scores, groups and decisions so rescans are fast.
6. **Actions:** move to quarantine, with an undo log.

The Tauri layer exposes commands (e.g. `start_scan`, `get_groups`, `set_decision`, `apply_decisions`, `undo`) and emits progress events. Thumbnails are served by Rust from a thumbnail cache.

**ONNX models** are not committed to the repo. They are downloaded on first launch and verified by checksum.

## UI

- Folder picker, progress bar
- Group list
- Side-by-side comparison view. Shortcuts: arrow keys = navigate, Space = keep, X = reject.
- Sliders for time gap and similarity threshold
- The suggested best photo is highlighted

## CI (GitHub Actions)

- **Pull requests:** `cargo fmt --check`, `cargo clippy`, `cargo test`, plus frontend lint and build, on both `windows-latest` and `macos-latest`
- **Release** (`v*` tag or manual run): `tauri-action` builds a per-user NSIS installer (Windows) and a DMG (macOS); the Windows installer is smoke-tested (install, launch, uninstall) before the release is published. Unsigned for now; macOS signing and notarization come later. See [docs/RELEASING.md](docs/RELEASING.md).

## Roadmap

The work plan lives in [docs/ROADMAP.md](docs/ROADMAP.md): small vertical slices, each testable in the running app. Every concept is committed to the repo, also drafts and work in progress (`docs/slices/NN-name.md` with status 📝 draft, ideas in `docs/ideas/`); the roadmap keeps only a one-line summary with the link, never the details. Plan each slice in plan mode first, save the approved plan into its `docs/slices/NN-name.md`, then implement and update its status in both files.

## Commands

Run from the repo root unless noted.

```bash
npm install                          # install frontend deps and Tauri CLI
npm run tauri dev                    # run the desktop app (Vite dev server + Rust)
npm run tauri build                  # build installers (heavy; use CI instead, see docs/RELEASING.md)

cargo fmt --all --check              # Rust formatting
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace               # Rust unit + integration tests

npm run lint                         # oxlint + Prettier check (ui)
npm run typecheck -w ui              # TypeScript
npm test                             # Vitest unit tests (ui)
npm run e2e                          # Playwright e2e tests (ui, starts Vite)
npm run build                        # frontend production build
```
