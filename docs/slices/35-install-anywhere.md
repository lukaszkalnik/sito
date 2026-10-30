# Slice 35 – Install anywhere

**Status:** 📝 concept drafted (2026-10-30; plan in plan mode when the slice starts)

## Idea

Users can install Sito into any folder they choose, including protected ones such as `Program Files` (on any drive).

Found on 2026-10-30 while installing v0.13.0: choosing `D:\Program Files\sito` on the installer's folder page failed with an error writing `sito.exe`. The installer is per-user only (`installMode: "currentUser"` in `src-tauri/tauri.conf.json`), runs without admin rights, and ordinary users may only read `Program Files` folders. The error says nothing about why.

## Candidate approaches

1. **Let the user choose „tylko dla mnie” or „dla wszystkich użytkowników”** (`installMode: "both"`). Our template `src-tauri/installer/installer.nsi` already handles it (NSIS MultiUser). „Dla wszystkich” asks for admin rights (UAC) and then any folder works, `Program Files` included; „tylko dla mnie” stays as today (no admin, `%LOCALAPPDATA%\Sito` by default). Probably the core of the slice.
2. **Check the chosen folder before installing:** if it is not writable in the chosen mode, say so in Polish on the folder page („Brak uprawnień do zapisu w tym folderze – wybierz inny albo zainstaluj dla wszystkich użytkowników”) instead of failing halfway with a raw write error. Needs a small NSIS check in the template (`Polish.nsh` for the text).
3. **Moving an existing installation:** when the folder changes and „Nie odinstalowuj” is kept, the old copy stays behind. Either default to uninstalling the old version when the folder differs, or say so on that page.

## To check when planning

- Per-machine installs write the uninstall entry to HKLM and need admin to update or uninstall; how updates (running a newer installer) behave in each mode, and switching from a per-user to a per-machine install.
- The smoke test in `.github/workflows/release.yml` installs silently with `/S` into `%LOCALAPPDATA%\Sito`; it must keep working (and ideally also cover the per-machine mode, `/ALLUSERS`).
- App data (action log, caches, window state) lives in the user profile in both modes, so nothing else moves.
- `docs/RELEASING.md` (installing on Windows) and `docs/domain/` describe the new choice.

## Try

Install into `D:\Program Files\Sito` „dla wszystkich użytkowników” (UAC prompt), into a folder under `D:\Lukasz` „tylko dla mnie”, and pick a protected folder „tylko dla mnie” to see the clear message; update an existing install in each mode.
