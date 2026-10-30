# Idea: side-by-side comparison

Status: idea, left out of [slice 13](../slices/13-viewer-zoom.md) (zoom in the viewer) on 2026-10-30.

- Show two (or more) photos of a series next to each other, zoomed to the same spot and level, so sharpness and faces can be compared without stepping back and forth.
- Panning or zooming one photo moves the others along.
- Can build on the viewer's zoom (`ui/src/features/viewer/zoom.ts`): the zoom is stored relative to the photo, so the same `Zoom` value can be drawn for each photo with its own `Geometry`.
