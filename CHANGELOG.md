# Changelog

One short note per milestone. Per-release notes come from the commits (`.github/scripts/release-notes.sh`).

## Milestone 7 — v1.1 (Nov 1, 2026)

- Stats: spend per month by category (past prices stay right), what you pay a year by category, payment method and currency, cost per day, and what pausing and cancelling saved.
- Calendar: a month at a glance with what's charged each day and the month's total.
- Edit a single charge: what was really taken (also in another currency), a bill's real amount, another date, or skip it.
- Variable bills learn their amount: coming ones use the average of the last three real ones.
- Ended and archived: finished payments in one place; archive or bring back.
- Android home-screen widget: monthly cost and the next charges.

## Milestone 6 — Sync and backup (Nov 1, 2026)

- Sync between your devices, peer to peer (Befikr's engine): pair by QR code or link, end-to-end encrypted, no account or server of ours.
- Newest change wins per field; your overwritten edits show on the Conflicts page to keep or drop. A price changed on two devices before they synced shows as one card: keep one of the two changes.
- Android syncs in the background (about every 15 minutes) so reminders follow changes made elsewhere.
- Daily backup to a folder you pick (keeps 7), export a copy, restore from a backup.

## Milestone 5 — Notifications (Nov 1, 2026)

- Reminders before each charge ("Netflix · €4.99 in 3 days"), at a time you choose; per payment or a default number of days.
- A heads-up 2 days before a free trial ends.
- Snooze from the notification (quick picks, or any day); Open goes straight to the payment.
- Desktop: runs in the tray with the window closed; start at login.

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
