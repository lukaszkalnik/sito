---
status: 🧪 ready for testing
---

# Slice 55 – Viewer bar on small screens

Since 2026-11-09; planned with the user and implemented 2026-11-12, ready for testing. Knowledge moved to [Viewer](../domain/timeline.md#viewer).

## Idea

Asked by the user on 2026-11-09, while testing [slice 48](48-compact-toolbar.md) on a MacBook: give the viewer bar ([Viewer](../domain/timeline.md#viewer)) the same clean-up that slice 48 gave the main window bar, so it stays readable and clear in a narrow window.

Found while testing (screenshot of a 1920 × 1080 Takeout photo): the bar shows name, date, resolution, file size, „data z Google Takeout” and the photo's folder in one line, and the folder is cut to „Zdjęcia Googl…”. The cut-off text has no tooltip and no hint that it is a folder, so the user could not tell what it was.

In a narrower viewer window the facts also ran under „Zaakceptuj” and „Odrzuć”; that was fixed while testing [slice 48](48-compact-toolbar.md#fixes-from-testing) (the folder gives way first, then the line is cut at its end). The facts cut off at the end are still not reachable.

## Context (measured 2026-11-12)

Measured in Playwright with the mocked viewer, for a worst case: a Takeout photo („PXL_20230715_143012345.MP.jpg”, „data poprawiona w Google Photos”, folder „Takeout/Zdjęcia Google/Zdjęcia z 2023”).

| Part | Width (px) |
|---|---|
| name | 230 |
| date („sobota, 1 czerwca 2024, 11:…”) | 211 |
| resolution | 83 |
| file size | 42 |
| date source note | 215 |
| folder | 246 |
| „Zaakceptuj” | 117 |
| „Odrzuć” (+ 4rem gap after it) | 98 + 64 |
| „Cofnij” (icon) | 36 |
| „Zamknij” | 76 |

The buttons with their gaps and the bar's padding take ~455 px; the whole info line needs ~1,100 px. So at 1,440 px the folder is cut to 92 px, at 900 px the date is already cut, and at the viewer's minimum width (480 px, `src-tauri/src/viewer.rs`) the info gets 0 px. The bar is 46 px high. A plain camera photo („IMG_1234.jpg”, no note) needs ~600 px of info.

The viewer's buttons have their own dark look (`viewer-close` in `PhotoViewer.css`, 1 px `#444` border, 30 px high); slice 48's shared look (`controls.css`) is not used there.

## Decisions (with the user, 2026-11-12)

- **Two lines of info**, like the path and summary of the main window bar: line 1 the name (bold) with its format badge („RAW”), then the folder; line 2 the date, resolution, file size and the date source note („data z Google Takeout”), separated by „·”. Each line is cut at its end, with the full line as tooltip. The bar's height is fixed (it does not depend on the photo).
- **The folder** gets a small, muted folder icon before it and is cut in the middle like the main window's path (its last folder stays readable as long as possible). Its tooltip names it: „Folder: Takeout/Zdjęcia Google/Zdjęcia z 2023”; screen readers also hear „Folder:”.
- **Narrow windows:** below a breakpoint (about 750 px, set by measuring so the info keeps ≥ ~300 px), „Zaakceptuj” and „Odrzuć” show only their icon (the check; the folder or trash). They keep their colours, accessible names and tooltips. The ~4rem gap after „Odrzuć” stays at every width.
- **„Zamknij” becomes an × icon button**, like „Cofnij”: its accessible name stays „Zamknij”, its tooltip is „Zamknij (Esc)”.
- **Look:** the viewer uses slice 48's shared controls (`controls.css`) with dark values of their tokens, set on `.viewer`, so the viewer stays dark also when the system is light. „Cofnij” and × are `.control-icon-button`s (icon in the accent colour), „Dopasuj” in the zoom badge a `.control-button`. „Zaakceptuj” and „Odrzuć” keep their own colours, but take the shared height, border width and radius. `.viewer-close` goes away. This covers what [slice 23](23-macos-polish.md) leaves for the viewer bar (apart from checking it on macOS).

## Implementation plan

### 1. Refactor first (`refactor:` commit, no behaviour change)

- `PhotoViewer.tsx` (266 lines) is split before new code goes in: `ViewerBar` and `RejectButton` move to `ui/src/features/viewer/ViewerBar.tsx`; `PhotoInfo`, `PhotoFacts`, `DateSource`, `FormatNote` and their tables to `PhotoInfo.tsx`. The bar's CSS (`.viewer-bar`, `.viewer-info`, `.viewer-name`, `.viewer-folder`, `.viewer-format`, `.viewer-date-source`, `.viewer-reject`) moves from `PhotoViewer.css` into `ViewerBar.css` / `PhotoInfo.css`.
- The middle-cut path becomes shared code (used by the timeline and the viewer), in a module named for what it is: `ui/src/paths/` with `pathTail.ts` (+ test, moved from `features/timeline/`) and `CutPath.tsx` + `CutPath.css` (today's `FolderPath` in `FolderInfo.tsx` and the `.folder-path*` CSS, renamed `.cut-path*`). It takes the `path` and an optional `title`; `FolderInfo` uses it unchanged in behaviour.
- `FolderIcon` moves out of `ChooseFolderButton.tsx` into `ui/src/features/scan/FolderIcon.tsx` (exported), so the viewer can draw the same folder. If `DestinationIcon`'s folder is the same path, it uses `FolderIcon` too.

### 2. Dark controls in the viewer

- `PhotoViewer.css`: `.viewer` (also `.viewer--empty`) sets the dark values of `--control-border`, `--accent`, `--bg` and `--text` (the ones from the dark block in `ui/src/index.css`), so `controls.css` draws dark controls there whatever the system theme. A comment says why.
- `controls.css`' header comment: the shared look of the main window bar *and the viewer bar*.

### 3. Icon buttons: „Cofnij” and ×

- `UndoButton` in the viewer bar and in `EmptyViewer` (`ViewerApp.tsx`): `className="control-icon-button"` instead of `viewer-close viewer-close--icon`.
- New `CloseIcon.tsx` (× outline, same style as `UndoIcon`: 24-unit viewBox, 2 px stroke, 18 px, `aria-hidden`). The close button is a `.control-icon-button` with `aria-label={t('viewer.close')}` and `title={t('viewer.closeHint')}` (new key: „Zamknij (Esc)”); still `onFirstClick(onClose)`.
- `.viewer-close`, `.viewer-close--icon` and their disabled rules go away; the step buttons keep their own class, with their border in `var(--control-border)`.
- `ZoomBadge`: „Dopasuj” gets `className="control-button"`; the `.viewer-zoom button` rule goes away.

### 4. „Zaakceptuj” and „Odrzuć”

- Both wrap their text in `<span className="bar-button-label">`. Below the breakpoint, `ViewerBar.css` hides that span visually (the `visually-hidden` technique, so the text stays the button's accessible name) and makes the padding square. No JS, no extra state: the names „Zaakceptuj” / „Cofnij akceptację” / „Odrzuć” and their tooltips stay as they are.
- Both take the shared height (`1.75rem`), a 1.5 px border and `box-sizing: border-box`; colours stay (green outline / solid green, solid red).
- The breakpoint is a single `@media (max-width: …)` value with a comment saying why, set by measuring (start at 750 px).

### 5. Two-line photo info (`PhotoInfo.tsx`, `PhotoInfo.css`)

- `.viewer-info` becomes a column of two lines (`<p>` each, one line, `overflow: hidden`, `text-overflow: ellipsis`, 0.85rem):
  - line 1: name (bold, `flex: 0 1 auto`, cut with an ellipsis), the format badge, then the folder: `FolderIcon` (14 px, muted), a visually hidden „Folder:” (new key `viewer.folderLabel`), and `CutPath` with `title={t('viewer.folder', { path })}` (new key „Folder: {{path}}”). The name gives way only after the folder's start is gone. The root folder keeps `timeline.rootFolder`.
  - line 2: date and time, resolution, file size and date source note, joined with „ · ” (as `scan.short` does in the main bar). The `<p>` has the whole line as `title`.
- The name gets its own `title` (the file name), for when it is cut.
- The bar aligns the info and the buttons to the centre; its height is fixed by the two lines (about 50 px), at every width and for every photo.

### 6. Tests

- `PhotoViewer.test.tsx`: the folder's tooltip „Folder: …” and accessible text; the root folder; the facts line's tooltip holds date, resolution, size and the Takeout note; „Zamknij” (found by name, now an icon) closes with its tooltip „Zamknij (Esc)”. Existing queries by name („Zaakceptuj”, „Odrzuć”, „Cofnij”, „Zamknij”) keep working; check for tests reading the buttons' text content or `.viewer-close`.
- `CutPath`: the `FolderInfo` tests keep covering it; `pathTail.test.ts` moves along.
- `ui/e2e/tauri-mock.ts`: an optional `viewerEntry` override (`name`, `relativeDir`, `timeSource`) for the shown photo, so e2e tests can show the Takeout worst case.
- New `ui/e2e/viewer-toolbar.spec.ts`, with the Takeout worst case, at 480 px, the breakpoint, one pixel above it, 900 and 1,440 px:
  - no two bar parts overlap (bounding boxes, as in `toolbar.spec.ts`), nothing overflows the bar;
  - the bar's height is the same at every width, and for a short plain photo;
  - below the breakpoint the buttons are found by their names „Zaakceptuj” and „Odrzuć” and are icon-sized;
  - from 900 px the name is not cut; at 1,440 px the folder's last part („Zdjęcia z 2023”) is not cut;
  - screenshots of the bar per width, to check the look.
- The narrow-window test in `viewer.spec.ts` (info clear of the buttons) moves into the new spec, adapted to the two lines.

### 7. Docs

- `docs/domain/timeline.md` (Viewer): the two lines, the folder with its icon, middle cut and tooltip, icon-only „Zaakceptuj” / „Odrzuć” in narrow windows, × for „Zamknij”, the shared dark look, the fixed height.
- `docs/domain/accepting.md` / `rejecting.md`, if they describe the buttons' look.
- `docs/slices/23-macos-polish.md`: the viewer bar now uses the shared look; what is left is checking it on macOS.
- This slice file and the roadmap: 🚧 → 🧪.

### Commits (on `main`, pushed)

1. `docs: plan slice 55` (this plan).
2. `refactor: split the viewer bar and share the middle-cut path`.
3. `feat: compact viewer bar for small screens` (steps 2–7, slice 🧪).

## Verification

- `npm run lint`, `npm run typecheck -w ui`, `npm test` and `npm run e2e` (viewer-toolbar, viewer, toolbar, accept, reject, follow-viewer specs).
- Debug build (`npm run tauri dev`), started for the user: open a Takeout photo and a RAW photo, resize the viewer from full screen down to 480 px and check that nothing overlaps, the bar keeps its height, the folder's tooltip names it, and the icon-only buttons still read clearly; step through photos with the keyboard and check the focus ring on the dark buttons.

## Builds on

- [Slice 48](48-compact-toolbar.md): `Field` (caption above a control), `controls.css`, the icon buttons (`UndoButton`, `UndoIcon`, `ChooseFolderButton`), the middle-cut path (`pathTail.ts`, `FolderInfo.tsx`) and the toolbar e2e checks.

## Implementation notes (2026-11-12)

- **Breakpoint 780 px** (`ViewerBar.css`), not 750: with the photo accepted, „Cofnij akceptację” (156 px) is the widest label, and the info then gets 281 px at 751 px but 310 px at 781 px (349 px for a photo not accepted). `viewer-toolbar.spec.ts` checks 480, 780, 781, 900 and 1,440 px, and ≥ 300 px of info with the photo accepted above the breakpoint.
- The bar is 54 px high (two lines at line height 1.4), the same at every width and for a plain camera photo.
- The viewer sets `color-scheme: dark` besides the dark token values, so system colours (the greyed „Cofnij” with only trash moves left, `GrayText`) are dark-mode ones too.
- `DestinationIcon`'s folder is a closed folder, a different drawing from `FolderIcon` (open folder), so it stays as it is.
- The facts line is built from the strings in `PhotoInfo.tsx` (`factTexts`), each in its own span with „ · ” between them, so the tooltip and the shown text are the same.
