---
status: 🧪 ready for testing
---

# Slice 61 – Gap as a number input

Asked for by the user on 2026-11-20 as a small UI improvement; implemented the same day, waiting for the user's test. What it established is in [grouping](../domain/grouping.md) and [timeline](../domain/timeline.md).

## Idea

„Odstęp w serii” in the main window bar was a slider of [slice 6](06-time-gap-slider.md) (1–60 s) with − and + and the value as text. A slider is hard to set to an exact value; a number input shows and takes the seconds directly.

## Try

Type a gap, step it with − and + or the arrow keys, type something out of range (e.g. 90 or 0) and leave the field, clear the field and leave it; the series regroup and the gap is remembered for the folder as before.

## Plan (done)

- `GapSlider` becomes `GapInput` (`ui/src/features/timeline/GapInput.tsx`): `[−] [10] s [+]`, a `type="number"` input (1–60, step 1) named by the „Odstęp w serii” caption; the browser's own spinner is hidden, since − and + step it.
- Typing: a whole number within the range applies at once (`typedGap` in `gapSetting.ts`); an empty, partial or out-of-range value waits, and leaving the field clamps it to the range or restores the gap when the field is empty (`finishedGap`). A step with − or + replaces text still being typed.
- Tests: `gapSetting.test.ts` (both functions), `GapInput.test.tsx` (typing, clamping, empty field, buttons after typing), `TimelineView.test.tsx` and `ui/e2e/gap.spec.ts` (regrouping, arrow keys, clamping, remembering per folder).
