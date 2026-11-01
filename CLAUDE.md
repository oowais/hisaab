# CLAUDE.md

Hisaab: private, local-first tracker for all recurring payments (subscriptions + bills), with each subscription stored as a history of priced periods. Android, Windows, Linux. Sibling of Befikr (https://github.com/oowais/befikr): same stack and feel, fully independent app; code is copied from Befikr, nothing is shared at runtime.

## Start every session here

1. Read `PLAN.md` → **Build order and status** (table + checklist + **Next:** line). That is the source of truth for what's done and what's next.
2. Work on the next unfinished step unless the user says otherwise. Don't start a milestone until the current one builds and its tests pass.
3. Before ending a milestone: update the status table, checklist and **Next:** line in `PLAN.md`, and add a short note to `CHANGELOG.md`. Commit them with the work.
4. When copying something from Befikr, read Befikr's version first and keep its structure and naming.

## Stack

Tauri 2 · Nuxt (`ssr: false`, SPA) · SQLite via `rusqlite` (bundled) · Vite+ (`vp`) for the JS tool chain · native Kotlin for Android where Tauri plugins fall short.

## Layout

- `app/` — Nuxt SPA. Design system: Glass (`app/assets/main.css`, from Befikr) — use its building blocks (`.pane`, `.popover`, `.btn`, `.chip`, `.field`, `.checkbox`, `.icon-btn`) and Lucide icons (`lucide-vue-next`), no emoji/unicode glyphs as icons.
- `crates/core/` — `hisaab-core`: storage, occurrence engine, invariants, currency (pure Rust, no Tauri).
- `src-tauri/` — Tauri app: commands wrapping core; desktop code in `src-tauri/src/desktop/` (`#[cfg(desktop)]`).
- `plugins/tauri-plugin-reminders/` — Kotlin side (in-app update, back button; reminders from milestone 5).
- `test/` — TS unit tests (`vp test`, helpers from `vite-plus/test`).

## Non-negotiable rules

- All DB writes go through the Rust repository layer (single place for sync change events).
- UUIDv7 primary keys; `updated_at` on every synced row; soft delete (`deleted_at`), never hard delete.
- Money as integer minor units; never store converted amounts.
- Occurrence, scheduling and currency logic in Rust (must run with no webview, e.g. from a notification).
- UX: show controls only when context needs them; undo toasts instead of confirm dialogs; every action instant.
- Dates are picked only with our own sheets (copy Befikr's `DateTimeSheet`); never native pickers or `<input type="date">`.
- Ask before adding a dependency Befikr doesn't use (agreed so far: `ureq` for FX rates, Simple Icons SVGs, a chart lib for Stats).

## Tool chain

- JS: `vp install`, `vp add`, `vp check` (`--fix`), `vp test`, `vp run generate`, `vp run typecheck` (Vue files: `vp check` doesn't type check them). Never npm/npx/bun, ESLint or Prettier. Config in `vite.config.ts`.
- Rust: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (needs `.output/public` to exist: `mkdir -p .output/public` or `vp run generate`).
- Desktop app: `vp run tauri dev`; local binary `vp run tauri build --debug --no-bundle` → `target/debug/hisaab`. Test with `XDG_DATA_HOME`/`XDG_CONFIG_HOME` pointed at a scratch folder.
- Icons: edit `src-tauri/android/icon/gen.py`, run it, then `vp run tauri icon src-tauri/android/icon/icon.svg -o src-tauri/icons` and delete `src-tauri/icons/{android,ios}`.

## Releases and signing

- `release.yml`: every push to `main` that changes the app builds the APK, Linux `.deb` + Arch `PKGBUILD` and Windows NSIS installer + `latest.json`, and attaches them to release `v0.x.y`. Android builds only run in CI.
- Android signing key: `~/.tauri/hisaab-sideload.p12` (+ `.pass`), CI secrets `ANDROID_KEYSTORE_B64` / `ANDROID_KEYSTORE_PASSWORD` (`src-tauri/android/README.md`). Windows updater key: `~/.tauri/hisaab-updater.key` (no password), secret `TAURI_SIGNING_PRIVATE_KEY`; public key in `src-tauri/tauri.conf.json`. Never generate new keys: installed apps would reject updates.
- Linux install/update: `scripts/install-arch.sh` (also usable as a Vicinae script).
- Dependabot opens monthly PRs; Tauri crate and `@tauri-apps/*` must be merged together.

## Git

- Default branch `main`. **Commit straight to local `main`** after checks pass; no branches or PRs. **Never push** — the user pushes (a push releases).
- **Conventional Commits** drive the version (`.github/scripts/next-version.sh`, always `0.x.y`): `feat` or breaking → minor, else patch; only user-facing types release. `feat:`/`fix:` subjects are shown to users (release notes, in-app What's new): write them as short user-facing sentences. Don't tag by hand or edit `version` in `tauri.conf.json`.
- Don't commit build outputs (`target/`, `.output/`, `node_modules/`, APKs) or keys.
