# Sito – Project Context

> "Sito" is Polish for *sieve*: the app sifts through photo series and keeps the best shot.

Desktop app that finds **series of similar shots** in a large photo gallery and suggests the best photo of each series. Domain knowledge is in [docs/domain/](docs/domain/README.md) (see [Where knowledge lives](#where-knowledge-lives)).

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
- **Commit and push every self-contained change, directly on `main`.** No feature branches or pull requests: always commit on `main` and push to `main`. One logical change per commit, Conventional Commits style (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`, `ci:`), with tests and checks green. Push right after committing.
- **Pause at process milestones so the context can be cleared.** At each point below, stop before starting the next piece of work and offer the user the chance to clear the context:
  - a plan is approved and saved into its slice file (before implementing it);
  - a slice is implemented and ready for testing (status 🧪);
  - testing of a slice is finished (the user's OK);
  - a release is published and installed. Releases are made only when the user asks for one, not after every slice; between them the user tests debug builds.

  Before pausing, make sure the slice files and the roadmap record where things stand (status, what is done, what comes next), so a fresh session can continue from them alone.
- **Every GitHub release has a changelog in English**, written into `CHANGELOG.md` (one `## vX.Y.Z – YYYY-MM-DD` section per version) and committed with the version bump; the release workflow publishes that section as the release notes and refuses a tag without one. Write it from the user's perspective and at a high level (grouped as New / Improved / Fixed), not as a list of commits. Name what got better, not just that something did: not "performance improvements" but e.g. "UI responsiveness while loading a large photo collection" or "opening a photo in the viewer is near-instant in folders with tens of thousands of photos". Cover everything since the previous release tag (`git log vPREV..HEAD`), including slices that were never released.
- **Code style and architecture:** see [Code and architecture rules](#code-and-architecture-rules) below.
- **End-to-end tests** where possible: integration tests in `crates/core/tests/` run the full pipeline on `fixtures/`; UI flows are tested with Playwright against the Vite dev server with mocked Tauri commands. Run them before pushing changes that touch the affected flow.
- **Local tool locations (Windows dev machine):** all toolchains live under `D:\Lukasz\tools` (C: is nearly full): Rust (`RUSTUP_HOME`, `CARGO_HOME`), Node, GitHub CLI, uv, npm cache/global prefix (`npm_config_cache`, `npm_config_prefix`), Playwright browsers (`PLAYWRIGHT_BROWSERS_PATH`), VS Build Tools. Never install large tools or caches on C:.
- **Low memory on the dev machine (7 GB RAM):** never run several heavy jobs at once (installers, Rust builds, browser tests). The user env var `CARGO_BUILD_JOBS=2` limits parallel rustc jobs locally (CI uses all cores).

## Code and architecture rules

Apply these rules both when **writing** and when **reviewing** code (including your own diff before committing). When touching existing code that breaks them, improve it in a separate `refactor:` commit rather than mixing it into a feature.

Lints enforce part of them, so CI fails on a violation: clippy (`Cargo.toml`, `clippy.toml`: functions over 50 lines, `_ =>` over enums, missing docs in `core`, `unwrap`/`expect`) and oxlint (`ui/.oxlintrc.json`: functions over 50 lines, files over 400 lines, nesting over 3, index keys, `!` assertions). Do not silence a lint to get green; fix the code, or use `#[expect(..., reason = "...")]` / an `oxlint-disable-next-line` comment with a reason where the rule truly does not fit.

### Structure

- **Package by feature first, then by layer.** Top-level folders/modules are features (`scan`, `timeline`, `series`, `reject`, `viewer`, ...), not technical kinds (`api/`, `utils/`, `types/`). Inside a feature, split by layer only when it grows:
  - **UI** (presentation): React components, CSS. Rust side: the feature's Tauri commands, events and protocols in `src-tauri/src/<feature>.rs` (submodules in `src-tauri/src/<feature>/`).
  - **Domain** (pure logic): types, rules, reducers, calculations. No I/O, no framework imports; this is where most unit tests live.
  - **Data** (I/O): SQLite, file system, Tauri `invoke`/`listen` wrappers (`ui/src/features/<name>/api.ts`), `localStorage`.
  - Allowed dependencies: UI uses Domain and Data; Data uses Domain; Domain uses neither.
  - Code shared by several features gets a module named for what it is, not a kind folder (`src-tauri/src/error.rs`, `protocol.rs`, `ui/src/shortcuts/`); app shell code (`lib.rs`, `window_fit.rs`, `smoke.rs`) stays at the top.
  - Across crates: `ui` calls `src-tauri`, which calls `core`. `core` (domain + data) depends on neither and never on Tauri.
- **Small, focused files.** One concept per file. Aim for < 300 lines of non-test code; past ~400 lines, split by sub-concept (Rust unit tests stay in the file's `#[cfg(test)]` module and do not count) (Rust: keep `foo.rs` and put submodules in `foo/`, no `mod.rs`). A large file is fine only when the domain genuinely is large and cohesive (e.g. a long `match` over many cases).
- **Small functions.** A function does one thing at one level of abstraction; aim for < 50 lines. Extract private helpers with descriptive names instead of comments explaining blocks.
- **Public surface stays minimal.** Rust: private/`pub(crate)` by default, re-export the intended API from `lib.rs`. TypeScript: export only what other modules use; no barrel files that re-export everything.

### Complexity

- **Low cyclomatic complexity and shallow nesting** (max ~3 levels). Prefer early returns / guard clauses, `?` in Rust, `let ... else`, iterator chains (`filter`, `map`, `fold`) and lookup tables over nested `if`/`for`.
- Replace boolean flag parameters with an enum or two functions.
- No clever code: readability beats brevity.

### Duplication

- Before writing a new function or type, **search for near-duplicates** (same shape, slightly different names or fields). In reviews, actively look for them.
- Extract a shared abstraction when the duplicates express the **same concept** and would change together; leave them separate when the similarity is accidental. Rule of thumb: extract on the third occurrence, or on the second if the logic is non-trivial.
- Frontend and backend types that mirror each other (command payloads, events) are defined once per side and kept in sync deliberately; consider generating TS types from Rust (`ts-rs`/`specta`) once they multiply.

### Types: make illegal states unrepresentable

- Model states with **enums / discriminated unions**, not combinations of booleans and optionals. Bad: `{ loading: boolean; error?: string; data?: T }`. Good: `type State = { kind: "idle" } | { kind: "loading" } | { kind: "error"; message: string } | { kind: "done"; data: T }`.
- Rust: enums with data (`enum Decision { Keep, Reject { moved_to: PathBuf } }`), exhaustive `match` (no `_ =>` catch-all over own enums, so new variants cause compile errors). TypeScript: discriminated unions with a `kind` field, exhaustive `switch` ending in an `assertNever(x: never)` check.
- **Newtypes** for values that must not be mixed up (`PhotoId(i64)`, `SeriesId`, durations vs. timestamps). Parse and validate at the boundary (`TryFrom`, constructors returning `Result`), then trust the type inside.
- **Dates and times:** EXIF times have no time zone. Keep them as `chrono::NaiveDateTime`, never convert through the machine's local time zone implicitly, and use distinct types for points in time and durations.
- Prefer immutable data: `readonly` fields/arrays in TS, no `mut` unless needed in Rust.

### Comments

- **Doc comments** (`///`, `//!`, `/** */`) say **what** a function, type or module does or represents: its contract, inputs, outputs, errors, units and invariants. They do **not** say where or by whom it is used; callers change and such comments go stale.
- **Inline comments** explain **why** (a non-obvious decision, a workaround, a platform quirk), never restate what the code does.
- Every public item in `core` has a doc comment. No commented-out code.

### Safe file operations

- Files are only ever **moved**, never deleted or overwritten. If the destination name exists, pick a new name.
- A move across drives (where `rename` fails) must copy, verify the copy (size + hash), and only then remove the original; otherwise fail visibly.
- Write the undo log entry **before** the move, in the same SQLite transaction as related changes, so a crash never leaves an untracked move.
- Handle paths as `Path`/`PathBuf`, never via lossy string conversion; support non-ASCII (Polish) names and long Windows paths.

### Scale and performance

- Assume galleries of 50 000+ photos. Never hold all decoded images in memory; stream or batch work.
- Keep IPC payloads small: send IDs and metadata, never image bytes (images go through the `thumb:`/`photo:` protocols).
- Long lists in the UI are virtualized. Long jobs are cancellable and throttle their progress events.

### Persistence (SQLite)

- Durable data (the action log) changes its schema only through numbered migrations (`PRAGMA user_version`, see `crates/core/src/actions.rs`); never edit an applied migration. Pure caches (the photo cache) may instead be emptied when their version changes, since they can be rebuilt.
- Multi-step changes (a move plus its undo entry, a batch of decisions) run in one transaction.
- SQL lives only in the data layer of `core`.

### Errors

- Commands return **error codes** (serializable enums), not English messages; the UI maps every code to a translated message in `pl.json`.
- Never swallow an error silently: show it to the user or log it.

### Security and privacy

- No network access except the checksum-verified model download and the signature-verified update check and download (Rust side only, see `docs/domain/updates.md`).
- Tauri capabilities and the CSP stay minimal; every new permission or allowed source needs a stated reason.
- Logs stay on the machine (the logger only runs in debug builds) and never contain image data.

### Testing

- Test behaviour, not implementation details. The pure domain layer carries most of the tests.
- Frontend tests query by role, label or text (Testing Library), not by CSS class.
- Tests are deterministic: no `sleep`, no real clock or randomness (inject or fake them), file system work in temp dirs, never on real photo folders.

### Accessibility

- Everything is usable by keyboard: native `<button>`/`<input>` elements, visible focus. Every shortcut is an action in `ui/src/shortcuts/keymap.ts` (one key per action, checked by a test); features bind their handlers with `useShortcuts` and never listen to `keydown` themselves.
- Icon-only controls have an `aria-label` from the translation file.

### Dependencies

- Add dependencies sparingly; check each license (it ends up in the third-party notices) and prefer well-maintained crates/packages.
- Commit lockfiles (`Cargo.lock`, `package-lock.json`).
- Every shipped dependency's license must be on a reviewed list: `about.toml` (Rust, checked by `cargo-about` in CI) and `ALLOWED_NPM_LICENSES` in `ui/build/thirdPartyNotices.ts` (npm, checked by every frontend build). Add a license there only after checking its terms. After changing Rust dependencies, run `npm run notices:rust` and commit `ui/notices/rust-crates.txt`; CI fails if it is stale.

### Rust best practices

- Errors: `thiserror` enums per module in `core`; propagate with `?`; no `unwrap`/`expect`/`panic!` outside tests. For a true invariant, use `#[expect(clippy::expect_used, reason = "...")]` on the item. `src-tauri` maps errors to serializable command error codes.
- Borrow over clone: take `&str`/`&Path`/`&[T]` in parameters, return owned values. Use `impl AsRef<Path>` / `impl IntoIterator` where it helps callers.
- Derive common traits (`Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Default`) where meaningful; implement `From`/`TryFrom` for conversions instead of ad-hoc `to_x` functions.
- Prefer iterators over index loops. Use `rayon` for CPU-heavy work. Never block an async command: run file and database work in `spawn_blocking` or the worker pool.
- Tauri commands are thin: parse input, call `core`, map the result. Long work runs off the main thread and reports progress via events. Shared state lives in `tauri::State` behind the narrowest lock possible; never hold a lock across an `await` or a long computation.

### TypeScript / React best practices

- Strict TS, no `any`, no non-null `!` assertions without a comment; validate untrusted data (Tauri responses, `localStorage`) in the data layer.
- Function components; components render, hooks hold logic. Move non-trivial logic into pure functions (reducers, selectors) next to the feature so it is testable without React.
- Derive values instead of storing them in state; keep state minimal and as local as possible. Use `useReducer` with a discriminated-union state for multi-step flows.
- Effects only for synchronizing with external systems (Tauri events, timers, DOM); always clean up listeners and subscriptions.
- Stable `key`s from IDs (never array indices for dynamic lists); memoize (`useMemo`, `useCallback`, `memo`) only where profiling or large lists justify it.
- All Tauri `invoke`/`listen` calls go through typed wrappers in the data layer, so components never call Tauri directly and e2e tests can mock them.

### Python

- Dev scripts (`scripts/`) follow PEP 8 with type hints.

## Tech stack

- **App shell:** Tauri 2 (Rust backend, system webview). Plugins: `dialog` (folder picker), `window-state` (window size and position), `log` (debug builds only).
- **Rust** (Cargo workspace: `crates/core` = `sito-core`, `src-tauri` = `sito`): `walkdir` (folder walk), `nom-exif` (EXIF), `image` + `jpeg-decoder` (decoding, downscaled JPEG decode for thumbnails), `blake3` (thumbnail cache keys), `chrono` (capture times), `rayon` (parallel reading), `rusqlite` with bundled SQLite (photo cache, action log), `thiserror` (error types), `serde` (IPC payloads). Planned: `ort` (ONNX Runtime; DirectML on Windows, CoreML on macOS, CPU fallback).
- **Frontend** (npm workspace `ui`): React 19, strict TypeScript, Vite, `i18next` + `react-i18next` (translations), `@tanstack/react-virtual` (virtualized timeline), `@tauri-apps/api` (IPC).
- **Tooling:** rustfmt, clippy (pedantic), oxlint, Prettier, Vitest + Testing Library + jsdom (unit tests), Playwright (e2e).

## Repository layout

```
sito/
├─ CLAUDE.md
├─ CHANGELOG.md       # release notes, one section per version (English)
├─ Cargo.toml         # Rust workspace (crates/core, src-tauri)
├─ package.json       # npm workspace (ui) + Tauri CLI
├─ crates/core/       # pure logic, no UI code
├─ src-tauri/         # thin layer: commands and progress events
├─ about.toml, about.hbs  # cargo-about: accepted Rust licenses, notices template
├─ ui/                # React + TypeScript
│  ├─ src/i18n/pl.json
│  ├─ build/          # Vite plugin writing THIRD-PARTY-NOTICES.txt
│  ├─ notices/        # generated Rust notices (committed)
│  └─ e2e/            # Playwright tests
├─ docs/domain/       # domain knowledge, one file per topic
├─ docs/ROADMAP.md    # work plan (vertical slices)
├─ docs/slices/       # one plan per slice (NN-name.md)
├─ docs/RELEASING.md  # how to build, release and install
├─ scripts/           # dev helper scripts (icon rendering)
├─ fixtures/          # small test photos with known groups
└─ .github/workflows/
```

## Architecture

- `core` processes photos as a pipeline of stages (scan, group, analyse, score, store, act); each stage is its own feature module with pure logic separated from I/O. What each stage does is domain knowledge (see [Where knowledge lives](#where-knowledge-lives)).
- Image work decodes downscaled as early as possible and always respects the EXIF orientation.
- Derived data (metadata, embeddings, scores) is cached in SQLite keyed by file identity (path + size + mtime), so a rescan only processes new or changed files.
- Tunable values (thresholds, score weights) are settings, not constants scattered in code.
- **ML models** are not committed to the repo. They are downloaded on first launch and verified by checksum; check a model's license before adding it.

## CI (GitHub Actions)

- **Pull requests:** `cargo fmt --check`, `cargo clippy`, `cargo test`, plus frontend lint and build, on both `windows-latest` and `macos-latest`
- **Release** (`v*` tag or manual run): `tauri-action` builds a per-user NSIS installer (Windows) and a DMG (macOS); the Windows installer is smoke-tested (install, launch, open the viewer, uninstall; see `src-tauri/src/smoke.rs`) before the release is published with the tag's `CHANGELOG.md` section as its notes. Unsigned for now; macOS signing and notarization come later. See [docs/RELEASING.md](docs/RELEASING.md).

## Where knowledge lives

Think of it as a team's tools: this file is the coding handbook, `docs/domain/` is the wiki (Confluence), `docs/slices/` are the tickets (Jira) and `docs/ROADMAP.md` is the board.

- **`CLAUDE.md` (always loaded):** how to write code for this project: stack, layout, layers, rules and workflow. No domain knowledge and no feature descriptions.
- **`docs/domain/` (read on demand):** what the app works on and how it behaves **today**, one file per topic, indexed in [docs/domain/README.md](docs/domain/README.md). Before planning or changing behaviour, read the index and the topic files the work touches.
- **`docs/slices/NN-name.md` (tickets):** one file per planned change, from the moment it is added (also drafts; loose ideas go into `docs/ideas/`). A slice holds the idea, candidate approaches and the approved plan; it links to domain files instead of repeating them. Plan each slice in plan mode first and save the approved plan into its file. Once a slice is done its file is history: **move what it established into `docs/domain/`** in the same work, and do not keep the slice up to date afterwards.
  - **Consider related future slices.** When planning or implementing a slice, read the concepts of related later slices (same files, same data, same kind of feature) and build the shared pieces generically now, so the later slice only adds its own part. Note in the later slice's file what it can build on.
  - **Fixes from testing get a sub-slice.** Fixes for problems found while testing a slice get a sub-slice of their own (e.g. `27a`, file `docs/slices/27a-name.md`), placed in the roadmap right after that slice. Every piece of work is documented in the slice it belongs to; never mix work of different slices in one slice file.
- **[docs/ROADMAP.md](docs/ROADMAP.md) (board):** only a table of links to the slices with each slice's status, never a description. Slices are small vertical slices, each testable in the running app. Slice numbers are permanent IDs (never renumbered; a new slice takes the next free number); the table's order decides what comes next. Keep the status in the roadmap and the slice file in sync.

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
npm run notices:rust                 # regenerate Rust license notices (needs `cargo install cargo-about --features cli`)
```
