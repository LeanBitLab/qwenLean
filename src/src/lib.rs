mod mcp;
mod settings;
mod window;
mod events;
mod dialogs;
mod tray;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    {
        if std::env::var("GDK_BACKEND").is_err() {
            unsafe { std::env::set_var("GDK_BACKEND", "x11") };
        }
    }

    let electron_bridge = include_str!("../electron-bridge.js");

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            log::info!("[SingleInstance] Second instance launched with args: {:?}", args);
            if let Some(url) = args.iter().find(|a| a.starts_with("qwen://")) {
                log::info!("[SingleInstance] Deep link from second instance: {}", url);
                let app = app.clone();
                let url = url.clone();
                tauri::async_runtime::spawn(async move {
                    window::handle_deep_link_url(&app, &url).await;
                });
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_mcp_bridge::init())
        .setup(move |app| {
            use std::sync::Arc;
            use tokio::sync::Mutex;
            let state: mcp::McpState = Arc::new(Mutex::new(None));
            app.manage(state);
            events::setup_event_forwarding(app.handle());

            #[cfg(desktop)]
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    window::setup_deep_link(&handle).await;
                });
            }

            // Clear ALL storage and inject qwen-core BEFORE page loads
            let pre_load_script = r#"
                (function() {
                    try {
                        localStorage.clear();
                        sessionStorage.clear();
                        localStorage.setItem("LOCAL_MCP_SERVER", JSON.stringify([{
                            id: "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                            name: "qwen-core",
                            description: "Core MCP server with 28 tools for file operations, search, bash execution, time management, and autonomous agent capabilities. Provides filesystem access, git operations, and sequential thinking for AI-assisted development.",
                            type: "stdio",
                            params: { command: "npx", args: ["-y", "qwen-core"] },
                            enabled: true,
                            default: false,
                            connectionStatus: "available",
                            errorMessage: "",
                            tools: []
                        }]));
                    } catch(e) {}
                })();
            "#;

            // Inject settings updates tab
            let settings_script = r##"
                (function() {
                    function injectUpdatesTab() {
                        if (document.getElementById('qwen-updates-tab')) return;

                        var sidebarContent = document.querySelector('.setting-side-bar-group-content');
                        if (!sidebarContent) return;

                        var tab = document.createElement('div');
                        tab.id = 'qwen-updates-tab';
                        tab.className = 'setting-side-bar-group-content-item';
                        tab.setAttribute('data-spm-anchor-id', '');
                        tab.innerHTML = '<span role="img" class="anticon"><svg width="1em" height="1em" fill="currentColor" aria-hidden="true" focusable="false" class=""><use xlink:href="#icon-line-information-circle"></use></svg></span><div class="setting-side-bar-group-content-item-title" data-spm-anchor-id="">Updates</div>';
                        sidebarContent.appendChild(tab);

                        var panel = document.createElement('div');
                        panel.id = 'qwen-updates-panel';
                        panel.style.cssText = 'display:none;flex:1;padding:32px;overflow-y:auto;background:#0f0f14;color:#e5e7eb;font-family:system-ui,sans-serif;';
                        panel.innerHTML = '<div style="max-width:500px;"><h2 style="margin:0 0 24px;font-size:24px;font-weight:600;">Updates</h2><div id="qwen-update-content"><p style="color:#9ca3af;">Checking for updates...</p></div></div>';

                        var mainContent = document.querySelector('.setting-content');
                        if (mainContent) {
                            mainContent.appendChild(panel);
                        } else {
                            document.body.appendChild(panel);
                        }

                        tab.onclick = function() {
                            sidebarContent.querySelectorAll('.setting-side-bar-group-content-item').forEach(function(t) {
                                t.classList.remove('selected');
                            });
                            tab.classList.add('selected');

                            var contentArea = document.querySelector('.setting-content');
                            if (contentArea) {
                                contentArea.querySelectorAll(':scope > div').forEach(function(d) {
                                    if (d.id !== 'qwen-updates-panel') d.style.display = 'none';
                                });
                            }

                            panel.style.display = 'block';
                            checkForUpdatesUI();
                        };
                    }

                    async function checkForUpdatesUI() {
                        var content = document.getElementById('qwen-update-content');
                        if (!content) return;
                        content.innerHTML = '<p style="color:#9ca3af;">Checking for updates...</p>';

                        try {
                            var info = await window.__TAURI__.core.invoke('get_update_info');
                            if (info.available) {
                                content.innerHTML = '<div style="background:#1a1a2e;border:1px solid #2d2d44;border-radius:12px;padding:24px;">' +
                                    '<div style="display:flex;align-items:center;gap:12px;margin-bottom:16px;">' +
                                    '<span style="font-size:32px;">&#x1f504;</span>' +
                                    '<div><div style="font-size:18px;font-weight:600;">Update Available</div>' +
                                    '<div style="color:#9ca3af;font-size:14px;">Version ' + info.latest_version + '</div></div></div>' +
                                    '<div style="margin-bottom:16px;"><span style="color:#9ca3af;">Current:</span> <strong>' + info.current_version + '</strong></div>' +
                                    '<div style="background:#16213e;padding:12px;border-radius:8px;margin-bottom:16px;font-size:13px;max-height:150px;overflow-y:auto;">' +
                                    info.release_notes.replace(/\n/g, '<br>') + '</div>' +
                                    '<button id="qwen-install-btn" style="width:100%;padding:12px;background:#22c55e;color:#fff;border:none;border-radius:8px;font-size:14px;font-weight:600;cursor:pointer;">Install Update</button></div>';
                                document.getElementById('qwen-install-btn').onclick = async function() {
                                    var btn = document.getElementById('qwen-install-btn');
                                    btn.textContent = 'Installing...';
                                    btn.disabled = true;
                                    btn.style.opacity = '0.7';
                                    try {
                                        await window.__TAURI__.core.invoke('install_update', { type: 'appimage' });
                                        btn.textContent = 'Installed! Restarting...';
                                        btn.style.background = '#3b82f6';
                                    } catch(e) {
                                        btn.textContent = 'Failed: ' + e;
                                        btn.style.background = '#ef4444';
                                        btn.style.opacity = '1';
                                    }
                                };
                            } else {
                                content.innerHTML = '<div style="background:#1a1a2e;border:1px solid #2d2d44;border-radius:12px;padding:32px;text-align:center;">' +
                                    '<div style="font-size:48px;margin-bottom:12px;">&#x2705;</div>' +
                                    '<div style="font-size:18px;font-weight:600;margin-bottom:4px;">You\'re up to date!</div>' +
                                    '<div style="color:#9ca3af;font-size:14px;">Version ' + info.current_version + '</div></div>';
                            }
                        } catch(e) {
                            content.innerHTML = '<div style="background:#1a1a2e;border:1px solid #2d2d44;border-radius:12px;padding:24px;">' +
                                '<div style="color:#ef4444;font-size:14px;">Error checking for updates: ' + e + '</div></div>';
                        }
                    }

                    // Poll for settings page (SPA navigation doesn't trigger DOM mutations reliably)
                    var checkInterval = setInterval(function() {
                        if (window.location.href.indexOf('/settings') !== -1) {
                            injectUpdatesTab();
                        }
                    }, 500);

                    // Also listen for SPA navigation events
                    window.addEventListener('popstate', function() {
                        if (window.location.href.indexOf('/settings') !== -1) {
                            setTimeout(injectUpdatesTab, 300);
                        }
                    });

                    // Initial check
                    if (window.location.href.indexOf('/settings') !== -1) {
                        setTimeout(injectUpdatesTab, 500);
                    }
                })();
            "##;

            let combined_script = format!("{}\n{}\n{}", pre_load_script, electron_bridge, settings_script);

            let icon_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("icons")
                .join("icon.png");
            let icon = if icon_path.exists() {
                let bytes = std::fs::read(&icon_path).ok();
                bytes.and_then(|b| {
                    image::load_from_memory(&b)
                        .ok()
                        .map(|img| img.into_rgba8())
                        .map(|img| {
                            let (w, h) = img.dimensions();
                            tauri::image::Image::new_owned(img.into_raw(), w, h)
                        })
                })
            } else {
                None
            };

            let url = "https://chat.qwen.ai".parse().unwrap();
            let window_builder = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External(url),
            )
            .title("Qwen Studio")
            .inner_size(1280.0, 840.0)
            .min_inner_size(400.0, 600.0)
            .center()
            .resizable(true)
            .decorations(true)
            .accept_first_mouse(false)
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 AliDesktop(QWENCHAT/2.2.0)")
            .initialization_script(&combined_script);

            let _main_window = if let Some(icon) = icon {
                window_builder.icon(icon).unwrap().build()?
            } else {
                window_builder.build()?
            };

            log::info!("[App] Main window created with electron bridge");

            tray::setup_tray(app.handle())?;
            log::info!("[App] System tray initialized");

            // Add menu button to Tauri's existing GTK HeaderBar on Linux
            #[cfg(target_os = "linux")]
            {
                use gtk::prelude::{BinExt, Cast, ContainerExt, CssProviderExt, GtkMenuItemExt, GtkWindowExt, HeaderBarExt, MenuButtonExt, MenuShellExt, WidgetExt};
                use gtk::{CssProvider, EventBox, Label, Menu, MenuButton, STYLE_PROVIDER_PRIORITY_APPLICATION};

                // Style the HeaderBar
                let css = r#"
                    headerbar, .titlebar {
                        min-height: 38px;
                        padding: 0 6px;
                        background: #2c2c2c;
                        color: #ffffff;
                    }
                    headerbar button, .titlebar button {
                        min-height: 28px;
                        min-width: 32px;
                        padding: 4px 6px;
                        background: transparent;
                        color: #ffffff;
                        border-radius: 4px;
                    }
                    headerbar button:hover, .titlebar button:hover {
                        background: rgba(255,255,255,0.12);
                    }
                    #menu-btn-label {
                        font-size: 16px;
                        padding: 0 4px;
                    }
                "#;
                let provider = CssProvider::new();
                if let Err(e) = provider.load_from_data(css.as_bytes()) {
                    log::warn!("[Titlebar] CSS load failed: {:?}", e);
                } else {
                    gtk::StyleContext::add_provider_for_screen(
                        &gtk::gdk::Screen::default().expect("no screen"),
                        &provider,
                        STYLE_PROVIDER_PRIORITY_APPLICATION,
                    );
                }

                if let Some(window) = app.get_webview_window("main") {
                    let app_handle = app.handle().clone();
                    if let Ok(gtk_window) = window.gtk_window() {
                        // Get Tauri's existing titlebar (EventBox wrapping HeaderBar)
                        if let Some(titlebar_widget) = gtk_window.titlebar() {
                            // Tauri wraps HeaderBar in an EventBox
                            if let Ok(event_box) = titlebar_widget.downcast::<EventBox>() {
                                if let Some(child) = event_box.child() {
                                    if let Ok(header_bar) = child.downcast::<gtk::HeaderBar>() {
                                        // Build menu
                                        let menu = Menu::new();

                                        let file_item = gtk::MenuItem::with_label("File");
                                        let file_menu = Menu::new();
                                        let minimize_mi = gtk::MenuItem::with_label("Minimize");
                                        let maximize_mi = gtk::MenuItem::with_label("Maximize");
                                        let quit_mi = gtk::MenuItem::with_label("Quit");
                                        file_menu.append(&minimize_mi);
                                        file_menu.append(&maximize_mi);
                                        file_menu.append(&quit_mi);
                                        file_item.set_submenu(Some(&file_menu));
                                        menu.append(&file_item);

                                        let edit_item = gtk::MenuItem::with_label("Edit");
                                        let edit_menu = Menu::new();
                                        let undo_mi = gtk::MenuItem::with_label("Undo");
                                        let redo_mi = gtk::MenuItem::with_label("Redo");
                                        let cut_mi = gtk::MenuItem::with_label("Cut");
                                        let copy_mi = gtk::MenuItem::with_label("Copy");
                                        let paste_mi = gtk::MenuItem::with_label("Paste");
                                        let select_all_mi = gtk::MenuItem::with_label("Select All");
                                        edit_menu.append(&undo_mi);
                                        edit_menu.append(&redo_mi);
                                        edit_menu.append(&cut_mi);
                                        edit_menu.append(&copy_mi);
                                        edit_menu.append(&paste_mi);
                                        edit_menu.append(&select_all_mi);
                                        edit_item.set_submenu(Some(&edit_menu));
                                        menu.append(&edit_item);

                                        let view_item = gtk::MenuItem::with_label("View");
                                        let view_menu = Menu::new();
                                        let devtools_mi = gtk::MenuItem::with_label("Toggle DevTools");
                                        let reload_mi = gtk::MenuItem::with_label("Reload");
                                        view_menu.append(&devtools_mi);
                                        view_menu.append(&reload_mi);
                                        view_item.set_submenu(Some(&view_menu));
                                        menu.append(&view_item);

                                        let win_mi = gtk::MenuItem::with_label("Window");
                                        let win_menu = Menu::new();
                                        let fullscreen_mi = gtk::MenuItem::with_label("Toggle Fullscreen");
                                        win_menu.append(&fullscreen_mi);
                                        win_mi.set_submenu(Some(&win_menu));
                                        menu.append(&win_mi);

                                        let help_item = gtk::MenuItem::with_label("Help");
                                        let help_menu = Menu::new();
                                        let docs_mi = gtk::MenuItem::with_label("Documentation");
                                        let github_mi = gtk::MenuItem::with_label("GitHub");
                                        let about_mi = gtk::MenuItem::with_label("About");
                                        let update_mi = gtk::MenuItem::with_label("Check for Updates");
                                        help_menu.append(&docs_mi);
                                        help_menu.append(&github_mi);
                                        help_menu.append(&update_mi);
                                        help_menu.append(&about_mi);
                                        help_item.set_submenu(Some(&help_menu));
                                        menu.append(&help_item);

                                        menu.show_all();

                                        // Menu button
                                        let menu_btn = MenuButton::new();
                                        menu_btn.set_popup(Some(&menu));
                                        let menu_label = Label::new(Some("☰"));
                                        menu_label.set_widget_name("menu-btn-label");
                                        menu_btn.add(&menu_label);
                                        menu_btn.show_all();

                                        // Add menu button to existing HeaderBar
                                        header_bar.pack_start(&menu_btn);
                                        header_bar.show_all();

                                        // Menu item signals
                                        let w = window.clone();
                                        minimize_mi.connect_activate(move |_| { let _ = w.minimize(); });

                                        let w = window.clone();
                                        maximize_mi.connect_activate(move |_| {
                                            if w.is_maximized().unwrap_or(false) { let _ = w.unmaximize(); }
                                            else { let _ = w.maximize(); }
                                        });

                                        let ah = app_handle.clone();
                                        quit_mi.connect_activate(move |_| { ah.exit(0); });

                                        let w = window.clone();
                                        devtools_mi.connect_activate(move |_| {
                                            if w.is_devtools_open() { w.close_devtools(); } else { w.open_devtools(); }
                                        });

                                        let w = window.clone();
                                        reload_mi.connect_activate(move |_| { let _ = w.eval("location.reload();"); });

                                        let w = window.clone();
                                        fullscreen_mi.connect_activate(move |_| {
                                            let f = w.is_fullscreen().unwrap_or(false);
                                            let _ = w.set_fullscreen(!f);
                                        });

                                        docs_mi.connect_activate(|_| { let _ = open::that("https://chat.qwen.ai"); });
                                        github_mi.connect_activate(|_| { let _ = open::that("https://github.com/youssefvdel/qwen-studio"); });

                                        let ah = app_handle.clone();
                                        update_mi.connect_activate(move |_| {
                                            let a = ah.clone();
                                            tauri::async_runtime::spawn(async move { check_for_updates(&a, true).await; });
                                        });

                                        let ver = env!("CARGO_PKG_VERSION").to_string();
                                        let w = window.clone();
                                        about_mi.connect_activate(move |_| {
                                            let _ = w.eval(format!(r#"alert("Qwen Studio v{}");"#, ver));
                                        });

                                        log::info!("[Titlebar] Menu button added to existing HeaderBar");
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Check for updates on startup
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                check_for_updates(&app_handle, false).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window::get_app_version,
            window::get_platform_info,
            window::open_devtool,
            window::toggle_hidden_devtools,
            window::minimize_window,
            window::maximize_window,
            window::close_window,
            window::open_external_link,
            dialogs::show_native_dialog,
            dialogs::request_file_access,
            mcp::mcp_client_connect,
            mcp::mcp_client_close,
            mcp::mcp_client_tool_list,
            mcp::mcp_client_tool_call,
            mcp::mcp_client_get_config,
            mcp::mcp_client_update_config,
            settings::get_setting,
            settings::set_setting,
            window::switch_theme,
            window::switch_ln,
            window::update_title_bar_for_system_theme,
            window::get_language,
            events::webview_loaded,
            install_update,
            get_update_info,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::WindowEvent {
                label,
                event: tauri::WindowEvent::CloseRequested { api, .. },
                ..
            } = event
            {
                if label == "main" {
                    api.prevent_close();
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
            }
        });
}

async fn check_for_updates(app: &tauri::AppHandle, manual: bool) {
    use tauri_plugin_updater::UpdaterExt;

    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            log::warn!("[Updater] Updater not available: {}", e);
            return;
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            log::info!("[Updater] Update available: {}", update.version);
            let install_type = detect_install_type();
            let version = update.version.clone();
            let notes = update.body.as_deref().unwrap_or("No release notes").replace('\n', "\\n").replace('"', "\\\"");

            if let Some(window) = app.get_webview_window("main") {
                let msg = match install_type {
                    InstallType::AppImage => format!(
                        "Update {} is available (AppImage).\n\n{}\n\nDownload and install now?",
                        version, notes
                    ),
                    InstallType::Deb => format!(
                        "Update {} is available (.deb).\n\n{}\n\nDownload and install? (requires sudo)",
                        version, notes
                    ),
                    InstallType::Rpm => format!(
                        "Update {} is available (.rpm).\n\n{}\n\nDownload and install? (requires sudo)",
                        version, notes
                    ),
                    InstallType::Unknown => format!(
                        "Update {} is available.\n\n{}\n\nOpen GitHub releases page?",
                        version, notes
                    ),
                };

                let _ = window.eval(format!(
                    r#"if(confirm("{}")) {{ window.__TAURI__.core.invoke('install_update', {{ type: '{}' }}); }}"#,
                    msg.replace('\n', "\\n"),
                    install_type.as_str()
                ));
            }
        }
        Ok(None) => {
            if manual {
                log::info!("[Updater] Already up to date");
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.eval(r#"alert("You are running the latest version.");"#);
                }
            }
        }
        Err(e) => {
            log::error!("[Updater] Update check failed: {}", e);
            if manual {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.eval(format!(
                        r#"alert("Update check failed:\n{}");"#,
                        e.to_string().replace('"', "\\\"")
                    ));
                }
            }
        }
    }
}

#[derive(Debug)]
enum InstallType {
    AppImage,
    Deb,
    Rpm,
    Unknown,
}

impl InstallType {
    fn as_str(&self) -> &'static str {
        match self {
            InstallType::AppImage => "appimage",
            InstallType::Deb => "deb",
            InstallType::Rpm => "rpm",
            InstallType::Unknown => "unknown",
        }
    }
}

fn detect_install_type() -> InstallType {
    if std::env::var("APPIMAGE").is_ok() {
        return InstallType::AppImage;
    }

    let output = std::process::Command::new("dpkg")
        .arg("-s")
        .arg("qwen-studio")
        .output();
    if let Ok(o) = output {
        if o.status.success() {
            return InstallType::Deb;
        }
    }

    let output = std::process::Command::new("rpm")
        .arg("-q")
        .arg("qwen-studio")
        .output();
    if let Ok(o) = output {
        if o.status.success() {
            return InstallType::Rpm;
        }
    }

    InstallType::Unknown
}

#[tauri::command]
async fn install_update(app: tauri::AppHandle, r#type: String) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?.ok_or("No update available")?;

    match r#type.as_str() {
        "appimage" => {
            log::info!("[Updater] Installing AppImage update");
            update
                .download_and_install(|_bytes, _total| {}, || {})
                .await
                .map_err(|e| e.to_string())?;
            log::info!("[Updater] AppImage update installed successfully");
        }
        "deb" => {
            let url = format!(
                "https://github.com/youssefvdel/qwen-studio/releases/download/v{}/qwen-studio_{}_amd64.deb",
                update.version, update.version
            );
            log::info!("[Updater] Downloading DEB: {}", url);
            let tmp_path = format!("/tmp/qwen-studio_{}.deb", update.version);
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(format!("curl -L '{}' -o '{}' && pkexec dpkg -i '{}'", url, tmp_path, tmp_path))
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                return Err("Failed to install DEB package".to_string());
            }
            log::info!("[Updater] DEB update installed");
            app.restart();
        }
        "rpm" => {
            let url = format!(
                "https://github.com/youssefvdel/qwen-studio/releases/download/v{}/qwen-studio-{}.x86_64.rpm",
                update.version, update.version
            );
            log::info!("[Updater] Downloading RPM: {}", url);
            let tmp_path = format!("/tmp/qwen-studio-{}.x86_64.rpm", update.version);
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(format!("curl -L '{}' -o '{}' && pkexec rpm -Uvh '{}'", url, tmp_path, tmp_path))
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                return Err("Failed to install RPM package".to_string());
            }
            log::info!("[Updater] RPM update installed");
            app.restart();
        }
        _ => {
            let _ = open::that("https://github.com/youssefvdel/qwen-studio/releases/latest");
            return Ok(());
        }
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct UpdateInfo {
    current_version: String,
    available: bool,
    latest_version: String,
    release_notes: String,
}

#[tauri::command]
async fn get_update_info(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    use tauri_plugin_updater::UpdaterExt;

    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => Ok(UpdateInfo {
            current_version: current_version.clone(),
            available: true,
            latest_version: update.version.clone(),
            release_notes: update.body.as_deref().unwrap_or("No release notes").to_string(),
        }),
        Ok(None) => Ok(UpdateInfo {
            current_version: current_version.clone(),
            available: false,
            latest_version: current_version,
            release_notes: String::new(),
        }),
        Err(e) => Err(e.to_string()),
    }
}


