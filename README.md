<p align="center">
  <img src="src-tauri/android/icon/icon.svg" width="96" height="96" alt="Hisaab icon">
</p>

<h1 align="center">Hisaab</h1>

<p align="center">
  <b>Every subscription and bill, with its whole history.</b><br>
  No account, no server, no cloud: your data stays on your devices.
</p>

---

_Hisaab_ means "account, reckoning". It tracks all your recurring payments — streaming, cloud storage, phone plans, rent, insurance — and keeps each one as a history: trials, pauses, cancellations and every price or plan change. Totals for any month in the past stay correct, in your own currency.

**Status:** early. The app shell, releases and in-app updates work; tracking comes next. See [PLAN.md](PLAN.md).

## Install

Android, Windows and Linux, from the [latest release](https://github.com/oowais/hisaab/releases/latest). Once installed, Hisaab updates itself from Settings.

- **Android** (8+, arm64): `hisaab.apk`.
- **Windows**: `hisaab_<version>_x64-setup.exe`.
- **Arch / CachyOS**: `scripts/install-arch.sh` installs or updates it as `hisaab-bin`, or add `hisaab-x86_64.AppImage` from GitHub (`oowais/hisaab`) in the CachyOS Package Installer, which also updates it. The AppImage uses the system's `webkit2gtk-4.1`, `gtk3` and `libayatana-appindicator`. Other distributions: the `.deb` or the AppImage.

## Build

Needs Rust, Node and [Vite+](https://viteplus.dev) (`vp`).

```bash
vp install
vp run tauri dev
```

Checks: `vp check`, `vp test`, `cargo test --workspace`.

## License

GPL-3.0. Sibling of [Befikr](https://github.com/oowais/befikr).
