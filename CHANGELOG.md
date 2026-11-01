# Changelog

One short note per milestone. Per-release notes come from the commits (`.github/scripts/release-notes.sh`).

## Milestone 1 — Scaffold (Nov 1, 2026)

- Tauri 2 + Nuxt app for Android, Windows and Linux, built with Vite+; look and structure copied from Befikr.
- Upcoming and Timeline tabs (empty for now), Settings with theme and updates.
- In-app updates: Android installs the new APK, Windows the signed installer; Linux updates with `scripts/install-arch.sh`.
- CI builds and releases all three on every push to `main`; PRs run Rust and web checks.
- `hisaab-core` crate with the SQLite store, migrations and settings, with tests.
