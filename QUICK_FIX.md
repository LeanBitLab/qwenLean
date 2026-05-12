# Quick Fix: Qwen Desktop Login Not Working (AppImage)

## Problem
When clicking "Sign In", the browser opens but doesn't show "Open with Qwen Desktop" option because the protocol handler isn't registered.

## Solution - Copy & Paste These Commands

### Step 1: Find your desktop file
Run this command and note the filename it shows:
```bash
ls ~/.local/share/applications/ | grep -i qwen
```

**Expected output:** Something like `appimagekit_qwen-desktop.desktop` or `qwen-desktop.desktop`

---

### Step 2: Register the protocol handler
**Replace `<FILENAME>` with the actual name from Step 1** (without the `.desktop` extension):

```bash
xdg-mime default <FILENAME>.desktop x-scheme-handler/qwen
```

**Example:** If Step 1 showed `appimagekit_qwen-desktop.desktop`, run:
```bash
xdg-mime default appimagekit_qwen-desktop.desktop x-scheme-handler/qwen
```

---

### Step 3: Update desktop database
```bash
update-desktop-database ~/.local/share/applications
```

---

### Step 4: Verify it worked
```bash
xdg-mime query default x-scheme-handler/qwen
```

**Expected output:** Your desktop filename (e.g., `appimagekit_qwen-desktop.desktop`)

If it shows your filename, registration succeeded! ✓

---

### Step 5: Test the login
1. **Close Qwen Desktop completely** (right-click tray icon → Quit)
2. **Reopen Qwen Desktop**
3. Click **"Sign In"**
4. Browser opens → complete authentication
5. Browser should now show **"Open with Qwen Desktop"** option
6. Click **"Open"** → You're logged in! ✓

---

## Still Not Working?

### Check if the .desktop file has the MimeType entry:
```bash
cat ~/.local/share/applications/<FILENAME>.desktop | grep MimeType
```

**Should show:** `MimeType=x-scheme-handler/qwen;`

If it's missing, add it manually:
```bash
echo "MimeType=x-scheme-handler/qwen;" >> ~/.local/share/applications/<FILENAME>.desktop
update-desktop-database ~/.local/share/applications
```

### For KDE Users:
Go to **System Settings → Applications → Default Applications** and ensure `qwen` protocol is mapped to Qwen Desktop.

### For GNOME Users:
Go to **Settings → Default Applications** and check if Qwen Desktop appears.

---

## One-Line Fix (if you know your desktop filename)
```bash
xdg-mime default appimagekit_qwen-desktop.desktop x-scheme-handler/qwen && update-desktop-database ~/.local/share/applications
```
