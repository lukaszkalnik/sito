---
status: 🧪 ready for testing
---

# Slice 48 – Main window bar on small screens

Since 2026-11-07; planned and implemented 2026-11-09, next: testing by the user. Knowledge moved to [Timeline](../domain/timeline.md#main-window-bar) and [Rejecting](../domain/rejecting.md).

## Idea

Asked by the user on 2026-11-07, with a screenshot of the main window on a MacBook screen: the bar above the grid ([Main window bar](../domain/timeline.md#main-window-bar)) gets crowded on a small screen. The folder path and the summary are cut to „/Users/lukasz/Pictures/Sk…” and „Znaleziono 1461 zdjęć. 24…”, while the controls next to them take most of the width with their inline labels („Odstęp w serii”, „Odrzucone zdjęcia do:”).

Clean the bar up so it stays readable and usable in a narrow window.

## Context (measured 2026-11-09)

The bar above the grid (`ui/src/features/timeline/TimelineView.tsx`) puts everything in one flex row with inline labels. Measured in Playwright with the mocked app, the controls take ~1,210 px:

| Part | Width |
|---|---|
| „Sito” | 37 |
| gap group (label, −, slider 8rem, +, value) | 350 |
| „Serie / Wszystkie zdjęcia” switch | 194 |
| icon + „Odrzucone zdjęcia do:” + select | 325 |
| „Cofnij” | 50 |
| „Wybierz inny folder” | 129 |

So the path and summary get 227 px at 1,440 px wide and 67 px at 1,280 px. At the window's minimum width of 900 px (`tauri.conf.json`) they get 0 px, the switch collapses to 2 px and „Wybierz inny folder” wraps onto three lines (the bar grows to 79 px). The bar's buttons are also unstyled browser defaults, which slice 23 flags as not native-looking on macOS.

## Decisions (with the user, 2026-11-09)

- **Captions above the controls:** „Odstęp w serii”, „Widok” and „Odrzucone zdjęcia do:” become a small caption line above their control instead of a label beside it. The bar already has two text lines (path, summary), so this barely adds height, and the height stays fixed.
- **„Serie / Wszystkie zdjęcia” becomes a dropdown „Widok”**, built from a list of views so that slice 44 only adds an entry.
- **„Wybierz inny folder” becomes a folder icon button just before the path**, with the text as `aria-label` and tooltip.
- **„Cofnij” becomes an icon button everywhere**: the main bar, the viewer bar and the empty viewer (a curved „undo” arrow; asked by the user during planning, the viewer for consistency). Its accessible name stays „Cofnij” (`aria-label`), and its tooltip stays `undo.hint` („Przywróć ostatnio odrzucone zdjęcie (Ctrl+Z)”). In the viewer bar, „Odrzuć” keeps its ~4rem gap to „Cofnij”, so the two are still not hit by mistake.
- **Narrow windows:** below a breakpoint (about 1,100 px, set by measuring so the path and summary keep ≥ ~300 px), the path and summary move to a full-width second row of the bar. The height depends only on the window width, never on content or background work (slice 37 stays intact).
- **Restyle the bar's buttons and dropdowns** in one shared look (border, 6px radius, the same height as the gap −/+ buttons, visible focus ring). This covers slice 23's note for the main window; the viewer bar is left alone.

## Implementation plan

### 1. Refactor first (`refactor:` commit, no behaviour change)

- Move `GapSlider` out of `TimelineView.tsx` into `ui/src/features/timeline/GapSlider.tsx`, and its CSS (`.timeline-gap*`) into `GapSlider.css`. `TimelineView.tsx` (284 lines) shrinks before new code goes in.

### 2. Shared bar controls (`ui/src/controls/`)

This is code shared by the timeline and reject features, so it goes in a module named for what it is (CLAUDE.md, Structure).
- `Field.tsx`: `Field({ caption, children })`. It renders `<div className="field">` with a `<label htmlFor={id} className="field-caption">` above the control. `children` is a render function receiving the `useId` id, so the caption is the control's accessible name. It is a column flex with a small caption (~0.75rem, muted colour, `white-space: nowrap`).
- `controls.css`: `.control-button` and `.control-select` share one look: `1px solid var(--border)`, radius 6px, height 1.75rem, `font: inherit`, 0.85rem, transparent background, `:focus-visible` outline in `var(--accent)`, and a disabled look. The existing `aria-disabled` rule for „Cofnij” moves here. `.control-icon-button` is the square variant.
- `Field.test.tsx`: the control is found by role with the caption as its name.

### 3. View dropdown (`ui/src/features/timeline/`)

- `views.ts` (domain): `VIEW_LABELS: Record<TimelineMode, 'view.series' | 'view.all'>` in display order. The `Record` makes TypeScript require an entry for every mode, so slice 44 adds its mode to `TimelineMode` (`layout.ts`) and one line here. Also `parseView(value: string): TimelineMode | null`, which looks the value up in the table (no cast). `views.test.ts` checks the order, parsing and an unknown value.
- `ViewPicker.tsx`: `<Field caption={t('view.label')}>` + `<select className="control-select">` with one `<option>` per entry. It replaces `ModeToggle` and its `.timeline-mode` CSS. `TimelineMode` stays a string union. If a later view needs data (slice 44: a mark kind), it then becomes a discriminated union with a `kind`; note this in slice 44.

### 4. Captions for the gap slider and the destination

- `GapSlider`: the label becomes the `Field` caption above the `[−] slider [+] 10 s` row. The slider stays 8rem, unless measuring at the breakpoint says a narrower one is needed. `title={t('gap.hint')}` stays on the group.
- `DestinationPicker` (`ui/src/features/reject/DestinationPicker.tsx`): caption „Odrzucone zdjęcia do:” above, then the icon and the `select.control-select`. The trash look (`--danger`, bold) stays. The accessible name stays „Odrzucone zdjęcia do:”, so `DestinationPicker.test.tsx` keeps working.

### 5. Folder button and bar layout (`TimelineView.tsx`, `TimelineView.css`)

- New `ChooseFolderButton` (in `ui/src/features/scan/`, next to the scan feature that owns choosing a folder): a `.control-icon-button` with an inline outline folder SVG (`aria-hidden`), `aria-label` and `title` = `t('scan.chooseAnother')` (no new strings), disabled while `busy`. It sits at the start of the info area, before path and summary.
- The bar becomes a CSS grid with template areas:
  - wide: `"title info controls"`, with `info` as `minmax(0, 1fr)`;
  - narrow (`@media (max-width: <breakpoint>)`): `"title controls" / "info info"`.

  The controls are one flex row aligned to the bottom (`align-items: end`), so „Cofnij” (no caption) lines up with the other controls. The info area keeps its one-line-each ellipsis and tooltips. The breakpoint is a single CSS value with a comment saying why it was chosen.
- „Cofnij” as an icon: `UndoButton` (`ui/src/features/reject/UndoButton.tsx`) renders an inline outline curved-arrow SVG (`aria-hidden`, `stroke="currentColor"`) instead of the text, with `aria-label={t('undo.button')}`. The `title`, `disabled` and `aria-disabled` behaviour is unchanged. The icon follows `currentColor`, so the „looks disabled but clickable” state greys the icon too: GrayText in the main bar, the existing `opacity: 0.25` in the viewer.
  - Main bar: `className="control-icon-button"`, at the right end of the controls.
  - Viewer (`PhotoViewer.tsx` bar and `ViewerApp.tsx` `EmptyViewer`): it keeps `viewer-close`, plus a square-padding modifier (e.g. `viewer-close--icon` in `PhotoViewer.css`) so the icon button has the height of „Zamknij”. Check that the 4rem gap after „Odrzuć” still reads as „set apart”.
- Icons: the folder and undo SVGs go next to their buttons, in the same style as `DestinationIcon.tsx` (24-unit viewBox, 2px stroke, 18px).

### 6. Tests

- `TimelineView.test.tsx`: the mode tests switch from `button` + `aria-pressed` to `combobox` „Widok” + `selectOptions` / `toHaveValue`. They still check that the view is kept per folder and pushed to the viewer. A new test checks that the folder button (by its name „Wybierz inny folder”) calls `onChooseAnother` and is disabled while busy.
- `UndoButton`: a small test (new `UndoButton.test.tsx`) that the icon look is still the button „Cofnij” with its hint as tooltip, disabled / `aria-disabled` as before. The existing `getByRole('button', { name: 'Cofnij' })` queries in unit and e2e tests (`PhotoViewer.test.tsx`, reject and accept specs) keep working through the `aria-label`. Check for any test that reads the button's text content.
- e2e: `timeline.spec.ts`, `thumbnails.spec.ts` and `persistence.spec.ts` select „Wszystkie zdjęcia” in the „Widok” combobox. Check the other specs that click „Wybierz inny folder” (grep): the name is unchanged, so they should keep working.
- New `ui/e2e/toolbar.spec.ts`, at 900, 1,100 and 1,440 px:
  - no two bar children overlap (bounding boxes);
  - nothing overflows the bar horizontally;
  - path and summary are ≥ 300 px wide;
  - the bar height is the same before and after the „Sprawdzanie zmian…” note.

### 7. Docs

- `docs/domain/timeline.md` (Main window bar): describe the layout (captions above the controls, the „Widok” dropdown, the folder icon button before the path, the second row in narrow windows, the fixed height per width).
- `docs/domain/timeline.md` (Viewer): „Cofnij” is an icon button; also `docs/domain/rejecting.md` if it describes the button's look.
- `docs/slices/23-macos-polish.md`: the main window's buttons are restyled by slice 48; what is left is the viewer bar.
- `docs/slices/44-out-of-focus.md`: the dropdown exists and is built from `VIEW_LABELS`; slice 44 adds a mode and an entry.
- This slice file and the roadmap: 🚧 → 🧪.

### Commits (on `main`, pushed)

1. `docs: plan slice 48` (this plan).
2. `refactor: move the gap slider out of TimelineView`.
3. `feat: compact main window bar for small screens` (steps 2–7, slice 🧪).

## Verification

- `npm run lint`, `npm run typecheck -w ui`, `npm test` and `npm run e2e` (toolbar, timeline, thumbnails, persistence, gap, scan specs).
- Screenshots of the bar at 900 / 1,100 / 1,440 px from the e2e run, to check the look.
- Debug build (`npm run tauri dev`), started for the user: resize the window from full screen down to the minimum and check that the bar never overlaps and the path stays readable. Switch views with the keyboard (Tab to „Widok”, arrow keys), change folder with the icon button, and use a trash destination (red look).

## Related

- [Slice 23](23-macos-polish.md) (macOS polish) notes that „Wybierz inny folder” does not look like a native macOS button; if this slice restyles the bar's buttons anyway, consider doing that part here.

## Implementation notes (2026-11-09)

- The controls measure about 670 px with the bold trash look, so at 1,101 px the path and summary got only 293 px. The breakpoint is therefore **1,120 px** (`TimelineView.css`); `toolbar.spec.ts` checks 900, 1,120, 1,121 and 1,440 px. The bar is 62 px high in one row and 111 px in two.
- The slider stays 8rem.
- In the viewer, „Cofnij” uses `viewer-close viewer-close--icon` and measures the same 30 px as „Zamknij”.
