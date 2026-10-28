# Domain

What Sito works on and how it finds photo series and picks the best shot. The rules for writing code are in [CLAUDE.md](../CLAUDE.md); what gets built when is in the [roadmap](ROADMAP.md).

## The problem

Sito works on large photo galleries taken with Android phones. It finds **series of similar shots**: photos taken seconds apart that differ in exposure, slightly in angle, or in facial expression. This is **not** primarily about byte-identical duplicates. There are **no** burst or motion photos.

## Pipeline (`crates/core`)

1. **Scanner:** walk folders recursively, read EXIF (capture time, orientation, camera). Fallback without EXIF: file modification time.
2. **Grouping:**
   - Sort photos by capture time. A new group starts when the gap exceeds a threshold (default ~5–10 s, adjustable in the UI).
   - Then split groups by embedding similarity (cosine) when they contain different subjects.
3. **Analysis** (parallel; decode downscaled early; respect EXIF orientation):
   - Image embedding via a small ONNX model (e.g. DINOv2-small or CLIP)
   - Sharpness: variance of the Laplacian
   - Exposure: share of clipped highlights and crushed shadows in the histogram
   - Faces and eye state (open/closed) via a small model (e.g. YuNet + landmarks)
4. **Scoring:** weighted score from sharpness, exposure and faces; suggest the best photo per group. Thresholds and score weights are settings, tuned in the app itself.
5. **Storage:** SQLite keyed by path + size + mtime. Stores embeddings, scores, groups and decisions so rescans are fast.
6. **Actions:** move rejected photos to quarantine, with an undo log.

## Backend interface

The Tauri layer exposes commands (e.g. `start_scan`, `get_groups`, `set_decision`, `apply_decisions`, `undo`) and emits progress events. Thumbnails and full photos are served by Rust through the `thumb://` and `photo://` protocols, thumbnails from a thumbnail cache.
