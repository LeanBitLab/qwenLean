# Clipboard Image Paste Thread Panic Fix

**Date:** May 22, 2026  
**Version:** 2.2.3  
**File Changed:** `src/window.rs`

---

## 🐛 The Problem

When pasting an image (e.g., a screenshot), the app crashed with:

```
thread 'tokio-rt-worker' panicked at glib-0.18.5/src/source.rs:345:14:
default main context already acquired by another thread
```

This panic happened on **every image paste attempt**.

---

## 🔍 Root Cause

GTK clipboard operations (`gtk::Clipboard::get()`, `wait_for_image()`) **must run on the GTK main thread** — the thread that owns the GLib main context.

Tauri command handlers run on **tokio worker threads**, which do NOT own the GLib main context.

The previous code used `glib::idle_add_local()`:

```rust
// ❌ BROKEN — runs on tokio worker thread
glib::idle_add_local(move || {
    let clipboard = gtk::Clipboard::get(...);
    let pixbuf = clipboard.wait_for_image();
    // ...
});
```

`idle_add_local()` requires the **calling thread to own** the default main context. Since the main GTK thread already owns it, this panics:

> *"Failed to acquire ownership of main context, already acquired by another thread"*

---

## ✅ The Fix

Replaced `glib::idle_add_local()` with `glib::MainContext::default().invoke()`:

```rust
// ✅ FIXED — invoke() is thread-safe, works from any thread
glib::MainContext::default().invoke(move || {
    let clipboard = gtk::Clipboard::get(...);
    let pixbuf = clipboard.wait_for_image();
    // ...
});
```

### Key Differences

| API | Thread Safety | Requires Main Context Ownership? |
|-----|--------------|----------------------------------|
| `glib::idle_add_local()` | ❌ Caller's thread only | ✅ Yes — panics if not owned |
| `glib::MainContext::default().invoke()` | ✅ Thread-safe | ❌ No — schedules on main thread |

Also removed `glib::ControlFlow::Break` since `invoke()` takes a `FnOnce` (one-shot), not a recurring idle callback.

---

## 📝 Changes

| File | What Changed | Why |
|------|-------------|-----|
| `src/window.rs` | `idle_add_local()` → `MainContext::default().invoke()` | Thread-safe clipboard read from tokio worker |
| `src/window.rs` | Removed `glib::ControlFlow::Break` | Not needed for one-shot invoke |

---

## ✅ Verification

```
cargo check → 0 errors, 0 warnings ✅
```
