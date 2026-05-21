# v2.2.3 — Updater Fix & Release Workflow

## Summary

Fixed false update notifications showing "new version available" even when on the latest version, fixed the broken CI/CD release workflow, and bumped version to 2.2.3.

---

## Changes

### 1. Version Bump → 2.2.3

| File | Before | After | Why |
|------|--------|-------|-----|
| `Cargo.toml` | 2.2.2 | 2.2.3 | Rust crate version |
| `tauri.conf.json` | 2.2.2 | 2.2.3 | Tauri app version |
| `package.json` | 2.2.0 | 2.2.3 | Was out of sync — now consistent |
| `src/lib.rs` user-agent | QWENCHAT/2.2.0 | QWENCHAT/2.2.3 | User-agent string for chat.qwen.ai |
| `src/window.rs` user-agent | QWENCHAT/2.2.0 | QWENCHAT/2.2.3 | Same — new windows |

---

### 2. Updater False Positive Fix

**Root Cause:** The updater blindly trusted `tauri-plugin-updater`'s response without verifying the remote version was actually newer. Also, background auto-checks (on startup + every 4 hours) were showing a popup banner notification — same as manual checks.

**Fix in `src/lib.rs`:**

- Added `compare_versions()` — semver comparison helper that returns -1/0/1
- `check_for_updates()`:
  - Now compares remote vs local version before notifying
  - Only emits `update-available` event (banner) on **manual** checks
  - Auto-checks (startup + periodic) silently log — no banner
- `get_update_info()`: Same version comparison guard — returns `available: false` if remote isn't newer
- `install_update_with_progress()`: Refuses to install if remote isn't strictly newer

**What this means for you:**
- ✅ No more "update available" banner when you're on the latest
- ✅ Background checks are silent — go to Settings > Updates to check manually
- ✅ Can't accidentally "downgrade" via the updater

---

### 3. Release Workflow Fix (`.github/workflows/release.yml`)

**Was broken because:**
- Ran `npm run build` — this script doesn't exist! (should be `tauri build`)
- Missing Rust toolchain setup
- Missing system dependencies (`libwebkit2gtk-4.1-dev`, etc.)
- No proper release artifact upload (deb, rpm, appimage, latest.json)

**Now:**
- Installs Rust toolchain + system deps
- Builds via `tauri build`
- Uploads all artifacts (deb, rpm, AppImage, .sig files) to GitHub Release
- Uploads `latest.json` to the release (required for the updater to work!)
- Uses `TAURI_SIGNING_PRIVATE_KEY` secret for signing

**Required GitHub Secrets:**
| Secret | Description |
|--------|-------------|
| `TAURI_SIGNING_PRIVATE_KEY` | Private key from `tauri signer generate` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the signing key |

---

### 4. Permissions Fix

Added `create_new_window` command to `permissions/custom.toml` — was missing, which could cause the Ctrl+N / tray "New Window" to fail in production builds.

---

## How the Updater Works

```
┌─────────────────────────────────────────────┐
│  check_for_updates(app, manual=false)       │
│  Called on: startup (3s delay) + every 4h   │
│                                             │
│  1. Fetch latest.json from GitHub releases  │
│  2. Compare remote version vs local version │
│  3. If remote <= local → skip (log only)    │
│  4. If remote > local → log only (no banner)│
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│  check_for_updates(app, manual=true)        │
│  Called on: "Check for Updates" menu click  │
│                                             │
│  Same as above BUT:                         │
│  4. If remote > local → emit banner event   │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│  get_update_info()                          │
│  Called from: Settings > Updates tab        │
│                                             │
│  1. Fetch latest.json                       │
│  2. Version comparison guard                │
│  3. Return { available, version, notes }    │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│  install_update_with_progress()             │
│  Called from: "Download & Install" button   │
│                                             │
│  1. Version comparison guard                │
│  2. Download with progress events           │
│  3. Install update binary                   │
│  4. User clicks "Restart Now"               │
│  5. app.restart() applies the update        │
└─────────────────────────────────────────────┘
```

---

## Publishing a New Release

```bash
# 1. Bump version in ALL files (keep in sync!)
#    Cargo.toml, tauri.conf.json, package.json, user-agent strings

# 2. Commit & tag
git add -A
git commit -m "v2.2.3"
git tag v2.2.3

# 3. Push — triggers the release workflow
git push origin main --tags
```

The GitHub Action will:
1. Build the app
2. Sign the binaries
3. Create a GitHub Release with all artifacts
4. Upload `latest.json` so the in-app updater works
