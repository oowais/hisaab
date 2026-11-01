# Changelog

One short note per milestone. Per-release notes come from the commits (`.github/scripts/release-notes.sh`).

## Milestone 2 — Domain core (Nov 1, 2026)

- Schema for categories, payment methods, subscriptions, periods, charge overrides and the FX cache.
- Occurrence engine: charges derived from periods (month-end clamping, leap years, any interval), with skip / move / amount / settled overrides.
- Subscription actions that keep the period history: change price, pause, resume, cancel, restart, convert a trial.
- Currency: ISO 4217 minor units, conversion via EUR, cached rates with the previous business day on weekends.
- 34 tests.

## Milestone 1 — Scaffold (Nov 1, 2026)

- Tauri 2 + Nuxt app for Android, Windows and Linux, built with Vite+; look and structure copied from Befikr.
- Upcoming and Timeline tabs (empty for now), Settings with theme and updates.
- In-app updates: Android installs the new APK, Windows the signed installer; Linux updates with `scripts/install-arch.sh`.
- CI builds and releases all three on every push to `main`; PRs run Rust and web checks.
- `hisaab-core` crate with the SQLite store, migrations and settings, with tests.
