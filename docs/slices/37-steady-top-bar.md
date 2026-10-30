# Slice 37 – Steady top bar during background work

**Status:** 📝 draft (concept; detailed plan when the slice starts)

## Context

From the user on 2026-10-30, while testing [slice 33](33-split-by-camera.md): while a scan works in the background, the top bar of the main window grows and shrinks with its status text. The user does not want the bar kept permanently taller just to make room for messages that are only shown for a while.

**Cause:** the bar's status line (`ScanStatus` in `ui/src/features/timeline/TimelineView.tsx`) holds the permanent summary (photos, series, photos without a date, unreadable entries) and the temporary notes. The temporary notes are „Sprawdzanie zmian…” during a background refresh and „Uzupełnianie dat z EXIF: 1234 z 34 600.” during the EXIF pass ([Photos](../domain/photos.md#first-scan-of-a-takeout)). Appending a note makes the line wrap, so the bar grows by a line, then shrinks again when the note goes away. The grid below jumps with it.

## Concept

- **Recommended:** take the temporary notes out of the bar.
  - Show them in a small floating status pill over the bottom of the grid, in the same place as the undo toast (`ui/src/features/reject/Toast.tsx`). The pill takes no layout space and keeps `role="status"`, so screen readers still announce it.
  - The bar then only holds the permanent summary, so its height stays the same.
  - If both the pill and the undo toast are shown, they stack so neither hides the other.
- **Alternatives considered:**
  - Keep the status line on one line with an ellipsis and the full text as a tooltip. The bar stays steady, but the progress can be cut off, which hides the very information the note is for.
  - Reserve room for a second line all the time. The user rejected this: the bar would stay large for nothing.

## Open questions

- Where the pill sits relative to the undo toast (stacked above it, or on the other side).
- Whether the permanent summary itself can still wrap in a narrow window. If so, a steady height in a narrow window also needs an ellipsis there, with the full text as a tooltip.

## Verification (outline)

- During a first Takeout scan with the EXIF pass, and during a background refresh, the top bar and the grid stay in place while the notes appear, update and disappear.
