# Qwen Desktop Linux - Login Issue Fix

## Problem
When clicking "Sign In" with the AppImage version, the browser opens but doesn't show "Open with Qwen Desktop" option. Instead, it only shows "Open with selected app" and Qwen Desktop isn't listed.

## Root Cause
The AppImage needs to register itself as the default handler for the `qwen://` protocol. This registration happens automatically when the app starts, but can fail if:
1. The `.desktop` file hasn't been created yet
2. The MIME type database isn't updated
3. The AppImage was moved after first run

## Solution

### Option 1: Automatic (App Should Do This)
The app now automatically:
1. Finds its `.desktop` file in `~/.local/share/applications/`
2. Adds `MimeType=x-scheme-handler/qwen;` to it
3. Registers with `xdg-mime`
4. Updates the desktop database

If automatic registration fails, you'll see a dialog with manual instructions.

### Option 2: Manual Registration

Run these commands in your terminal:

#### Step 1: Find your desktop file
```bash
ls ~/.local/share/applications/ | grep qwen
```

You should see something like:
- `appimagekit_qwen-desktop.desktop`
- `qwen-desktop.desktop`
- Or similar

#### Step 2: Register the protocol handler
Replace `<filename>` with the actual name from Step 1:

```bash
xdg-mime default <filename>.desktop x-scheme-handler/qwen
```

Example:
```bash
xdg-mime default appimagekit_qwen-desktop.desktop x-scheme-handler/qwen
```

#### Step 3: Update the desktop database
```bash
update-desktop-database ~/.local/share/applications
```

#### Step 4: Verify registration
```bash
xdg-mime query default x-scheme-handler/qwen
```

This should output your desktop file name.

### Option 3: For .deb Package Users

If you installed via `.deb` package, the protocol handler should be pre-configured. If not:

```bash
sudo update-desktop-database
xdg-mime default qwen-desktop.desktop x-scheme-handler/qwen
```

## Testing

After registration:
1. Close Qwen Desktop completely (right-click tray icon → Quit)
2. Reopen Qwen Desktop
3. Click "Sign In"
4. Browser should open and after authentication, prompt to open with Qwen Desktop
5. Click "Open" and you should be logged in

## Troubleshooting

### Still not working?

1. **Check if .desktop file exists:**
   ```bash
   ls -la ~/.local/share/applications/ | grep qwen
   ```

2. **Check if MimeType is present in .desktop file:**
   ```bash
   grep MimeType ~/.local/share/applications/<filename>.desktop
   ```
   
   Should show: `MimeType=x-scheme-handler/qwen;`

3. **Try running Qwen Desktop from terminal:**
   ```bash
   /path/to/Qwen-Desktop.AppImage
   ```
   
   Look for `[Protocol]` messages in the output.

4. **Check system logs:**
   ```bash
   journalctl -f | grep qwen
   ```

### KDE/GNOME Specific

**KDE Plasma:**
- Go to System Settings → Applications → Default Applications
- Look for "Custom Protocol Handlers"
- Ensure `qwen` is mapped to Qwen Desktop

**GNOME:**
- Go to Settings → Default Applications
- Check if Qwen Desktop appears in the list

## For Developers

To test protocol handling during development:

```bash
# Test if protocol is registered
xdg-mime query default x-scheme-handler/qwen

# Test opening a qwen:// URL
xdg-open "qwen://open?token=test123"
```

The app should receive the token and log it to console.
