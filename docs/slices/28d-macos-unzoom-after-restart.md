---
status: 🚫 rejected
---

# Slice 28d – Un-zooming after a restart on macOS

Found while testing [slice 28a](28a-macos-small-screen-restore.md) on the dev Mac (2026-11-17). Open questions answered on the dev Mac and the plan written on 2026-11-17. **Rejected by the user on 2026-11-17:** the impact is too small for the fix, which would replace the window-state plugin. Only the place a window un-zooms to after a restart is off; it still comes back maximized at the right size, on its screen, and every other window place is unaffected. The findings and the plan stay here in case it is picked up again.

## The bug

On macOS, a window that was zoomed (maximized) when Sito quit comes back zoomed, but un-zooming it puts it in the top-left corner of the screen instead of the frame it had before the zoom. Its size is right; only its position is wrong. Within one run of the app un-zooming is right, because macOS itself keeps the frame from before the zoom. Behaviour today is in [windows.md](../domain/windows.md).

## Cause

- [slice 28a](28a-macos-small-screen-restore.md) places a window from the window-state plugin's file (`window_fit::saved`); for a maximized window it takes the position before maximizing from the plugin's `prev_x`/`prev_y`, as the plugin's own restore did before. It sets that frame, then maximizes, so macOS remembers that frame as the one to un-zoom to.
- The plugin (tauri-plugin-window-state 2.5.0, `src/lib.rs`, its `WindowEvent::Moved` handler) copies `x`/`y` into `prev_x`/`prev_y` on **every** move, without asking whether the window is maximized. The zoom animation on macOS sends about ten `Moved` events, so `prev` ends up as the animation's second-to-last point.
- The saved inner size (`width`, `height`) is right: the plugin's `Resized` handler and its save on close skip a maximized window.
- So the file holds no correct pre-zoom position; nothing on the restore side alone can fix this.

## Findings (dev Mac, 2026-11-17)

A temporary log (not committed) in a debug build printed every `Moved`/`Resized` with what the window reported while handling it: `is_maximized()`, `is_minimized()`, `outer_position()`, `inner_size()`. The window was zoomed and un-zoomed through the Window menu's Zoom (the same action as a double-click on the title bar). Physical pixels, Retina screen at 2×.

- **Zoom** (window at 106,130, 2560×1600): 11 `Moved` events with the animation's points (104,130 → 102,126 → … → 2,62 → 0,60), each followed by a `Resized`. They all arrive after the animation has finished: in **every** event `is_maximized()` is already `true` and the getters report the final frame (0,60, 2772×1740). The `Resized` payloads are the final size too; only the `Moved` payloads carry the animation.
- **Un-zoom:** the same in reverse (4,62 → … → 106,130); in every event `is_maximized()` is already `false` and the getters report the final frame 106,130, 2560×1600.
- **The file after zooming and quitting:** `x: 0, y: 60, prev_x: 4, prev_y: 62, width: 2560, height: 1600, maximized: true`. The bug, reproduced.
- **Restart:** `place` set 4,62 and 2560×1600, then maximized. All 17 events from that arrive with `is_maximized()` already `true`, so none of them shows the frame that was set. Un-zooming then went to exactly 4,62: macOS un-zooms to the frame `place` set, so the restore path is right and only the saved position is wrong.
- **A window that is never moved sends no events.** Opened at its default place and zoomed straight away, a window has never reported a normal frame through an event.

Answers to the open questions:

