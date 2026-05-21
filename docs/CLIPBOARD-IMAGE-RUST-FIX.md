# Clipboard Image Paste Fix — Custom Rust Command

**Version:** 2.2.3  
**Date:** 2026-05-22  
**Problem:** Pasting screenshots/images from outside the app fails silently or pastes text instead

---

## 🐛 Root Cause

Tauri's `clipboard-manager` plugin `readImage()` **fails on Linux/WebKitGTK** when the clipboard contains image data from screenshot tools (GNOME Screenshot, Flameshot, etc.).

The old flow:
1. `cm.readImage()` → **throws error** (plugin can't read GTK clipboard image format)
2. Falls to `.catch()` → tries `cm.readText()`
3. `readText()` also **throws** because clipboard has image data, not text
4. Logs: `"The clipboard contents were not available in the requested format or the clipboard is empty."`
5. **Nothing is pasted** ❌

## ✅ The Fix

Created a **custom Rust command** `read_clipboard_image` that uses GTK's native clipboard API directly — bypassing the unreliable Tauri plugin.

### New Flow (3-tier fallback)

| Priority | Method | Description |
|----------|--------|-------------|
| 1 | `invoke('read_clipboard_image')` | Custom Rust command → GTK clipboard → pixbuf → PNG → base64 |
| 2 | `cm.readImage()` | Tauri plugin (fallback for edge cases) |
| 3 | `cm.readText()` | Text paste (only if no image found) |

---

## 📝 Files Changed

| File | What Changed | Why |
|------|-------------|-----|
| **`Cargo.toml`** | Added `base64 = "0.22"` dependency | Needed to encode PNG bytes as base64 for JS |
| **`src/window.rs`** | Added `read_clipboard_image` Tauri command (Linux-only) | Uses GTK clipboard API directly — more reliable than plugin |
| **`src/lib.rs`** | Registered `read_clipboard_image` in invoke handler | Makes command callable from JS |
| **`permissions/custom.toml`** | Added `read_clipboard_image` to `window-commands` | Tauri permission system requires explicit allow |
| **`electron-bridge.js`** | Rewrote paste handler with 3-tier fallback | Custom command first, plugin second, text last |

---

## 🔧 How the Custom Rust Command Works

```rust
#[tauri::command]
pub async fn read_clipboard_image() -> Result<String, String> {
    // 1. Schedule on GTK main thread (required for GTK API)
    glib::idle_add_local(move || {
        // 2. Get GTK clipboard
        let clipboard = gtk::Clipboard::get(&gtk::gdk::Atom::intern("CLIPBOARD"));
        
        // 3. Read image as GdkPixbuf (handles PNG, BMP, JPEG, etc.)
        let pixbuf = clipboard.wait_for_image()?;
        
        // 4. Save pixbuf as PNG bytes in memory
        let png_bytes = pixbuf.save_to_bufferv("png", &[])?;
        
        // 5. Send back to async context
        tx.send(Ok(png_bytes));
    });
    
    // 6. Base64 encode for JS
    let encoded = base64::STANDARD.encode(&png_bytes);
    Ok(encoded)
}
```

### JS Side
```js
invoke('read_clipboard_image').then(function(base64Png) {
    var blob = base64ToBlob(base64Png, 'image/png');
    var file = new File([blob], 'pasted-image.png', { type: 'image/png' });
    injectImageFile(file);  // Upload via #filesUpload / #dropzone-container
});
```

---

## 🧪 Testing

1. Take a screenshot (PrtSc, Flameshot, GNOME Screenshot)
2. Focus Qwen Studio chat input
3. Ctrl+V → image should upload immediately ✅
4. Copy text → Ctrl+V → text should paste normally ✅
5. Copy file from file manager → Ctrl+V → file should upload ✅
