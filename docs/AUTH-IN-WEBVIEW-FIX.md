# Auth In-WebView Fix (No External Browser / No Deep Link)

## 🐛 The Problem

When clicking "Login" on `chat.qwen.ai`, the Qwen web app calls `electronAPI.open_external_link(authUrl)`.
The old flow:

1. `open_external_link()` → opens URL in **system browser** (Firefox/Chrome)
2. User authenticates in browser
3. Browser redirects to `qwen://callback?token=...` (deep link)
4. Tauri deep-link plugin catches it → injects token into WebView

**This was broken because:**
- Deep-link protocol registration is unreliable on Linux
- Opening external browser breaks the UX entirely
- The web app showed `"Oops! Something unexpected happened"` errors during login

## ✅ The Fix

Auth now happens **inside the WebView** — no external browser, no deep link needed.

### Flow (New):

1. User clicks "Login" on `chat.qwen.ai`
2. Web app calls `electronAPI.open_external_link(authUrl)`
3. `open_external_link` detects it's an **auth URL** (by domain or path)
4. Instead of opening system browser → **navigates the WebView** to the auth page
5. User logs in inside the app
6. Auth page redirects back to `chat.qwen.ai` with session cookies
7. User is logged in ✅

## 📝 Files Changed

| File | What Changed | Why |
|------|-------------|-----|
| **`src/lib.rs`** | Commented out `tauri_plugin_deep_link::init()` | Deep link no longer needed |
| **`src/lib.rs`** | Commented out `setup_deep_link()` call in `.setup()` | No deep link handler needed |
| **`src/window.rs`** | Rewrote `open_external_link()` | Detects auth URLs → navigates WebView instead of opening browser |
| **`src/window.rs`** | Added `#[allow(dead_code)]` to `setup_deep_link` + `handle_deep_link_url` | Suppress warnings for disabled functions |
| **`capabilities/default.json`** | Added `*.alibaba.com`, `*.aliyun.com`, `*.alibabacloud.com` to remote URLs | Auth may redirect to Alibaba/Aliyun domains; WebView needs Tauri API access there |

## 🔧 Auth URL Detection

The `open_external_link` command checks if a URL is an auth URL by:

### Domain matching:
- `accounts.qwen.ai`
- `account.qwen.ai`
- `login.qwen.ai`
- `auth.qwen.ai`
- `oauth.qwen.ai`
- `passport.alibaba.com`
- `login.alibaba.com`
- `signin.alibaba.com`
- `accounts.alibaba.com`
- `account.alibaba.com`
- `login.aliyun.com`
- `account.aliyun.com`
- `signin.aliyun.com`

### Path matching:
- `/login`
- `/auth`
- `/oauth`
- `/callback`
- `/signin`
- `/signup`

If ANY match → navigate WebView. Otherwise → open in system browser.

## 🔒 Safety

- Deep link functions are **commented out, not deleted** — easy to re-enable if needed
- `pre_load_script` already has a guard that **skips MCP injection** on non-`chat.qwen.ai` pages and auth pages (login/auth/callback/oauth paths)
- localStorage/sessionStorage are NOT cleared on auth pages, preserving session cookies
- The WebView navigates naturally through the auth flow — cookies and redirects work as expected

## 🚀 How to Re-enable Deep Links (if needed)

1. Uncomment `.plugin(tauri_plugin_deep_link::init())` in `src/lib.rs`
2. Uncomment the `setup_deep_link` block in `.setup()`
3. Remove the `#[allow(dead_code)]` attributes from `setup_deep_link` and `handle_deep_link_url`
