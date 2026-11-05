# quota-line

A Claude Code mod that keeps a status line under the prompt with the account's quota usage per rate-limit window (5-hour, 7-day) and how long until each window resets, for example:

```
5h 42.5% · resets in 2h 10m  |  7d 18% · resets in 3d 1h
```

Once a window is used up (100%), the line switches to the credit spent since that moment and when the quota comes back:

```
⛔ 5h quota used up · credit used $2.25 · resets in 45m
```

The countdown refreshes every minute. Off a subscription (no rate-limit readings) the line stays hidden.

## Install

It is enabled for everyone working in this repository by `.claude/settings.json` (marketplace `sito-mods` at the repository root). To install it for all your projects instead:

```
/plugin install quota-line --marketplace lukaszkalnik/sito
```
