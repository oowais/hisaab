# Sideload signing key

The release APK is signed with one key, every build, so updates install over
each other (Android rejects an update signed with a different key — you'd
have to uninstall and lose data).

The repo is public, so the key is **not** committed. It lives on the user's
machine next to the updater key, and in two repo secrets that CI reads
(`.github/workflows/android.yml`):

| File                                                    | Secret                                      |
| ------------------------------------------------------- | ------------------------------------------- |
| `~/.tauri/hisaab-sideload.p12` (PKCS12, alias `hisaab`) | `ANDROID_KEYSTORE_B64` (base64 of the file) |
| `~/.tauri/hisaab-sideload.pass`                         | `ANDROID_KEYSTORE_PASSWORD`                 |

```bash
base64 -w0 ~/.tauri/hisaab-sideload.p12 | gh secret set ANDROID_KEYSTORE_B64 -R oowais/hisaab
gh secret set ANDROID_KEYSTORE_PASSWORD -R oowais/hisaab < ~/.tauri/hisaab-sideload.pass
```

Never generate a new key once an APK is installed: updates would stop
installing. Keep a backup of both files.
