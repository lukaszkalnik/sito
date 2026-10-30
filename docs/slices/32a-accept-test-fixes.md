# Slice 32a – Fixes from testing slice 32 (accept a photo)

**Status:** ✅ done (tested by the user on 2026-10-30; what it established is in [docs/domain/accepting.md](../domain/accepting.md))

## Context

Asked for by the user on 2026-10-30 while testing [slice 32](32-accept-photo.md) in the debug build:

1. The green check of an accepted photo should sit in the bottom-right corner of its thumbnail, not the top-left. (Bottom-left was tried first and rejected.)

## Done

- `AcceptedBadge.css`: the check is placed at `bottom`/`right`, on grid tiles and series-strip thumbnails alike.
- Grid tiles of accepted photos (`timeline-tile--accepted`) give their caption extra right padding, so a long file name ends in its ellipsis before the check instead of running under it.
- [docs/domain/accepting.md](../domain/accepting.md) says where the check sits.
