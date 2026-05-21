# Drag-and-Drop & Clipboard Paste Fix

## Problem Summary

The Qwen web app (chat.qwen.ai) was built for **Electron/Chromium**, which handles file drag-and-drop and clipboard paste natively. When wrapping it in **Tauri v2** (which uses **WebKitGTK** on Linux), both features broke:

1. **Drag-and-drop**: Files dragged from the OS file manager were not accepted
2. **Clipboard paste**: Images copied to clipboard couldn't be pasted into the chat input

## Root Cause Analysis

### Drag-and-Drop

In Electron/Chromium, when you drag a file from the OS onto the web page:
- The browser fires standard DOM `DragEvent`s (`dragenter`, `dragover`, `drop`)
- The `drop` event's `dataTransfer.files` contains actual `File` objects
- The Qwen web app's React `useDragFile` hook listens on `#dropzone-container` for these events

In **Tauri/WebKitGTK**:
- Tauri **intercepts OS-level drag events** before they reach the WebView
- Tauri emits them as native events via `onDragDropEvent()` on the `WebviewWindow`
- The web-level `drop` event either **doesn't fire** or fires with **empty `dataTransfer.files`**
- The web app's React handler never receives the actual files

### Clipboard Paste

In Electron/Chromium, when you paste an image (Ctrl+V):
- The browser fires a `paste` event with `clipboardData.items` containing the image
- The Qwen web app's `inputPasteHandler` reads these items and calls `FilesManager.addFile()`

In **Tauri/WebKitGTK**:
- WebKitGTK does **not** provide image data in the standard `paste` event's `clipboardData`
- `navigator.clipboard.read()` may not be available or may lack permission
- The web app's paste handler finds no image items and does nothing

## Files Changed

### 1. `electron-bridge.js` — Main fix (JavaScript injection layer)

#### `setupTauriDragDrop()` — REWRITTEN

**What changed:**
- The old code only tried to inject files into `#filesUpload` (a hidden file input)
- The new code bridges Tauri's native drag events to **synthetic DOM events** that the web app understands

**How it works now:**

1. **`onDragDropEvent` with type `over`**:
   - Dispatches synthetic `DragEvent('dragenter')` and `DragEvent('dragover')` on `#dropzone-container`
   - This triggers the web app's drag-over UI (the visual drop zone indicator)
   - The synthetic events have a `DataTransfer` object so the React handler detects it as a file drag

2. **`onDragDropEvent` with type `cancel`/`leave`**:
   - Dispatches synthetic `DragEvent('dragleave')` on `#dropzone-container`
   - Hides the web app's drag-over UI

3. **`onDragDropEvent` with type `drop`**:
   - Reads file contents via Tauri's `fs.readFile()` plugin (with actual byte content, not empty)
   - Creates proper `File` objects with correct MIME types
   - **Primary method**: Dispatches a synthetic `DragEvent('drop')` with a `DataTransfer` containing the files on `#dropzone-container` — the web app's React handler picks this up naturally
   - **Fallback 1**: Injects files into `#filesUpload` input and fires a `change` event
   - **Fallback 2**: Dispatches on `document.body` as last resort

**Why this approach:**
- The web app's `useDragFile` hook specifically listens on `#dropzone-container` for `drop` events
- By dispatching a synthetic event there with real `File` objects in `dataTransfer`, the React handler processes files exactly as it would in Electron
- Multiple fallbacks ensure robustness across different states of the SPA

**Additional improvements:**
- Added more MIME type mappings (audio, code files, etc.)
- Cached `#dropzone-container` reference with re-query on SPA navigation
- Better error handling with multiple fallback strategies

#### `setupClipboardPaste()` — NEW FUNCTION

