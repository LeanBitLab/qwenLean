# Build Qwen Desktop on GitHub Actions

You can now build AppImage and DEB packages automatically using GitHub Actions.

## Option 1: Automatic Build on Tag Push

Push a version tag to trigger the release workflow:

```bash
git tag v2.0.1
git push origin v2.0.1
```

This will:
- Build both AppImage and DEB packages
- Create a draft GitHub Release with the artifacts attached
- Upload artifacts for direct download

## Option 2: Manual Trigger (No Tag Needed)

1. Go to your repository on GitHub
2. Click **Actions** tab
3. Select **Release** workflow
4. Click **Run workflow** button
5. Choose branch (usually `main`)
6. Click **Run workflow**

The workflow will run and produce:
- `.AppImage` file (portable, no installation needed)
- `.deb` file (for Debian/Ubuntu systems)

## Download Built Artifacts

After the workflow completes:

### From Release Page (Tag builds only):
- Go to **Releases** section
- Open the draft release created by the workflow
- Download AppImage or DEB files

### From Workflow Run (All builds):
- Go to **Actions** → Select the workflow run
- Scroll to **Artifacts** section
- Download `qwen-desktop-appimage` or `qwen-desktop-deb`

## Install After Download

### AppImage:
```bash
chmod +x Qwen-Desktop-*.AppImage
./Qwen-Desktop-*.AppImage
```

### DEB:
```bash
sudo dpkg -i qwen-desktop-linux_*.amd64.deb
sudo apt-get install -f  # Fix dependencies if needed
```

## Protocol Handler Registration (For Login Fix)

After installing, register the protocol handler for proper login flow:

```bash
# For DEB installation
xdg-mime default qwen-desktop.desktop x-scheme-handler/qwen

# For AppImage (create desktop file first if needed)
xdg-mime default appimagekit_qwen-desktop.desktop x-scheme-handler/qwen

update-desktop-database ~/.local/share/applications
```

Then restart Qwen Desktop and try signing in again.
