# Hisaab — Implementation Plan

Hisaab ("account / reckoning" in Hindi/Urdu) is a local-first tracker for **all recurring payments** (subscriptions + bills) for Android, Windows and Linux. Sibling of **Befikr** (my to-do app): same stack, same feel, but a **fully independent app** — copy code from Befikr, share nothing at runtime.

> Befikr repo: https://github.com/oowais/befikr — read it first; copy its scaffold, sync, backup, notifications, theme and CI. Copy the model and structure, not every feature (see **Backlog**).

---

## 1. Product in one paragraph

Existing trackers (Tilla, Wallos, Recurring Expense Tracker, Bobby) store one current price + one next date per subscription. Hisaab treats **a subscription as a history**: a stable identity plus dated **periods** (trial / active / paused), each with its own amount, currency, plan label and interval. Pause, resume, cancel, restart and price/plan changes close one period and open the next. This powers the signature **Timeline view** (one lane per subscription over time) and correct historic totals.

## 2. Decisions (fixed — don't revisit)

| Topic         | Decision                                                                                                                                          |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| App shape     | Independent app, own repo, own DB, own sync. Copy from Befikr, no shared package                                                                  |
| Repo / CI     | Public GitHub repo; GitHub Actions builds Android, Windows, Linux like Befikr                                                                     |
| Stack         | Tauri 2 + Nuxt (SSR off) + Vue + plain SQLite via `rusqlite` (same as Befikr)                                                                     |
| Domain logic  | Rust (`crates/core`): occurrences, invariants, currency. Must run with no webview (notifications, Android receivers via JNI), same rule as Befikr |
| JS tool chain | Vite+ (`vp`): install, check (fmt + lint + types), test, run. Not plain pnpm like Befikr                                                          |
| Updates       | In-app from day one: Android downloads the release APK; Windows uses the signed Tauri updater; Linux via `scripts/install-arch.sh`                |
| Signing       | Keys live in `~/.tauri/` and repo secrets, never committed (public repo)                                                                          |
| License       | GPL-3.0, like Befikr                                                                                                                              |
| Scope         | All recurring payments; `kind` = `subscription` \| `bill`                                                                                         |
| Paid logic    | A charge counts as **paid automatically** on its due date. User edits only exceptions                                                             |
| FX rates      | Public Frankfurter API only (`https://api.frankfurter.dev/v1/`), cached in SQLite. No URL setting                                                 |
| Main currency | Setting, default EUR                                                                                                                              |
| Sync          | Befikr's sync engine copied (LWW + conflict view)                                                                                                 |
| Backup        | Befikr's daily export to Syncthing folder (keep 7) + manual export/import                                                                         |
| UX rule       | Minimal: show controls only when they make sense, never drop features. Undo snackbars instead of confirm dialogs                                  |

## 3. Data model (SQLite)

All ids UUIDv7. All rows: `created_at`, `updated_at`, `deleted_at` (soft-delete tombstones for sync). Money = **integer minor units**; decimals per currency from ISO 4217 (JPY 0, most 2).

```sql
category        (id, name, color, sort)
payment_method  (id, name)                   -- 'Visa ..4417', 'PayPal'

subscription    -- identity; never changes with price
  id, kind,                 -- 'subscription' | 'bill'
  name, template_key,       -- template_key nullable (e.g. 'netflix')
  icon, color,
  category_id, payment_method_id,
  url, notes,
  remind_days_before,       -- nullable = use global default
  archived_at

period          -- contiguous stretch with one set of terms
  id, subscription_id,
  status,                   -- 'trial' | 'active' | 'paused'
  plan_label,               -- nullable: 'Standard', 'Basic', 'v2'
  amount_minor, currency,   -- paused periods: amount 0 allowed
  every_n, unit,            -- unit: 'day' | 'week' | 'month' | 'year'
  anchor_date,              -- first charge date of this period
  start_date, end_date,     -- end_date null = open-ended
  amount_kind               -- 'fixed' | 'estimated'

charge_override -- only when reality differs from the rule
  id, period_id, due_date,
  kind,                     -- 'skipped' | 'moved' | 'amount_changed' | 'settled' (only records what the bank took)
  new_due_date,             -- for 'moved'
  actual_amount_minor, actual_currency,
  settled_amount_minor, settled_currency   -- what the bank actually took

fx_rate         -- local cache, NOT synced
  date, currency, rate_vs_eur, PRIMARY KEY(date, currency)

setting         (key, value)   -- main_currency, default_remind_days, ...
```

### Invariants (validate on save, unit-test)

