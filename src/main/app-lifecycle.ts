/**
 * App Lifecycle — protocol handler, deep links, quit state, app flags
 *
 * Responsibilities:
 * - configureApp() — Sets all app.commandLine flags (GPU, sandbox, platform hints).
 *   Called BEFORE app.whenReady() so flags take effect.
 * - setupProtocolHandler() — Registers qwen:// as a custom protocol handler.
 *   On Linux AppImage, patches the auto-generated .desktop file to add the MIME type.
 * - handleDeepLink() — Parses qwen://open?token=xxx URLs and sends the auth token
 *   to the renderer via IPC event.
 * - isQuitting/setQuitting — Global quit state used by window-manager for
 *   close-to-tray behavior vs actual quit.
 */

import { app, BrowserWindow, dialog } from "electron";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";
import { execSync, exec } from "child_process";
import { promisify } from "util";
import * as http from "http";

const execAsync = promisify(exec);

// Local HTTP server for OAuth callback fallback (unused, kept for future)
let authCallbackServer: http.Server | null = null;
const AUTH_CALLBACK_PORT = 14920;

// === Quit State ===
/** Tracks whether the app is intentionally quitting (tray Quit) vs hiding to tray. */
let _isQuitting = false;

/** Check if app is in quit state. Used by window-manager close handler. */
export function isQuitting(): boolean {
  return _isQuitting;
}

/** Set the quit state. Called from tray "Quit" menu item. */
export function setQuitting(value: boolean): void {
  _isQuitting = value;
}

/**
 * Register qwen:// protocol handler for AppImage on Linux.
 * AppImage creates its .desktop file dynamically on mount, so we
 * retry registration after a delay to ensure the file exists.
 */
async function registerAppImageProtocolHandler(): Promise<void> {
  if (process.platform !== "linux") return;
  if (!app.isPackaged) {
    console.log("[Protocol] Skipping registration in dev mode");
    return;
  }

  const desktopDir = path.join(os.homedir(), ".local", "share", "applications");
  console.log("[Protocol] Starting registration...");

  async function tryRegister(): Promise<boolean> {
    try {
      const files = await fs.promises.readdir(desktopDir);
      console.log(
        "[Protocol] Found desktop files:",
        files.filter((f) => f.toLowerCase().includes("qwen")),
      );

      const appimageDesktop = files.find(
        (f) =>
          f.toLowerCase().includes("qwen") ||
          f.toLowerCase().includes("qwen-desktop"),
      );

      if (!appimageDesktop) {
        console.log("[Protocol] No .desktop file found yet");
        return false;
      }

      const desktopFile = path.join(desktopDir, appimageDesktop);
      console.log("[Protocol] Found:", desktopFile);

      let content = await fs.promises.readFile(desktopFile, "utf-8");
      console.log(
        "[Protocol] Current MimeType:",
        content.match(/MimeType=.*/)?.[0] || "none",
      );

      if (content.includes("x-scheme-handler/qwen")) {
        console.log("[Protocol] Already registered");
        return true;
      }

      if (content.includes("MimeType=")) {
        content = content.replace(
          /(MimeType=[^;]*);/,
          "$1;x-scheme-handler/qwen;",
        );
      } else {
        content += "\nMimeType=x-scheme-handler/qwen;\n";
      }

      await fs.promises.writeFile(desktopFile, content);
      console.log("[Protocol] Patched:", desktopFile);

      await execAsync(`xdg-mime default ${appimageDesktop} x-scheme-handler/qwen`);
      console.log("[Protocol] xdg-mime registered");

      await execAsync(`update-desktop-database ${desktopDir}`);
      console.log("[Protocol] Desktop database updated");

      const { stdout } = await execAsync(`xdg-mime query default x-scheme-handler/qwen`);
      const handler = stdout.trim();
      console.log("[Protocol] Verified handler:", handler);
      return true;
    } catch (error) {
      console.error("[Protocol] Registration attempt failed:", error);
      return false;
    }
  }

  // Try immediately, then retry every 2 seconds for 10 seconds
  if (await tryRegister()) return;

  let attempts = 0;
  const runRetry = async () => {
    attempts++;
    console.log(`[Protocol] Retry attempt ${attempts}/5...`);

    // Wait 2 seconds
    await new Promise((resolve) => setTimeout(resolve, 2000));

    if (await tryRegister()) {
      return;
    }

    if (attempts < 5) {
      await runRetry();
    } else {
      console.log(
        "[Protocol] Automatic registration failed. Manual steps required:",
      );
      console.log("1. Find your .desktop file in ~/.local/share/applications/");
      console.log(
        "2. Add 'MimeType=x-scheme-handler/qwen;' to the [Desktop Entry] section",
      );
      console.log(
        "3. Run: xdg-mime default <filename>.desktop x-scheme-handler/qwen",
      );
      console.log("4. Run: update-desktop-database ~/.local/share/applications");

      // Show dialog to user with instructions
      setTimeout(() => {
        dialog.showMessageBox({
          type: "warning",
          title: "Protocol Handler Registration",
          message:
            "Qwen Desktop couldn't automatically register as the default handler for qwen:// links.",
          detail:
            "To enable login via browser, please run these commands in terminal:\n\n" +
            "1. Find your desktop file:\n   ls ~/.local/share/applications/ | grep qwen\n\n" +
            "2. Register the protocol (replace <filename> with actual name):\n   xdg-mime default <filename>.desktop x-scheme-handler/qwen\n\n" +
            "3. Update desktop database:\n   update-desktop-database ~/.local/share/applications",
          buttons: ["OK"],
        });
      }, 1000);
    }
  };

  runRetry();
}