- **Is "the last frame while not maximized" reliable from events on macOS?** Yes, if it is read from the window's getters while handling the event (not from the `Moved` payload) and an event is skipped while `is_maximized()` is true. It also needs a starting value: the frame the window was placed at, because a window that is not moved sends nothing, and after a restart even the placing events arrive already maximized.
- **Windows and Linux:** not checked yet, and not checkable from this Mac. On Windows, maximizing is not animated by the app, so the plugin probably saves the right `prev`; what matters for the plan is whether `is_maximized()` is already true when the maximize's `Moved`/`Resized` are handled (otherwise the maximized frame would be taken as the normal one), and that a minimized window (-32000) reports `is_minimized()`. On Linux (GTK) maximizing goes through the window manager and may report `is_maximized()` late; the same question. Both are answered by the restart spec's new maximize case (step 2) on the CI runners, and on Windows also by the same temporary log on the Windows machine when it is next at hand.
- **The viewer** goes through the same code (`show_placed`, the plugin's handler for every window), so it behaves the same; not tried separately.

## Candidate approaches

1. **Track the normal frame ourselves.** On `Moved`/`Resized` of a window that is not maximized (and not minimized), keep its outer position and inner size; save that with the state on close (our own small file, or our own field next to the plugin's). `placement` then uses our position for a maximized window instead of `prev_x`/`prev_y`.
2. **Read the frame at the moment of maximizing**: keep the last normal frame and freeze it on the first event where the window reports maximized. Same data as 1, decided by event order.
3. **Replace the plugin's tracking and saving entirely** (noted in 28a's "Settled at the review" as a possible slice of its own). Larger; `placement` and the restore path would stay as they are.

Approach 2 is the same as 1 given the findings (every zoom event already reports maximized). Approach 1 cannot live next to the plugin: the plugin keeps its state private and rewrites its whole file on every close and on app exit, so a corrected `prev` written into its file is overwritten, and a second file would duplicate what the plugin saves. Tracking the normal frame ourselves is most of what the plugin does, so the plan is approach 3 in a form that keeps the plugin's file.

## Plan

Sito tracks and saves the window state itself, in the same file and format the plugin used, and the plugin is removed. Keeping the file (`.window-state.json` in the app config folder, same fields) means existing users keep their window places, and the restart spec, the smoke test (`e2e-app/smoke-test.sh`) and the workflows that read or remove the file stay as they are.

One commit per step, checks green.

1. **Windows remember their frame from before maximizing (`fix:`).** In `src-tauri/src/window_fit/` (the file `window_fit.rs` keeps the Tauri glue and `placement`):
   - **Domain (`window_fit/tracked.rs`, pure):** `Tracked { normal: Frame, maximized: bool }`, where `Frame` is an outer position and an inner size, and `observe(&mut self, seen: Seen)`, where `Seen` is what the window reports (frame, maximized, minimized). A window that reports neither maximized nor minimized updates `normal`; `maximized` follows what the window reports unless it is minimized (a minimized window keeps the flag it had, so one minimized from maximized comes back maximized). Tests feed the sequences logged above: the zoom (all events maximized, `normal` stays at the frame from before), the un-zoom (`normal` becomes the final frame), the restart (seeded with the placed frame, every placing event maximized, `normal` stays the seed), a minimized window on Windows (-32000, ignored).
   - **Data (`window_fit/state_file.rs`, replacing `saved.rs`):** reads the whole file into `label → Tracked` and writes it back. Reading stays as today for files the plugin wrote (a maximized entry takes `prev_x`/`prev_y`; zero size or a missing file means nothing remembered; an unreadable file is a warning). Writing puts the normal position into both `x`/`y` and `prev_x`/`prev_y`, the normal inner size into `width`/`height`, `maximized`, and the plugin's other fields with their defaults (`visible: true, decorated: true, fullscreen: false`), so the file stays readable by today's reader and by the plugin, should it ever come back. Entries of windows that are not open (the viewer, once closed) are kept. Tests: a plugin-written file, a round trip, a window that is not open keeps its entry.
   - **State:** `WindowStates(Mutex<HashMap<String, Tracked>>)`, managed by the app, loaded from the file in `setup`. The lock is held only to read or update the map, never while calling the window.
   - **Tauri glue (`window_fit.rs`):**
     - `show_placed` seeds the window's entry with the frame it sets (the placement before maximizing, and whether it maximizes); on first launch, with the frame `fit_to_screen` sets, or the default frame read before anything is set when it fits. So a window never moved still has its normal frame.
     - `on_window_event` replaces `save_on_close`: on `Moved`/`Resized` it reads the window's getters and calls `observe` (not the event's payload, which on macOS is a point of the animation); on `CloseRequested` it saves.
     - `save(app)`: for every open window, observes it once more (whether it is maximized now), then writes the file. Called on `CloseRequested`, on `RunEvent::Exit` (Cmd+Q on macOS quits without closing the windows first; the plugin saved there too, so `lib.rs` switches from `.run(context)` to `.build(context)?.run(|app, event| …)`), and by the updater's `before_exit` instead of `save_window_state`.
     - `placement` and `place` stay as they are.
   - **The plugin goes:** `tauri-plugin-window-state` leaves `src-tauri/Cargo.toml` and `lib.rs`, `STATE_FLAGS` goes, the viewer's comment on the plugin's lock is rewritten (placing stays on the main thread: the window's getters and setters run there anyway). `npm run notices:rust` regenerates the notices; `cargo audit`.
2. **The restart spec covers a maximized window (`test:`).** `e2e-app/specs/restart.spec.ts` gets a second case: move the main window to a known frame, maximize it through the window plugin's commands (as the spec already moves and resizes; the e2e capability gets `allow-maximize`/`allow-unmaximize`/`allow-is-maximized` if it lacks them), quit, start again, check that it is maximized, un-maximize it, and check that its frame is the one from before. The case is checked to fail without step 1 (locally on the Mac, where `set_maximized` zooms with the animation, before step 1 is committed). On Linux under Xvfb without a window manager, maximizing may do nothing; then the case is skipped there with that reason in the spec, which also answers the Linux question above.
3. **Docs (`docs:`).** `windows.md` drops the known exception and says that a window remembers the frame it had before it was maximized. `CLAUDE.md` (tech stack) and `docs/ARCHITECTURE.md` no longer list the `window-state` plugin. This file and the roadmap get the status.

## For review

- **Removing the plugin** reverses 28a's "the plugin stays". The reasons given there were the platform quirks of the saving half (a minimized window at -32000, a maximized one reporting the screen's frame); the plugin handles both only by skipping minimized and maximized windows, which `Tracked::observe` does the same way. Keeping the plugin and patching its file is not workable (it rewrites the file on exit after us).
- **Same file and format** rather than a new file of our own: no migration, and no changes to the e2e tooling and workflows. The cost is a format chosen by the plugin with fields we do not use (`visible`, `decorated`, `fullscreen`, and `prev_x`/`prev_y` duplicating `x`/`y`).

## Verification

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, e2e-app lint and typecheck; `npm run e2e:app` on the Mac, with the new case failing before step 1 and passing after it.
- By hand on the dev Mac: zoom the main window, quit, start, un-zoom: it returns to its frame from before the zoom. The same for the viewer. Move and resize both windows, restart: both come back. Remove the state file: both open centred.
- CI: `e2e-app.yml` green on Windows, Linux and macOS, with the maximize case running (or skipped with its reason on Linux).
- On the Windows machine when next at hand: the same by hand, maximizing with the title bar button and with Win+Up.

## Tests

- Unit tests of `Tracked::observe` with the event sequences logged on the dev Mac (see Findings), and of reading and writing the state file.
- The real-app restart spec (`e2e-app/specs/restart.spec.ts`) gets the maximize case (step 2).
