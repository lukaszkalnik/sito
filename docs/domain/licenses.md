# Licenses

Sito is licensed MIT OR Apache-2.0. The app contains compiled copies of Rust crates and npm packages whose licenses require their notices to ship with every copy.

## In the app

- The menu item **„Pomoc → Licencje…”** (on macOS in the Help menu of the app menu) opens a dialog in the main window, over the start screen or the timeline. The viewer window has no menu on Windows.
- The dialog shows the app version, says that Sito is licensed MIT or Apache 2.0, and shows one scrollable plain text:
  1. Sito's own license texts (MIT, Apache 2.0),
  2. the Rust crates compiled into the app, grouped by license text, each with its version and crates.io link (where its source code is available, as MPL-2.0 asks),
  3. the npm packages bundled into the UI, each with its version, license, npm link and license file.
- „Zamknij” or Escape closes it. If the text cannot be loaded, it says so; opening it again retries.

## What is included

- Only what ships: Rust dependencies for Windows and macOS without build and dev dependencies, and exactly the npm packages in the UI bundle.
- Accepted licenses are a reviewed list. Rust: MIT, Apache-2.0, Unicode-3.0, BSD-3-Clause, Zlib, MPL-2.0. npm: MIT, ISC, Apache-2.0. A dependency with another license fails CI until the license is reviewed and added.