/**
 * Configure app command-line flags.
 * Call this ONCE at startup before app.whenReady().
 * Replaces module-level side effects.
 */
export function configureApp(): void {
  // Wayland/X11 platform support (Fedora KDE defaults to Wayland)
  app.commandLine.appendSwitch("enable-features", "UseOzonePlatform");
  app.commandLine.appendSwitch("ozone-platform-hint", "x11");

  // Disable GPU acceleration to prevent crashes on Linux
  app.commandLine.appendSwitch("disable-gpu");
  app.commandLine.appendSwitch("disable-gpu-compositing");
  app.commandLine.appendSwitch("disable-software-rasterizer");
  app.commandLine.appendSwitch("no-sandbox");
  app.commandLine.appendSwitch("disable-dev-shm-usage");
  app.commandLine.appendSwitch("disable-gpu-sandbox");
  app.commandLine.appendSwitch("use-gl", "swiftshader");
  app.commandLine.appendSwitch("ignore-gpu-blocklist");
  app.commandLine.appendSwitch("disable-features", "VizDisplayCompositor");

  // Debug flags - Enable remote debugging for chrome-devtools-mcp
  app.commandLine.appendSwitch("enable-logging");
  app.commandLine.appendSwitch("v", "1");
  app.commandLine.appendSwitch("remote-debugging-port", "9222");
  app.commandLine.appendSwitch("remote-allow-origins", "*");
}

/**
 * Handle qwen:// deep link URLs.
 */
export function handleDeepLink(
  url: string,
  mainWindow: BrowserWindow | null,
): void {
  console.log("[DeepLink] Handling URL:", url);
  const urlObj = new URL(url);
  if (urlObj.pathname === "/open") {
    const token = urlObj.searchParams.get("token");
    if (token) {
      console.log("[DeepLink] Auth token received");
      mainWindow?.webContents.send("event_from_main", {
        type: "auth_token",
        payload: { token },
      });
    }
  }
}

/**
 * Setup protocol handler (qwen://).
 */
export function setupProtocolHandler(handlers: {
  onDeepLink: (url: string) => void;
}): void {
  // FIRST: Register .desktop file and MIME handler for AppImage
  registerAppImageProtocolHandler().catch((err) => {
    console.error("[Protocol] registerAppImageProtocolHandler failed:", err);
  });

  // THEN: Set as default protocol client (uses the .desktop file)
  if (process.defaultApp) {
    if (process.argv.length >= 2) {
      app.setAsDefaultProtocolClient("qwen", process.execPath, [
        process.argv[1],
      ]);
    }
  } else {
    app.setAsDefaultProtocolClient("qwen");
  }

  // Handle qwen:// URLs (macOS)
  app.on("open-url", (event, url) => {
    event.preventDefault();
    handlers.onDeepLink(url);
  });

  // Also check for qwen:// in initial command line args (first launch)
  const qwenUrl = process.argv.find((arg) => arg.startsWith("qwen://"));
  if (qwenUrl) {
    console.log("[Protocol] Deep link found in startup args:", qwenUrl);
    handlers.onDeepLink(qwenUrl);
  }
}
