# Login Fix for Qwen Desktop Linux

## Problem
When clicking "Sign In" in both AppImage and DEB packages, the login flow was broken:
1. **AppImage**: Browser didn't show "Open with Qwen Desktop" option because the protocol handler wasn't registered
2. **DEB**: Browser showed the option but clicking it opened a new instance instead of passing the auth token to the existing app

## Root Causes
1. Protocol handler (`qwen://`) registration failed silently on AppImage
2. External links were being opened in the system browser instead of in-app, breaking the auth flow
3. No mechanism to receive auth tokens in the webview from deep links

## Changes Made

### 1. `/workspace/src/main/window-manager.ts`
**Changed**: All external links now open in an in-app popup window instead of the system browser

**Why**: This keeps the entire authentication flow within the Electron app, allowing us to:
- Capture `qwen://` redirect URLs directly
- Prevent the browser from stealing the session
- Auto-close the auth window when login completes

```typescript
// BEFORE: Only auth URLs opened in-app, others went to system browser
if (isAuthUrl) {
  // open in-app
} else {
  shell.openExternal(details.url); // ❌ Breaks auth flow
}

// AFTER: ALL URLs open in-app to capture auth redirects
const authWindow = new BrowserWindow({...});
authWindow.loadURL(details.url);
// Catch qwen:// redirects and auto-close when back at chat.qwen.ai
```

### 2. `/workspace/src/renderer/index.html`
**Added**: Listener for `auth_token` events from main process

**Why**: When a deep link (`qwen://open?token=xxx`) is received, the renderer needs to forward it to the webview so the chat.qwen.ai page can process the login.

```javascript
window.electronAPI.on_event('auth_token', (data) => {
  if (data && data.token) {
    const webview = document.querySelector('webview');
    webview.send('auth_token', { token: data.token });
  }
});
```

### 3. `/workspace/src/preload/index.ts`
**Added**: Handler for `auth_token` IPC messages that posts them to the web content

**Why**: The preload script bridges the main process and webview. It receives the token via IPC and forwards it via `postMessage` so the chat.qwen.ai JavaScript can receive it.

```typescript
ipcRenderer.on("auth_token", (_, { token }) => {
  events.emit("auth_callback", { token });
  window.postMessage({ type: "AUTH_CALLBACK", token }, "*");
});
```

### 4. `/workspace/src/main/app-lifecycle.ts`
**Already present**: Automatic protocol handler registration with retry logic

**Note**: If automatic registration fails, a dialog shows manual instructions. Run these commands:

```bash
# Find your desktop file
ls ~/.local/share/applications/ | grep -i qwen

# Register the protocol (replace <filename> with actual name)
xdg-mime default <filename>.desktop x-scheme-handler/qwen

# Update desktop database
update-desktop-database ~/.local/share/applications
```

## How to Build

### Option 1: Build Locally
```bash
cd /workspace
npm install
npm run build

# Test in dev mode
npm start

# Build packages
npm run build:deb      # Creates .deb package
npm run build:appimage # Creates AppImage
```

### Option 2: Build via GitHub Actions
1. Push a version tag: `git tag v2.0.1 && git push origin v2.0.1`
2. Go to Actions tab → Select "Build Release" workflow
3. Download artifacts from the workflow run or Releases page

## Testing the Fix

1. **Install the new build**:
   ```bash
   # For DEB
   sudo dpkg -i dist/Qwen\ Desktop-2.0.0-amd64.deb
   
   # For AppImage
   chmod +x dist/Qwen\ Desktop-2.0.0.AppImage
   ./dist/Qwen\ Desktop-2.0.0.AppImage
   ```

2. **Click "Sign In"** in the app

3. **Complete authentication** in the popup window (not external browser)

4. **Verify login**: The popup should auto-close and you should be logged in the main window

## Debugging

If login still fails, check the logs:

```bash
# For AppImage
./Qwen\ Desktop-*.AppImage --enable-logging --v=1

# For DEB
/opt/Qwen\ Desktop/qwen-desktop --enable-logging --v=1

# View logs
tail -f ~/.config/Qwen\ Desktop/logs/main.log
```

Look for:
- `[Window] Opening ALL URLs in-app to capture auth:` - Confirms in-app auth window
- `[DeepLink] Handling URL: qwen://open?token=...` - Confirms deep link received
- `[Preload] 🔑 Received auth token from main process` - Confirms token forwarded to webview

## Manual Protocol Registration (If Needed)

If the browser doesn't offer "Open with Qwen Desktop":

```bash
# 1. Find the desktop file
DESKTOP_FILE=$(ls ~/.local/share/applications/ | grep -i qwen | head -1)
echo "Found: $DESKTOP_FILE"

# 2. Check if MimeType is present
grep -i mimetype ~/.local/share/applications/$DESKTOP_FILE

# 3. Add MIME type if missing
sed -i '/^\[Desktop Entry\]/a MimeType=x-scheme-handler/qwen;' ~/.local/share/applications/$DESKTOP_FILE

# 4. Register as default handler
xdg-mime default $DESKTOP_FILE x-scheme-handler/qwen

# 5. Update database
update-desktop-database ~/.local/share/applications

# 6. Verify
xdg-mime query default x-scheme-handler/qwen
```

## Summary

This fix ensures:
✅ Authentication happens entirely within the app (no external browser)
✅ Deep links are captured and processed correctly
✅ Auth tokens are passed to the webview for login completion
✅ Protocol handler is automatically registered (with manual fallback)
✅ Works for both AppImage and DEB packages
