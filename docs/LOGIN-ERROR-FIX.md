# Login Error Fix — "Oops! Something unexpected happened"

## 🐛 Root Cause

The `pre_load_script` in `src/window.rs` was **wiping ALL localStorage and sessionStorage** on every page load:

```js
// BEFORE (broken)
try { localStorage.clear(); } catch(e) {}   // ❌ NUKES AUTH TOKENS
try { sessionStorage.clear(); } catch(e) {} // ❌ NUKES SESSION STATE
```

When Qwen's OAuth login flow completes, the web app stores auth tokens in `localStorage`:
- `token`, `sid`, `ticket`, `auth_token`, etc.

Then the page navigates/redirects back to `chat.qwen.ai` → the init script runs → **all auth data is destroyed** → login fails with failure code.

## ✅ The Fix

**Removed** both `localStorage.clear()` and `sessionStorage.clear()` calls entirely.

The MCP injector now **only** reads → merges → writes the `LOCAL_MCP_SERVER` key without touching anything else.

```js
// AFTER (fixed)
// No clear() calls — auth tokens, session state, user preferences all preserved
// Only LOCAL_MCP_SERVER is managed (merge qwen-core entry, preserve user-added servers)
```

## 📝 Changes

| File | What Changed |
|------|-------------|
| `src/window.rs` | Removed `localStorage.clear()` + `sessionStorage.clear()` from `pre_load_script` |

## 🔍 Why This Works

The original `clear()` calls were meant to force a "clean session" but they were too aggressive. The MCP merge logic already handles the only thing that needs managing — the MCP server list. Everything else (auth, preferences, session) should be left alone.
