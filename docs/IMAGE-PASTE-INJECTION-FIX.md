# Image Paste Not Injecting into Chat UI — Fix

**Date:** 2026-05-21
**Version:** 2.2.3
**File Changed:** `electron-bridge.js`

## 🐛 Problem

Rust side reads clipboard image perfectly (916x790 pixels → 157KB PNG → 210K base64 chars), but the image **never appears in the chat UI**.

## 🔍 Root Cause (3 issues)

### 1. `#filesUpload` input doesn't exist
The old code only tried `document.getElementById('filesUpload')` — but the Qwen React app's hidden file input has **no predictable ID**. The query returned `null` → silently skipped.

### 2. Focus lost after async call
The paste handler calls `invoke('read_clipboard_image')` which is **async**. By the time Rust returns the base64 (50-200ms later), `document.activeElement` may have changed → synthetic paste dispatched on wrong element.

### 3. Synthetic paste target was wrong
Dispatched on `document.activeElement || document.body` — but the React app listens for paste on the **chat textarea**, not body.

## ✅ Fix

### 1. Find ALL `<input type="file">` elements
Instead of looking for a specific ID, query all file inputs on the page:
```js
var allFileInputs = document.querySelectorAll('input[type="file"]');
```
Set files and dispatch `change` on **all** of them with `bubbles: true` so React's event delegation picks it up.

### 2. Save active element BEFORE async call
```js
var savedActiveElement = document.activeElement; // Save immediately
invoke('read_clipboard_image').then(function(base64) {
  injectImageFile(file, savedActiveElement); // Use saved reference
});
```

### 3. Smart paste target fallback chain
If saved element isn't suitable, find the chat input:
```js
var pasteTarget = savedActiveElement || document.activeElement;
if (!pasteTarget || pasteTarget === document.body) {
  pasteTarget = document.querySelector('textarea') ||
                document.querySelector('[contenteditable="true"]') ||
                document.querySelector('[role="textbox"]') ||
                document.body;
}
```

### 4. Four injection methods (ordered by reliability)
1. **ALL `<input type="file">` elements** — set files + dispatch change
2. **`#filesUpload`** — legacy fallback
3. **Synthetic paste event** — on saved/smart target with file in clipboardData
4. **`#dropzone-container` drop** — last resort

## 📝 Files Changed

| File | What Changed |
|------|-------------|
| `electron-bridge.js` | Rewrote `injectImageFile()` with 4-method fallback chain |
| | Save `document.activeElement` before async invoke |
| | Pass `savedActiveElement` through all handler functions |
| | Smart textarea/contenteditable detection for paste target |
| | Added logging for discovered file inputs and injection results |
