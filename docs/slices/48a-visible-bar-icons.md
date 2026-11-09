---
status: 🧪 ready for testing
---

# Slice 48a – Better visible controls in the main window bar

Fix from testing [slice 48](48-compact-toolbar.md), 2026-11-09; next: testing by the user.

## Problem

Testing slice 48 on the MacBook (2026-11-09), the user found the bar's icon buttons (folder, „Cofnij”, gap −/+) hard to see: dark grey icons inside a very light 1 px border (`--border`, meant for separators).

## Fix

- New colour token `--control-border` (`ui/src/index.css`, light and dark), darker than `--border`; buttons and dropdowns of the bar get a 1.5 px border in it (`ui/src/controls/controls.css`).
- Icon buttons (`.control-icon-button`) draw their icon in the accent colour. Disabled ones stay faded, and „Cofnij” with only trash moves left stays grey (`GrayText`).
- Docs: [Timeline](../domain/timeline.md#main-window-bar).