- Periods of one subscription **never overlap**. Gaps = not subscribed.
- At most one open-ended period per subscription, and it's the latest.
- `end_date >= start_date`.
- Subscription with no open period and last period ended → shown as **Ended**.

### Action → storage mapping

| User action                                               | Writes                                                                           |
| --------------------------------------------------------- | -------------------------------------------------------------------------------- |
| Add                                                       | 1 `subscription` + 1 open period (`active` or `trial`)                           |
| Change price / plan "from <date>" (default: next charge)  | Close current period at date−1; new period with new amount/plan, same anchor day |
| Pause (optional "resume on")                              | Close active; open `paused` (end = resume date or null)                          |
| Resume                                                    | Close paused; new `active` period, anchor = resume date (user can adjust)        |
| Cancel ("ends on", default: end of current billing cycle) | Set `end_date` on last period                                                    |
| Restart ended                                             | New `active` period after the gap                                                |
| Trial → paid                                              | Close `trial`; open `active`                                                     |
| Future announced price rise                               | Future-dated period (shows lighter on timeline, used in forecasts)               |
| One charge differed / skipped / moved                     | `charge_override`                                                                |

## 4. Occurrence engine (pure Rust in `crates/core`, heavily tested)

`occurrences(subscription, periods, overrides, from, to) → Charge[]`

- Charges are **derived**, never stored (except overrides).
- For each non-paused period: start at `anchor_date`, step by `every_n unit` within `[start_date, end_date]`.
- **Day clamping:** keep the anchor's day; clamp per month. Anchor Jan 31 monthly → Feb 28/29 → Mar 31. Compute each occurrence from the anchor (`anchor + k*interval`), never from the previous occurrence (prevents drift).
- Apply overrides (skip, move, amount change).
- Charge status: `upcoming` if due > today, else `paid` (auto-paid rule).
- Return: `{ subscriptionId, periodId, dueDate, amountMinor, currency, settled?, status, isOverride }`.

Required tests: month-end clamping, leap years, weekly/yearly, every 3 months, pause in middle, overlapping-period rejection, override skip/move/amount, ranges crossing period boundaries, zero-decimal currency.

## 5. Currency

- `fx_rate` filled from Frankfurter: latest once/day when online (`/v1/latest?base=EUR`); historic ranges lazily when stats need them (`/v1/2025-01-01..2025-12-31?base=EUR`).
- Convert A→B via EUR. Weekend/holiday → previous available business day.
- Rules:
  - Past charge with `settled_amount` → use it.
  - Past charge otherwise → rate on due date.
  - Upcoming / forecast / headers → latest cached rate.
  - Currency not in ECB set → show original only, exclude from totals with a small warning (MVP).
- Never store converted values. Show "rates from <date>" when stale (>3 days).
- Rows always show original currency; converted value muted beside it when different: `$9.99 · €9.21`.

## 6. Screens

Android: bottom nav with 3 tabs (Upcoming · Timeline · Stats). Desktop: sidebar + list/detail split. Settings via icon.

### 6.1 Upcoming (home)

- Header: **€87.40 / month** in main currency; tap cycles month → year → day.
- Groups: Today · This week · Later this month · Next month. Row = color/icon, name, relative date ("in 3 days"), amount (original + converted).
- Paused collapsed into one "N paused" row. Ended hidden.
- Swipe left = snooze/move this charge (date picker). Long-press / right-click = Pause · Change price · Cancel.
- Filter chip: All · Subscriptions · Bills.

### 6.2 Quick add (bottom sheet / dialog)

