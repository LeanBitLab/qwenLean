# Image Paste Infinite Loop Fix

**Version:** v2.2.3  
**Date:** 2026-05-22  
**File Changed:** `electron-bridge.js`

---

## 🐛 Problem

When pasting an image (Ctrl+V), the image was pasted **~100 times** instead of once.

## 🔍 Root Cause

**Infinite recursion** in the paste handler:

```
User presses Ctrl+V
    ↓
Paste listener intercepts → reads clipboard image → calls injectImageFile()
    ↓
injectImageFile() Method 3 dispatches synthetic ClipboardEvent('paste')
    ↓
Our OWN paste listener intercepts the synthetic paste (it's on document in capture phase!)
    ↓
Reads clipboard image AGAIN → calls injectImageFile() AGAIN
    ↓
injectImageFile() dispatches ANOTHER synthetic paste
    ↓
... INFINITE LOOP (~100 iterations until something times out)
```

The synthetic paste was meant to trigger the web app's React paste handler, but our own listener intercepted it first because it runs in **capture phase** on `document` — before any other handler.

## ✅ Fix (2 changes)

### 1. Removed Method 3 (synthetic paste dispatch) from `injectImageFile()`

The synthetic `ClipboardEvent('paste')` dispatch was the root cause. Removed it entirely.

**Remaining injection methods are sufficient:**
- **Method 1:** Find ALL `<input type="file">` → set files + dispatch `change` event
- **Method 2:** `#filesUpload` legacy input → set files + dispatch `change`
- **Method 4:** `#dropzone-container` → dispatch synthetic `drop` event

### 2. Added re-entrancy guard as safety net

Even with Method 3 removed, added a guard flag to prevent ANY future re-entrancy:

```js
var __pasteInProgress = false;
function resetPasteGuard() { __pasteInProgress = false; }

document.addEventListener('paste', function(e) {
  if (__pasteInProgress) {
    console.log('[ElectronBridge] Blocking re-entrant paste event');
    return;
  }
  __pasteInProgress = true;
  // ... async clipboard read + injection ...
  // resetPasteGuard() called at every terminal path
}, true);
```

The guard is reset at **all 7 terminal paths** (image success, image fail → text, empty clipboard, file URL, etc.)

## 📝 Files Changed

| File | What Changed |
|------|-------------|
| `electron-bridge.js` | Removed Method 3 (synthetic paste) from `injectImageFile()` |
| `electron-bridge.js` | Added `__pasteInProgress` re-entrancy guard + `resetPasteGuard()` at all terminal paths |

## ✅ Result

- Ctrl+V pastes image **exactly once** ✅
- No more infinite loop ✅
- File input injection (Methods 1, 2) + dropzone drop (Method 4) handle all upload paths ✅
