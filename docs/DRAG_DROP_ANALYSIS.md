# Drag & Drop Analysis - chat.qwen.ai

## Date: 2026-05-21
## Status: WebKitGTK Limitation Confirmed

---

## 1. Problem Statement

On Linux, the Tauri v2 app uses **WebKitGTK** as the WebView backend. HTML5 drag/drop events (`dragenter`, `dragover`, `dragleave`, `drop`) do not fire properly, making file uploads via drag-and-drop impossible.

### Affected Features:
- Dragging files onto the chat window to upload
- Pasting images from clipboard

### Working Platforms:
- Windows (WebView2/Chromium) - ✅ Works
- macOS (WKWebView) - ✅ Works
- Linux (WebKitGTK) - ❌ Does NOT work

---

## 2. Technical Analysis

### chat.qwen.ai Implementation

The official web app (tested in Chrome) uses:

#### Drop Zone Overlay
```html
<div class="dropzone-overlay" role="region">
  <div class="dropzone-overlay-content">
    <div class="add-files-placeholder">
      <div class="emoji">
        <img alt="" src="https://img.alicdn.com/...svg">
      </div>
      <div class="title">Drop any files here to add to the conversation</div>
      <div class="description">Add a file, image, video, or audio</div>
    </div>
  </div>
</div>
```

- Position: `fixed` (covers entire viewport)
- Visibility: Shown when dragging files over window
- Text content: "Drop any files here to add to the conversation"

#### File Input
```html
<input type="file" id="filesUpload" multiple="" style="display: none;">
```
- Hidden file input
- Located inside `.mode-select` container
- Accepts: images, documents, audio, video (via `accept` attribute)

#### Event Flow (Chrome/Chromium - WORKS)
1. Browser detects native drag events
2. Shows `.dropzone-overlay` when dragging over window
3. On drop: files are set on `#filesUpload`
4. Change event triggers React state update
5. File upload begins

### WebKitGTK Issue

WebKitGTK doesn't expose native file drop events to JavaScript. Tauri's `disable_drag_drop_handler()` and HTML5 drag/drop APIs don't work on Linux.

**References:**
- [tauri-apps/tauri#13171](https://github.com/tauri-apps/tauri/issues/13171): "If you need the native html/js drag and drop api to work you must disable tauri's own drag drop events. They are mutually exclusive because of webview limitations."
- [tauri-apps/tauri#11930](https://github.com/tauri-apps/tauri/issues/11930): HTML drag/drop events don't work reliably on macOS/Linux with WebKitGTK.

---

## 3. Proposed Solutions

### Option A: Native GTK Drag-Drop → JS Bridge (Recommended)

**Architecture:**
```
User drags files over window
    ↓
GTK detects drag events (Rust)
    ↓
Rust emits 'file-drag-enter/drop' event to webview
    ↓
JS shows drop zone overlay + listens for event
    ↓
On drop: Rust reads file paths → sends to JS
    ↓
JS programmatically sets files on #filesUpload
    ↓
React app processes file upload
```

**Pros:**
- Native feel on Linux
- Consistent UX across platforms
- No user code changes needed in chat.qwen.ai

**Cons:**
- Requires Rust code changes
- Complex event passing between Rust ↔ JS

### Option B: Clipboard Image Paste (Separate Issue)

Similar approach - Rust handles clipboard read, JS triggers file input.

### Option C: Custom JS Drag Detection

Use mouse movement polling to detect if a file is being dragged (hacky workaround).

---

## 4. Implementation Notes

### Current electron-bridge.js Structure
The bridge currently provides:
- `window.electronAPI` for IPC calls
- Clipboard paste support (v2.2.1+)
- Drag-drop pass-through (non-functional on WebKitGTK)

### Key Files to Modify
1. `src/lib.rs` - Add GTK drag event listeners
2. `electron-bridge.js` - Add drop zone overlay + file handling
3. `capabilities/default.json` - May need additional permissions

### Drop Zone HTML Template
```html
<div id="qwen-studio-drop-overlay" style="
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 99999;
  pointer-events: none;
">
  <div style="
    background: white;
    padding: 40px 60px;
    border-radius: 16px;
    text-align: center;
    pointer-events: auto;
  ">
    <div style="font-size: 48px; margin-bottom: 16px;">📎</div>
    <div style="font-size: 18px; font-weight: 500;">Drop files here to upload</div>
  </div>
</div>
```

---

## 5. Related Issues

| Issue | Link |
|-------|------|
| WebKitGTK drag/drop limitation | [#3](https://github.com/youssefvdel/qwen-studio/issues/3) |
| Clipboard paste fix | v2.2.1 release |

---

## 6. Next Steps

1. [ ] Implement GTK drag-drop listener in Rust
2. [ ] Emit events to webview when files are dragged/dropped
3. [ ] Add drop zone overlay to electron-bridge.js
4. [ ] Handle dropped files programmatically
5. [ ] Test on multiple Linux distros

---

## 7. Screenshots

See `.agent-screenshots/` directory for:
- `current.png` - Login page
- `drop-zone-visible.png` - Drop zone appearing during drag (Chrome)

---

*Document generated during drag-drop investigation on 2026-05-21*