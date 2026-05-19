use tauri::{
    menu::{Menu, MenuItem, SubmenuBuilder},
    AppHandle, Manager, Runtime,
};

pub fn create_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let file_menu = SubmenuBuilder::with_id(app, "file", "File")
        .item(&MenuItem::with_id(
            app,
            "minimize",
            "Minimize",
            true,
            Some("Ctrl+M"),
        )?)
        .item(&MenuItem::with_id(
            app,
            "maximize",
            "Maximize",
            true,
            Some("Ctrl+Shift+M"),
        )?)
        .separator()
        .item(&MenuItem::with_id(
            app,
            "quit",
            "Quit",
            true,
            Some("Ctrl+Q"),
        )?)
        .build()?;

    let edit_menu = SubmenuBuilder::with_id(app, "edit", "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let view_menu = SubmenuBuilder::with_id(app, "view", "View")
        .item(&MenuItem::with_id(
            app,
            "toggle_devtools",
            "Toggle DevTools",
            true,
            Some("F12"),
        )?)
        .separator()
        .item(&MenuItem::with_id(
            app,
            "reload",
            "Reload",
            true,
            Some("Ctrl+R"),
        )?)
        .build()?;

    let window_menu = SubmenuBuilder::with_id(app, "window", "Window")
        .minimize()
        .separator()
        .item(&MenuItem::with_id(
            app,
            "toggle_fullscreen",
            "Toggle Fullscreen",
            true,
            Some("F11"),
        )?)
        .build()?;

    let help_menu = SubmenuBuilder::with_id(app, "help", "Help")
        .item(&MenuItem::with_id(
            app,
            "open_docs",
            "Documentation",
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            app,
            "open_github",
            "GitHub Repository",
            true,
            None::<&str>,
        )?)
        .separator()
        .item(&MenuItem::with_id(
            app,
            "about",
            "About Qwen Studio",
            true,
            None::<&str>,
        )?)
        .build()?;

    Menu::with_items(
        app,
        &[&file_menu, &edit_menu, &view_menu, &window_menu, &help_menu],
    )
}

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "minimize" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.minimize();
            }
        }
        "maximize" => {
            if let Some(window) = app.get_webview_window("main") {
                if window.is_maximized().unwrap_or(false) {
                    let _ = window.unmaximize();
                } else {
                    let _ = window.maximize();
                }
            }
        }
        "quit" => {
            app.exit(0);
        }
        "toggle_devtools" => {
            if let Some(window) = app.get_webview_window("main") {
                if window.is_devtools_open() {
                    window.close_devtools();
                } else {
                    window.open_devtools();
                }
            }
        }
        "reload" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval("window.location.reload();");
            }
        }
        "toggle_fullscreen" => {
            if let Some(window) = app.get_webview_window("main") {
                let is_fullscreen = window.is_fullscreen().unwrap_or(false);
                let _ = window.set_fullscreen(!is_fullscreen);
            }
        }
        "open_docs" => {
            let _ = open::that("https://chat.qwen.ai");
        }
        "open_github" => {
            let _ = open::that("https://github.com/youssefvdel/qwen-studio");
        }
        "about" => {
            let version = env!("CARGO_PKG_VERSION");
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval(format!(
                    r#"alert("Qwen Studio v{version}\n\nOpen-source Qwen AI desktop client with MCP support.\nMIT License");"#
                ));
            }
        }
        _ => {}
    }
}
