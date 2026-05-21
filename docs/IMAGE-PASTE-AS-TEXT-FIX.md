# Fix: Images Pasting as Text Instead of Uploading

**Version:** v2.2.3  
**Date:** 2026-05-21  
**File changed:** `electron-bridge.js`

---

## 🐛 Problem

When pasting an image from outside the app (e.g., screenshot, browser copy), the image was being inserted as **text** instead of being **uploaded** as an image file.

## 🔍 Root Cause

The Tauri v2 `clipboard-manager` plugin's `readImage()` returns image data in **two possible formats** depending on the platform:

1. **Base64-encoded PNG string** in the `data` field (most common on Linux/WebKitGTK)
2. **Raw RGBA pixel bytes** in the `rgba` field (Uint8Array or number array)

The original code assumed the data was always raw RGBA pixels:

```js
var pixels = imageData.data || imageData.rgba;
var pixelArray = pixels instanceof Uint8Array ? pixels : new Uint8Array(pixels);
```

When `data` was a **base64 string**, `new Uint8Array(string)` created an **empty/invalid array** (Uint8Array constructor treats string as length, not data). This caused:

1. Canvas `putImageData` with empty pixels → transparent/empty PNG
2. The empty PNG file was injected but the web app couldn't process it
3. Meanwhile, `readImage()` might throw or return invalid data → `.catch` fallback ran `readText()` → pasted the image URL/filename as text

## ✅ Fix

Rewrote `handleImagePaste()` to detect and handle **both** formats:

### Case 1: Base64 PNG string (most common)
```js
if (imageData.data && typeof imageData.data === 'string') {
  var blob = base64ToBlob(imageData.data, 'image/png');
  var file = new File([blob], fileName, { type: 'image/png' });
  injectImageFile(file);
}
```

Added `base64ToBlob()` helper that:
- Strips `data:image/png;base64,` prefix if present
- Decodes base64 via `atob()`
- Converts to `Uint8Array` → `Blob`

### Case 2: Raw RGBA pixels (fallback)
```js
if (imageData.rgba || imageData.data) {
  // Canvas approach: createImageData → putImageData → toBlob
}
```

### Added logging for debugging
```js
console.log('[ElectronBridge] Image data format:', {
  hasData: !!imageData.data,
  hasRgba: !!imageData.rgba,
  dataType: typeof imageData.data,
  rgbaLength: imageData.rgba ? imageData.rgba.length : 'N/A'
});
```

### Refactored injection logic
Extracted `injectImageFile()` as a reusable helper (same 3 methods: `#filesUpload` → `#dropzone-container` → synthetic paste event).

## 📝 Changes Summary

| What | Why |
|------|-----|
| Added `base64ToBlob()` helper | Decode base64 PNG strings to Blobs |
| Added `injectImageFile()` helper | Reusable file injection (3 fallback methods) |
| Rewrote `handleImagePaste()` | Detect base64 vs RGBA format and handle both |
| Added format detection logging | Helps debug future clipboard issues |

## 🧪 Testing

1. Take a screenshot → Ctrl+V → image should upload ✅
2. Copy image from browser → Ctrl+V → image should upload ✅
3. Copy text → Ctrl+V → text should paste ✅
4. Copy image file path → Ctrl+V → text should paste ✅