**What it does:**
- Listens for `paste` events in capture phase (before the web app's handler)
- Uses Tauri's `clipboardManager.readImage()` to read image data from clipboard
- Converts raw RGBA pixel data to a PNG `Blob` using an offscreen `<canvas>`
- Creates a `File` object from the PNG blob
- Injects the file into `#filesUpload` input OR dispatches a synthetic `ClipboardEvent('paste')` with the image in `clipboardData`

**Why this approach:**
- WebKitGTK doesn't expose clipboard image data through standard web APIs
- Tauri's clipboard manager plugin provides native clipboard access
- The canvas RGBA→PNG conversion is needed because Tauri returns raw pixel data, not encoded images
- Multiple injection methods ensure the web app receives the image regardless of which handler it uses

### 2. `src/lib.rs` — Rust initialization script

**What changed:**
- **Removed** the old `paste` event handler from `pre_load_script` that tried `navigator.clipboard.read()` and `clipboardManager.readText()`
- Replaced with a comment explaining that paste handling is now in `electron-bridge.js`

**Why:**
- The old handler ran before `__TAURI__` was available, so it couldn't use Tauri's clipboard manager reliably
- It conflicted with the new `electron-bridge.js` handler (both tried to handle the same paste event)
- Text paste works natively in WebKitGTK — no special handling needed
- Image paste now has a proper dedicated handler in `electron-bridge.js` with full Tauri access

### 3. `capabilities/default.json` — Tauri permissions

**What changed:**
Added filesystem read permissions for user directories:
```json
"fs:allow-read-file",
"fs:allow-home-read-recursive",
"fs:allow-desktop-read-recursive",
"fs:allow-document-read-recursive",
"fs:allow-download-read-recursive"
```

**Why:**
- The drag-and-drop handler reads files via Tauri's `fs.readFile()` plugin
- Without these permissions, `readFile()` would fail for files outside the app's sandboxed directories
- Users drag files from their Home, Desktop, Documents, and Downloads folders
- The `-recursive` variants allow reading files in subdirectories

## Testing Checklist

- [ ] Drag a file from file manager onto the chat → file should appear in upload area
- [ ] Drag an image file onto the chat → image should appear as preview
- [ ] Drag multiple files onto the chat → all should appear
- [ ] Copy an image (e.g., screenshot) and paste (Ctrl+V) → image should appear in upload
- [ ] Copy text and paste (Ctrl+V) → text should insert into chat input normally
- [ ] Drag a file then cancel (drag away) → drop zone UI should disappear

## Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│  OS File Manager (drag file)                        │
└──────────────────────┬──────────────────────────────┘
                       │ OS drag event
                       ▼
┌─────────────────────────────────────────────────────┐
│  Tauri WebView (WebKitGTK)                          │
│  ┌─────────────────────────────────────────────┐    │
│  │ WebKitGTK intercepts drag → onDragDropEvent │    │
│  └──────────────────────┬──────────────────────┘    │
│                         │                           │
│  ┌──────────────────────▼──────────────────────┐    │
│  │ electron-bridge.js                          │    │
│  │                                             │    │
│  │ onDragDropEvent('over')                     │    │
│  │   → dispatch synthetic dragenter/dragover   │    │
│  │     on #dropzone-container                  │    │
│  │                                             │    │
│  │ onDragDropEvent('drop')                     │    │
│  │   → fs.readFile(paths) via Tauri fs plugin  │    │
│  │   → create File objects with actual content │    │
│  │   → dispatch synthetic DragEvent('drop')    │    │
│  │     with DataTransfer on #dropzone-container│    │
│  └──────────────────────┬──────────────────────┘    │
│                         │                           │
│  ┌──────────────────────▼──────────────────────┐    │
│  │ Qwen Web App (React)                        │    │
│  │                                             │    │
│  │ useDragFile hook on #dropzone-container     │    │
│  │   → reads e.dataTransfer.files              │    │
│  │   → FilesManager.addFiles(files)            │    │
│  │   → Uploads to server                       │    │
│  └─────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
```

## Notes

- The `#dropzone-container` element is created dynamically by the Qwen React app. The bridge code re-queries it on each event to handle SPA navigation.
- The `#filesUpload` element is a hidden file input used by the web app for file picker uploads. It's used as a fallback method.
- The synthetic `DataTransfer` API is supported in WebKitGTK (WebKit2GTK 2.38+).
- The clipboard image paste uses canvas RGBA→PNG conversion because Tauri's `readImage()` returns raw pixel data, not encoded image formats.
