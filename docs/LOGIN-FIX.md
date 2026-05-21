# Login Fix — "Oops! Something unexpected happened"

**Date:** 2026-05-22  
**Version:** 2.2.3  
**File Changed:** `src/window.rs`

## 🐛 The Bug

When trying to log in to chat.qwen.ai, the app showed:

```
Oops! Something unexpected happened.
Try refreshing and if the issue persists please contact support.
Failure code: 1779386796589
```

This was a **regression** introduced when trying to fix the MCP injector overwrite issue.

## 🔍 Root Cause

The original `pre_load_script` cleared **all** localStorage and sessionStorage on every page load:

```js
localStorage.clear();
sessionStorage.clear();
localStorage.setItem("LOCAL_MCP_SERVER", ...);
```

When I fixed the MCP injector to preserve user-added servers, I initially removed the clears entirely. But that wasn't the issue.

The real problem: **clearing storage on every page load interferes with the login flow**. The Qwen web app:
1. Sets auth tokens in localStorage during login
2. May navigate or reload as part of the auth flow
3. Our script runs on page load → clears those tokens → login fails

Even when I tried to preserve auth tokens before clearing, the timing was wrong — the script runs before the page's own JavaScript, so it would clear tokens that were just set by the auth callback.

## ✅ The Fix

**Stop clearing storage entirely.** The script now only handles MCP config merging:

```js
// Read existing LOCAL_MCP_SERVER
var existing = localStorage.getItem("LOCAL_MCP_SERVER");

// Parse and merge qwen-core entry
// Write back merged config
```

No more `localStorage.clear()` or `sessionStorage.clear()`. The Qwen web app manages its own auth state, and we don't interfere.

## 📝 Changes

| File | What Changed | Why |
|------|-------------|-----|
| `src/window.rs` | Removed all `localStorage.clear()` and `sessionStorage.clear()` calls | These were breaking the login flow by wiping auth tokens |
| `src/window.rs` | Kept only MCP config merge logic | Preserves user-added MCP servers without touching other storage |

## ✅ Result

- **Login works** — auth tokens persist across page loads as the Qwen app expects
- **MCP servers preserved** — user-added servers survive page loads via merge logic
- **No interference** — we only touch `LOCAL_MCP_SERVER`, nothing else
- **Clean separation** — auth state is managed by the Qwen web app, not our pre-load script
