# Clipboard Paste Fix — Text & Image (v2.2.3)

## 🐛 The Problem

Both **text** and **image** paste from outside the app (Ctrl+V) were broken in Qwen Studio.

### Root Cause: WebKitGTK Clipboard Gap

Tauri on Linux uses **WebKitGTK** as its renderer (unlike Electron which uses Chromium). WebKitGTK does **NOT** bridge the system clipboard (X11/Wayland) to the web `ClipboardEvent.clipboardData` API.

This means:
- **Text paste**: `e.clipboardData.getData('text')` returns `''` (empty string) → nothing appears
- **Image paste**: `e.clipboardData.items` is empty → image paste silently fails

The Qwen web app's React paste handlers rely on `clipboardData` being populated — which it never is in Tauri/WebKitGTK.

## ✅ The Fix

### Strategy

Intercept ALL paste events in capture phase, **prevent the broken default**, then read clipboard data via Tauri's native `clipboard-manager` plugin and manually inject the content:

```
User presses Ctrl+V
  → Capture-phase listener fires
  → e.preventDefault() + e.stopPropagation()
  → Tauri clipboardManager.readImage() (async)
    → Image found? → Convert RGBA→PNG via canvas → Inject as File
    → No image? → clipboardManager.readText()
      → Text found? → Insert at cursor position in active element
      → Empty? → Log and do nothing
```

### Files Changed

| File | What | Why |
|------|------|-----|
| `electron-bridge.js` | Rewrote `setupClipboardPaste()` entirely | Old handler only tried images, didn't prevent default, didn't handle text |
| `capabilities/default.json` | `"windows": ["main"]` → `["main", "window-*"]` | New windows (Ctrl+N) now also get clipboard permissions |

### Text Paste Details

The `insertTextAtCursor()` helper handles 3 cases:

1. **`<input>` / `<textarea>`** — Manipulates `.value` directly, fires `input` + `change` events so React state updates
2. **`contenteditable`** — Uses `Selection` + `Range` API to insert at cursor, fires `input` event
3. **Fallback** — Dispatches `beforeinput` + `input` InputEvents with `inputType: 'insertText'` and `data: text` — works with React apps using custom input components

### Image Paste Details

The `handleImagePaste()` function:

1. Creates an offscreen `<canvas>` with image dimensions
2. Writes RGBA pixel data via `ImageData` + `putImageData()`
3. Calls `canvas.toBlob()` to get a PNG Blob
4. Wraps in a `File` object with proper filename
5. Tries 3 injection methods in order:
   - `#filesUpload` input (same as drag-drop)
   - `#dropzone-container` synthetic drop event
   - Synthetic paste event with `clipboardData` containing the image

### Why `e.preventDefault()` + `e.stopPropagation()`?

Without preventing default, the browser fires its own (broken) paste handler which sees empty `clipboardData` and does nothing — but may interfere with our async Tauri clipboard read. Stopping propagation prevents the Qwen web app's own paste handler from firing prematurely with empty data.

## 🧪 Testing

1. Copy text from Firefox/Terminal → Ctrl+V in chat input → should appear ✅
2. Screenshot with `gnome-screenshot` or `flameshot` → Ctrl+V in chat → should upload ✅
3. Copy text within the app → Ctrl+V → should work ✅
4. Open new window (Ctrl+N) → paste should work there too ✅

## 📝 Notes

- The Tauri clipboard API is **async** (returns Promises), so all paste operations happen asynchronously after `preventDefault()`. This is unavoidable but imperceptible to the user.
- `clipboardManager.readImage()` **throws** when no image is in clipboard (not just returns null). The code catches this and falls through to `readText()`.
- `clipboardManager.readText()` returns `null` or `''` when clipboard is empty — both are handled gracefully.
