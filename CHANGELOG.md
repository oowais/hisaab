# Changelog

One short note per milestone. Per-release notes come from the commits (`.github/scripts/release-notes.sh`).

## Milestone 4 — Timeline (Nov 1, 2026)

- Timeline: one lane per subscription over time, with trials, pauses and future changes.
- Diamonds at price and plan changes; tap for what changed.
- Zoom 6 months / 1 year / all, pinch or ctrl+scroll; sort by start, cost or category; show ended ones.

## Milestone 3 — MVP UI (Nov 1, 2026)

- Upcoming: what's charged next, grouped by when, with the total per month, year or day in your main currency.
- Quick add with 54 templates (brand icons offline), any currency, any interval, free trials.
- Detail: change price, pause, resume, cancel, restart, convert a trial; price history and total spent.
- Exchange rates from the ECB (Frankfurter), cached; past charges use the rate of their day.
- Settings: main currency, default reminder days.

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