- Single search field over templates (icon + name). No match = custom.
- Then: amount + currency on one row (currency default = template's or last used).
- Sentence "**monthly**, starting **today**" — tap words to expand interval / date pickers.
- "More" reveals: kind, category, payment method, plan label, trial toggle (+ trial end), reminder days, URL, notes.

### 6.3 Detail

- Card: name, current plan, price, next charge, status chip (Trial / Active / Paused / Ended).
- Buttons depend on status only:
  - Trial → Convert to paid · Cancel
  - Active → Pause · Change price · Cancel
  - Paused → Resume · Cancel
  - Ended → Restart · Archive
- "Change price" asks only: new amount (+ currency), from when, optional plan label.
- Mini timeline strip, period list (`Premium · €12.99 · Mar 2024 – Jan 2025`), "Total spent: €412", charge history with overrides editable.

### 6.4 Timeline (signature screen — hand-built SVG Vue component)

- One lane per subscription; x = time. Zoom: 6 months / 1 year / all; pinch (Android), ctrl+scroll (desktop).
- Bar style by status: active = filled (subscription color), trial = light fill + dashed border, paused = dashed outline only.
- Diamond marker at every price/plan change; tap → "Standard €13.99 → Basic €4.99".
- Dashed "today" line; future periods lighter.
- Sort: start date · current cost · category. Toggle show ended.
- Tap bar → that period in Detail.

### 6.5 Stats (v1.1)

- Monthly spend over time, stacked by category (uses periods → historically correct).
- Breakdown by category, payment method, currency; cost per day; "saved by pausing/cancelling".

### 6.6 Settings

- Main currency, default reminder days, reminder time, theme, sync (Befikr UI), backup/export/import, template management.

## 7. Notifications

Copy Befikr's scheduler.

- N days before charge (per-subscription override) at configured time: "Netflix · €4.99 in 3 days".
- Trial ending: 2 days before trial end.
- Actions: **Open** · **Snooze** (date/time picker, like Befikr).
- Reschedule on any period/override change and on app start.

## 8. Templates

Bundled JSON: `{ key, name, kind, icon, color, defaultCurrency, defaultInterval }`.
Start with ~40: streaming, music, cloud (Proton, iCloud, Google One), dev tools, Claude, phone plans, Deutschlandticket, rent, Haftpflicht, Hausrat, Rundfunkbeitrag, gym.
Icons: bundle [Simple Icons](https://simpleicons.org) SVGs for brands (offline); generic icons for bills; custom upload allowed.

## 9. Sync & backup

- Copy Befikr's engine. Synced tables: everything except `fx_rate`.
- Conflict special case: two devices both change price offline → overlapping periods. Apply LWW, then if overlap detected, surface in the conflict view rather than auto-merge. Befikr's conflicts are per field; overlaps get their own row-level "overlap" entry in the same view.
- Daily export to Syncthing folder (keep 7), manual export/import, same as Befikr (SQLite snapshots, `.db`).

## 10. Milestones

1. **Scaffold** — repo, copy Befikr's Tauri/Nuxt setup, theme, CI (Android/Windows/Linux builds), lint, tests. App boots on all three.
2. **Domain core** — schema + migrations, repositories, occurrence engine, invariants, currency conversion. Full unit tests. No UI beyond debug.
3. **MVP UI** — Upcoming, Quick add + templates, Detail with status actions, Settings basics. FX fetch + cache.
4. **Timeline** view.
5. **Notifications** (copy from Befikr, adapt).
6. **Sync + backup** (copy from Befikr, adapt tables).
7. **v1.1** — Stats, calendar month view, charge overrides UI, archive, variable bills, Android widget.
8. **Later** — shared/split cost, budget cap, CSV/Wallos import, trial countdown with cancel link.

Each milestone: working build, tests green, short note in `CHANGELOG.md`.

### Build order and status

| #   | Milestone                      | Status                                                                                        |
| --- | ------------------------------ | --------------------------------------------------------------------------------------------- |
| 1   | Scaffold + CI + in-app updates | Done (Nov 1, 2026): released 0.1.1, Android update verified; Windows and Linux still to check |
| 2   | Domain core                    | Done (Nov 1, 2026): `crates/core`, 34 tests                                                   |
| 3   | MVP UI                         | Done (Nov 1, 2026)                                                                            |
| 4   | Timeline                       | Done (Nov 1, 2026)                                                                            |
| 5   | Notifications                  | Done (Nov 1, 2026): needs a release to check on Android                                       |
| 6   | Sync + backup                  | Done (Nov 1, 2026): needs a release to check pairing and background sync on Android           |
| 7   | v1.1                           | Done (Nov 1, 2026): widget needs a release to check on Android                                |
| 8   | Later                          | Next                                                                                          |

Milestone 1 checklist:

- [x] Vite+ Nuxt app with Befikr's Glass CSS, splash, theme, toast, What's new sheet
- [x] Tab bar (Upcoming · Timeline), Settings (theme, version, check for updates, update)
- [x] Rust workspace: `hisaab-core` (Store, migrations, settings) + tests; Tauri app; Android plugin (updater, back button)
- [x] App icon (`src-tauri/android/icon/gen.py`) for Android and desktop
- [x] CI: `release.yml` (APK + .deb/PKGBUILD + NSIS + latest.json on push to main), `core.yml`, `web.yml`, Dependabot
- [x] Linux install/update script (`scripts/install-arch.sh`)
- [x] Set repo secrets, first push, install the APK from the release, confirm an in-app update on Android (0.1.0 → 0.1.1, Nov 1, 2026)
- [ ] Confirm the in-app update on Windows and `scripts/install-arch.sh` on Linux

Milestone 2 checklist:

- [x] Migration 2: category, payment_method, subscription, period, charge_override, fx_rate (no foreign keys: sync may deliver rows out of order)
- [x] Occurrence engine (`occurrence.rs`): anchor-based steps with month-end clamping, overrides, paid/upcoming
- [x] Invariants and shown status (`invariants.rs`)
- [x] Repositories and actions (`subscriptions.rs`): add (with trial), change terms, pause/resume, cancel, restart, convert trial, overrides (re-homed on splits), archive, delete/restore, categories, payment methods
- [x] Currency (`currency.rs`, `fx.rs`): ISO 4217 codes and minor units, conversion via EUR, rate cache with previous-business-day lookup, valuation rules, staleness
- [x] Tests: everything in §4's list, plus actions and currency

Decisions made in milestone 2:

- Dates are calendar days (`YYYY-MM-DD`), not instants.
- Free trials (amount 0) produce no charges; a trial with an end date is followed by the paid period from the next day, so it converts by itself unless cancelled.
- A paused period keeps its terms (resume copies them); it produces no charges.
- An open-ended pause drops later scheduled changes (e.g. a future price rise); a pause with a resume date keeps them.
- Shown status: the period covering today; else the next scheduled one; else Ended.

Milestone 3 checklist:

- [x] Tauri commands over the core (`src-tauri/src/lib.rs`), overview layer in core (`overview.rs`: status, next charge, yearly cost, upcoming, history, total spent)
- [x] FX: latest rates once a day when online, historic ranges when a history needs them (`src-tauri/src/fx.rs`, `ureq`)
- [x] Upcoming: total per month / year / day (tap), groups, filter chips (only with both kinds), swipe left to move a charge, long-press menu, paused collapsed, ended hidden
- [x] Quick add: template search (54 templates, Simple Icons + Lucide fallbacks for brands Simple Icons dropped), amount + currency, tappable "monthly, starting today", More
- [x] Detail: status card, actions by status (sheets with sensible default dates), mini timeline strip, periods, total spent, charge history (undo an override), edit, archive, delete with undo
- [x] Settings: main currency, default reminder days
- [x] Icon and colour picker (Quick add, Edit): 57 brand glyphs, 41 generic icons, 12 swatches
- [x] Desktop list/detail split (Nov 1, 2026): from 1100px wide, Upcoming with the open payment beside it (`/?sub=<id>`; `/sub/<id>` redirects there), tabs as a left rail
- [x] Categories and payment methods in Settings → Data (Nov 1, 2026): rename, colour, order, delete with undo (links restored)
- [ ] Managing templates in Settings: skipped for now (Nov 1, 2026), by choice

Milestone 4 checklist:

- [x] `timeline` command (core `TimelineLane`: view + all periods, archived left out)
- [x] Hand-built SVG chart (`TimelineChart.vue`): lanes, month / quarter / year axis, status styles, future periods lighter, dashed today line, names that stay readable while scrolling
- [x] Zoom: 6 months / 1 year / All, pinch, ctrl+scroll (keeps the point under the fingers / pointer)
- [x] Price / plan change diamonds (tap: "Standard €13.99 → Basic €4.99"), sort by start / cost / category, show ended
- [x] Tap a bar → Detail with that period highlighted

Milestone 5 checklist:

- [x] Core reminders (`reminders.rs`): computed like charges; one alarm per subscription (its next reminder); N days before a charge at the reminder time, trial ending 2 days before (wins a tie with the first charge); `reminder_state` (this device, not synced) so nothing fires twice and nothing from the past fires on first sight; snooze; quick picks
- [x] Android (Befikr's, adapted): exact alarms, notification with Open · Snooze, snooze overlay ("Pick a day" opens the app), boot / update / time-change restore, JNI bridge (`src-tauri/src/jni.rs`)
- [x] Desktop: scheduler thread, notifications with Open · Snooze, tray (window close hides), start at login + start hidden
- [x] Re-armed on start and after every change; reminder time setting; Android permission rows in Settings; in-app snooze sheet (`/sub/<id>?snooze=1`)
- [ ] Check on a phone: Kotlin only compiles in CI (no local `javac`); exact alarm + notification + overlay on a real device; Windows toast actions

Milestone 6 checklist:

- [x] Core (`sync.rs`, `conflicts.rs`, `peers.rs`, Befikr's): change-log triggers on every synced table (not `fx_rate`, `reminder_state`), HLC, per-field newest-wins merge, conflicts for this device's lost edits, backfill of data from before sync
- [x] Overlapping periods after a merge (price changed on two devices): one `overlap` conflict per subscription; keeping one drops the other and closes the period before it
- [x] `crates/sync` (iroh, Befikr's): pairing by `hisaab://pair?…` link / QR, change exchange, own ALPN; two-device test
- [x] App: sync commands, `data-changed` / `sync-changed` events, nudge after writes; Android background sync (WorkManager, JNI `syncOnce`) re-arms reminders
- [x] Backups as SQLite snapshots (`.db`, not JSON as first planned: same as Befikr, restores everything): daily to a folder (keep 7), export, restore (keeps this device's sync id, peers and shown reminders); desktop in Rust, Android in Kotlin (SAF)
- [x] UI: Sync (on/off, name, devices, QR / scan / paste link), Conflicts, Backup; sync dot and stale hint on Upcoming; Settings → Data
- [ ] Check on real devices: pairing phone ↔ computer, QR scan (camera), background sync, Android backup folder (Kotlin only compiles in CI)

Milestone 7 checklist:

- [x] Charge overrides UI: `ChargeSheet` from Detail (new Coming list + history) and the Upcoming menu: amount taken (other currency = settled), a bill's real amount, another date, skip, back to the usual; an amount override keeps a moved date
- [x] Variable bills: a charge without its real figure costs the average of the last 3 real ones (`occurrence.rs`); "Amount varies" in Change price; history asks for the real amount
- [x] Ended and archived screen (`/archive`, core `archived`), from Upcoming and Settings → Data
- [x] Stats tab (core `stats.rs`, hand-built SVG, no chart library): monthly spend stacked by category (12 months / all, this and next month as forecast), yearly cost by category / payment method / currency, per day, saved by pausing and cancelling
- [x] Calendar (`/calendar`, core `month_charges`): month grid with each day's payments, month total, day list with move / change
- [x] Android widget (`PaymentsWidget.kt`, JNI `widget`): monthly cost, next charges, + opens Quick add; refreshed on every arm and after background sync
- [ ] Check on a phone: the widget (Kotlin only compiles in CI)

Milestone 8 so far:

- [x] Trial countdown (Nov 1, 2026): "Free trials" on Upcoming (soonest first), a trial card in Detail (days left, what follows, Cancel at <site>); 15 templates carry their cancel page; links open in the browser (`open_url`: `xdg-open` / `explorer` on desktop, an intent on Android; before, Detail's link did nothing)
- [x] Monthly budget (Nov 1, 2026): set in Settings → Money (synced, main currency; follows a main-currency change at the latest rates); bar on Upcoming (orange from 90%, red over), a note in Quick add when a new payment goes over, a dashed line and "over in N months" on Stats
- [x] Shared payments (Nov 1, 2026): `subscription.my_share` (percent, synced; migration 5), set in Quick add's More and Edit (Just me / Half / A third / A quarter / other %); the charge stays whole, totals, Stats, Calendar, total spent and the budget count your part; rows show "yours €5.00"
- [x] Search (Nov 1, 2026, from the Backlog; Befikr's): `/search` from Upcoming's header, `/` or Ctrl+F on computers; name, notes, link, plans, category and payment method, every word must appear; running first, then ended and archived (core `search.rs`)
- [x] Share to Hisaab (Nov 1, 2026, from the Backlog; Befikr's): Android share sheet → Quick add on step 2 with what the text gives (`app/utils/share.ts`): a template by the link's site or a name in the text, the amount and currency, the link; else the subject, first line or site as the name. Needs a release to check on a phone (Kotlin only compiles in CI)

- [ ] CSV/Wallos import: skipped for now (Nov 1, 2026), by choice

**Next:** pick from the Backlog below; your feedback from the Linux install when you have it. Still open from earlier milestones: checks on real devices (Windows update, Linux install, Android reminders, sync, widget).

### Backlog (Befikr features not copied yet)

Morning digest, voice input, project page (`pages.yml`), AUR `hisaab-bin`.

## 11. Working agreements for Claude Code

- Read Befikr first; mirror its folder structure, naming, conventions and component patterns.
- Commit straight to local `main` (Conventional Commits); never push — the user pushes, and a push releases.
- Small commits per step; don't start the next milestone until the current one builds and tests pass.
- Ask before adding dependencies not already used in Befikr (except a chart lib in Stats).
- Keep the UI minimal and consistent with Befikr; when unsure about a UX detail, ask rather than invent.
