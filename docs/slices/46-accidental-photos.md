---
status: 📝 concept drafted
---

# Slice 46 – Mark accidental photos

Since 2026-11-07; plan in plan mode when the slice starts.

## Idea

Asked by the user on 2026-11-07: Sito marks photos taken by accident, e.g. while walking, in a pocket or bag, with the phone lying on a table, or with a finger over the lens, so the user can find and reject them quickly.

A per-photo mark („Przypadkowe”) like [slice 44](44-out-of-focus.md) and [slice 45](45-bad-exposure.md). The app only marks; the user decides.

## Typical accidental photos

- **In a pocket or bag:** almost black or a uniform dark smear of fabric, little detail.
- **Lying on a table:** the ceiling or a lamp, or the table surface close up; often a plain, low-detail surface.
- **While walking:** the ground, feet or a tilted, motion-blurred street.
- **Finger over the lens:** a large blurry, often reddish or dark blob over a corner or an edge.
- Often single shots or short accidental bursts taken while the phone is put away.

## Candidate approaches

1. **Combine simple signals.** Low detail and low colour variety over the whole photo (entropy, edge density), together with slice 44's blur and slice 45's exposure measures. Cheap and needs no model, but a plain sky or a white wall would be marked too.
2. **Image embeddings.** With the embedding model of [slice 19](19-split-by-content.md): if it is a CLIP-like model, compare each photo with text prompts („the inside of a pocket”, „a blurry photo of the floor”, „a finger covering the lens”, „a ceiling lamp”) and mark photos close to them; otherwise train a small classifier on top of the embeddings with labelled examples.
3. **A dedicated small classifier** (ONNX) for accidental shots, if a suitable model with a fitting license exists.

## To check when planning

- Collect real examples from the user's galleries (they stay on the machine) to tune and test the measure; add a few synthetic ones to `fixtures/` for the integration tests.
- Whether EXIF helps: orientation and focus distance, very short exposure series, phone models only.
- Whether this slice should wait for slice 19's model (then it comes after slice 19 in the roadmap) or start with approach 1.

## Builds on

- [Slice 44](44-out-of-focus.md): the mark kinds, the background analysis pass and cache, the badge, the marked-only view in the „Widok” dropdown and dismissing.
- [Slice 45](45-bad-exposure.md): the exposure measures, as one signal of an accidental photo.
- [Slice 19](19-split-by-content.md): the image embeddings, if approach 2 is chosen.

## Try

Open a folder with known pocket, table and walking shots: are they marked, and are plain skies and walls left alone?
