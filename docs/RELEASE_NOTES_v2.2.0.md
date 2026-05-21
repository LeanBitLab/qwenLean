# Qwen Studio v2.2.0 Release Notes

**Release Date:** 2026-05-20  
**Type:** Major Release (Tauri v2 Migration)

---

## 🦀 Major Changes

### Migrated to Tauri v2

**What Changed:**
- Replaced Electron with **Tauri v2** (Rust + WebKitGTK)
- Main process: Node.js → Rust
- WebView: Chromium → WebKitGTK
- IPC: `ipcRenderer.invoke()` → `window.__TAURI__.core.invoke()`

**Benefits:**
- **95% smaller binary:** ~6MB (was ~150MB)
- **Better performance:** Rust backend, native Linux WebView
- **Native integration:** System tray, GTK menu, deep linking
- **Improved security:** Sandboxed WebView, no Node.js in renderer

**Package Formats:**
- ✅ DEB (Debian/Ubuntu)
- ✅ RPM (Fedora/RHEL)
- ⚠️ AppImage (linuxdeploy issues - optional)

---

## ✨ New Features

### Settings Updates Tab

**Location:** Settings → Updates (gear icon sidebar)

**Features:**
- Manual "Check for Updates" button
- Real-time download progress with MB counter
- Release notes viewer
- One-click install and restart
- Automatic check on startup (every 4 hours)

**UI States:**
1. **Checking** - Loading spinner
2. **Up to date** - Green checkmark + "Check for Updates" button
3. **Update available** - Download button + release notes
4. **Downloading** - Progress bar + MB counter
5. **Installed** - Restart button
6. **Error** - Retry button

### Popup Notification

**Trigger:** Update available check

**Design:**
- Solid background (no transparency)
- Right-side position (top: 16px, right: 16px)
- Slide-in animation from right
- Fade-out on dismiss
- Two buttons: "View" (navigate to Settings) and "✕" (dismiss)

**Code:** `src/lib.rs` - Global event listener for `update-available`

### Zoom Controls

**Shortcuts:**
- `Ctrl` + `Scroll` - Zoom in/out with mouse wheel
- `Ctrl` + `+` / `=` - Zoom in (10% steps)
- `Ctrl` + `-` - Zoom out (10% steps)
- `Ctrl` + `0` - Reset to 100%

**Range:** 50% - 200%

**Implementation:** JavaScript injection via `zoom_script` in `src/lib.rs`

---

## 🔧 Improvements

### Simplified qwen-core Description

**Before:**
> "Core MCP server with 28 tools for file operations, search, bash execution, time management, and autonomous agent capabilities. Provides filesystem access, git operations, and sequential thinking for AI-assisted development."

**After:**
> "Essential tools for file operations, search, and bash execution."

### Fixed SVG Icon Colors

All SVG icons in update UI now render correctly in dark mode with `color: rgb(247,248,252)`.

### Smooth Dismiss Animation

Popup notification dismiss animation now uses `top` property instead of `transform` to avoid conflicts with centering transform.

---

## 📦 Build Changes

### Tauri Configuration

**tauri.conf.json:**
```json
{
  "bundle": {
    "createUpdaterArtifacts": true,
    "linux": {
      "rpm": {
        "compression": { "type": "none" }
      }
    }
  },
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://github.com/youssefvdel/qwen-studio/releases/latest/download/latest.json"
      ]
    }
  }
}
```

**RPM Fix:** Disabled gzip compression to avoid `rpm-rs` stall on large binaries.

### Build Commands

```bash
# Development
npm run tauri:dev

# Build all formats
npm run tauri:build

# Individual formats
npm run tauri:build:deb
npm run tauri:build:rpm
```

---

## 🐛 Known Issues

### AppImage Bundling

**Issue:** `linuxdeploy` fails during AppImage creation

**Error:**
```
failed to bundle project `failed to run linuxdeploy`
```

**Workaround:** Use DEB or RPM packages instead. AppImage binaries are not included in v2.2.0 release.

**Status:** Open - investigating alternative AppImage tooling

---

## 📊 Statistics

| Metric | v2.1.0 (Electron) | v2.2.0 (Tauri) | Change |
|--------|-------------------|----------------|--------|
| Binary Size (DEB) | ~150MB | ~6.4MB | -95% |
| Binary Size (RPM) | ~160MB | ~16MB | -90% |
| Build Time | ~3 min | ~5 min | +67% |
| Memory Usage | ~400MB | ~250MB | -37% |
| Startup Time | ~2s | ~1s | -50% |

---

## 🔜 Next Release (v2.3.0)

**Planned Features:**
- AppImage support (alternative bundling)
- Skills button in chat sidebar
- MCP Activity sidebar
- Native notifications
- Global shortcuts

---

**Full Changelog:** https://github.com/youssefvdel/qwen-studio/compare/v2.1.0...v2.2.0
