# Slice 34a – Fixes from testing slice 34 (reject into the trash)

**Status:** 🧪 ready for testing (implemented on 2026-10-30)

## Context

Asked for by the user on 2026-10-30 while testing [slice 34](34-reject-to-trash.md) in the debug build:

1. The „Odrzuć” button in the viewer should show where rejected photos go, the Recycle Bin or the „Odrzucone” folder, with an icon. Today only its tooltip says so.

## Plan

### 1. `feat(ui): show where „Odrzuć” moves the photo with an icon`
- `ui/src/features/viewer/PhotoViewer.tsx`: the button shows the destination's icon (`reject/DestinationIcon.tsx`, the folder or trash outline already used by the picker in the main window) before „Odrzuć”. The icon is `aria-hidden`; the button keeps its name „Odrzuć”, and its tooltip still names the destination.
- `PhotoViewer` takes the destination (`rejectTo: RejectDestination`) instead of a ready-made hint, and picks the hint itself; `ViewerApp.tsx` passes it.
- `PhotoViewer.css`: icon and text side by side, the icon in the button's white text colour.
- Tests: the button shows the folder icon by default and the trash icon once the trash is chosen, and changes when the choice changes in the main window.

### 2. `docs: slice 34a ready for testing`
- `docs/domain/rejecting.md`, slice and roadmap → 🧪.

Checks: `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`; then a look in the debug build.

## Verification (2026-10-30)

Implemented as planned. In the debug build (computer use): „Odrzuć” shows the folder icon in white before its text with the folder chosen. The trash icon and the live switch are covered by `ViewerApp.test.tsx`. Checks green (oxlint, Prettier, tsc, Vitest 229, Playwright viewer and reject specs).

**Next:** the user tests the debug build, together with slices 13 and 34.
