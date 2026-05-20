mod dialogs;
mod events;
mod mcp;
mod settings;
mod tray;
mod window;

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
                            params: { command: "npx", args: ["-y", "@youssefvdel/qwen-core"] },
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
                    var injected = false;

                    function injectUpdatesTab() {
                        if (injected) return;

                        var sidebarContent = document.querySelector('.setting-side-bar-group-content');
                        if (!sidebarContent) return;

                        var aboutTab = null;
                        var items = sidebarContent.querySelectorAll('.setting-side-bar-group-content-item');
                        for (var i = 0; i < items.length; i++) {
                            var title = items[i].querySelector('.setting-side-bar-group-content-item-title');
                            if (title && title.textContent.trim() === 'About') {
                                aboutTab = items[i];
                                break;
                            }
                        }

                        var tab = document.createElement('div');
                        tab.id = 'qwen-updates-tab';
                        tab.className = 'setting-side-bar-group-content-item';
                        tab.setAttribute('data-spm-anchor-id', '');
                        tab.innerHTML = '<span role="img" class="anticon"><svg width="1em" height="1em" fill="currentColor" aria-hidden="true" focusable="false" class=""><use xlink:href="#icon-line-download-02"></use></svg></span><div class="setting-side-bar-group-content-item-title" data-spm-anchor-id="">Updates</div>';

                        if (aboutTab) {
                            sidebarContent.insertBefore(tab, aboutTab);
                        } else {
                            sidebarContent.appendChild(tab);
                        }

                        var panel = document.createElement('div');
                        panel.id = 'qwen-updates-panel';
                        panel.className = 'qwen-chat-setting-general';
                        panel.style.display = 'none';
                        panel.innerHTML =
                            '<div class="setting-content-title">' +
                                '<div class="setting-content-title-title">Updates</div>' +
                            '</div>' +
                            '<div id="qwen-update-content" style="max-width:500px;"></div>';

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

                        var observer = new MutationObserver(function(mutations) {
                            for (var i = 0; i < mutations.length; i++) {
                                var m = mutations[i];
                                if (m.attributeName === 'class' && m.target !== tab) {
                                    if (m.target.classList.contains('selected')) {
                                        tab.classList.remove('selected');
                                        panel.style.display = 'none';
                                    }
                                }
                            }
                        });

                        sidebarContent.querySelectorAll('.setting-side-bar-group-content-item').forEach(function(t) {
                            observer.observe(t, { attributes: true, attributeFilter: ['class'] });
                        });

                        injected = true;
                    }

                    function cardStyle() {
                        return 'background:rgba(255,255,255,0.03);border:1px solid rgba(255,255,255,0.06);border-radius:12px;padding:20px;margin-bottom:16px;';
                    }

                    function progressBarStyle() {
                        return 'width:100%;height:4px;background:rgba(255,255,255,0.08);border-radius:2px;overflow:hidden;margin:12px 0;';
                    }

                    function progressFillStyle(pct) {
                        return 'height:100%;width:' + pct + '%;background:rgb(97,92,237);border-radius:2px;transition:width 0.3s ease;';
                    }

                    function btnStyle() {
                        return 'padding:8px 20px;background:rgb(97,92,237);color:rgb(247,248,252);border:none;border-radius:8px;font-size:14px;font-weight:500;cursor:pointer;height:36px;font-family:inherit;transition:background 0.15s ease;';
                    }

                    function btnSecondaryStyle() {
                        return 'padding:8px 20px;background:transparent;color:rgb(247,248,252);border:1px solid rgba(255,255,255,0.12);border-radius:8px;font-size:14px;font-weight:500;cursor:pointer;height:36px;font-family:inherit;transition:background 0.15s ease;';
                    }

                    async function checkForUpdatesUI() {
                        var content = document.getElementById('qwen-update-content');
                        if (!content) return;
                        content.innerHTML =
                            '<div style="' + cardStyle() + '">' +
                                '<div class="qwen-chat-setting-content-item-label">Checking for updates...</div>' +
                            '</div>';

                        try {
                            var info = await window.__TAURI__.core.invoke('get_update_info');
                            if (info.available) {
                                content.innerHTML =
                                    '<div style="' + cardStyle() + '">' +
                                        '<div style="display:flex;align-items:center;gap:10px;margin-bottom:12px;">' +
                                            '<span style="font-size:20px;">&#x2b07;</span>' +
                                            '<div class="qwen-chat-setting-content-item-label" style="font-size:16px;font-weight:600;">Update Available</div>' +
                                        '</div>' +
                                        '<div style="color:rgba(255,255,255,0.5);font-size:13px;margin-bottom:16px;">v' + info.latest_version + ' &mdash; you are on v' + info.current_version + '</div>' +
                                        (info.release_notes ? '<div style="background:rgba(255,255,255,0.04);padding:12px;border-radius:8px;margin-bottom:16px;font-size:13px;color:rgba(255,255,255,0.6);max-height:120px;overflow-y:auto;line-height:1.5;">' + info.release_notes.replace(/\n/g, '<br>') + '</div>' : '') +
                                        '<div style="display:flex;gap:8px;">' +
                                            '<button id="qwen-install-btn" style="' + btnStyle() + '">Download &amp; Install</button>' +
                                        '</div>' +
                                    '</div>';
                                document.getElementById('qwen-install-btn').onclick = async function() {
                                    startInstallUI();
                                };
                            } else {
                                content.innerHTML =
                                    '<div style="' + cardStyle() + 'text-align:center;">' +
                                        '<div style="font-size:32px;margin-bottom:8px;">&#x2705;</div>' +
                                        '<div class="qwen-chat-setting-content-item-label" style="font-size:16px;font-weight:600;margin-bottom:4px;">You\'re up to date</div>' +
                                        '<div style="color:rgba(255,255,255,0.5);font-size:13px;">Running v' + info.current_version + '</div>' +
                                    '</div>';
                            }
                        } catch(e) {
                            content.innerHTML =
                                '<div style="' + cardStyle() + '">' +
                                    '<div style="color:rgb(239,68,68);font-size:13px;">Could not check for updates: ' + e + '</div>' +
                                '</div>';
                        }
                    }

                    async function startInstallUI() {
                        var content = document.getElementById('qwen-update-content');
                        if (!content) return;

                        content.innerHTML =
                            '<div style="' + cardStyle() + '">' +
                                '<div style="display:flex;align-items:center;gap:10px;margin-bottom:12px;">' +
                                    '<span style="font-size:20px;">&#x2b07;</span>' +
                                    '<div class="qwen-chat-setting-content-item-label" style="font-size:16px;font-weight:600;">Downloading Update</div>' +
                                '</div>' +
                                '<div style="color:rgba(255,255,255,0.5);font-size:13px;margin-bottom:4px;" id="dl-status">Starting download...</div>' +
                                '<div style="' + progressBarStyle() + '"><div id="dl-bar" style="' + progressFillStyle(0) + '"></div></div>' +
                                '<div style="color:rgba(255,255,255,0.4);font-size:12px;" id="dl-bytes">0 / 0 MB</div>' +
                            '</div>';

                        // Listen for progress events
                        var unlisten = await window.__TAURI__.event.listen('update-progress', function(event) {
                            var data = event.payload;
                            var bar = document.getElementById('dl-bar');
                            var status = document.getElementById('dl-status');
                            var bytes = document.getElementById('dl-bytes');
                            if (bar) bar.style.width = data.progress + '%';
                            if (status) status.textContent = data.status;
                            if (bytes && data.downloaded) bytes.textContent = data.downloaded + ' / ' + data.total + ' MB';
                        });

                        try {
                            await window.__TAURI__.core.invoke('install_update_with_progress');
                            unlisten();

                            // Install complete — show restart
                            content.innerHTML =
                                '<div style="' + cardStyle() + 'text-align:center;">' +
                                    '<div style="font-size:32px;margin-bottom:8px;">&#x2705;</div>' +
                                    '<div class="qwen-chat-setting-content-item-label" style="font-size:16px;font-weight:600;margin-bottom:4px;">Update Installed</div>' +
                                    '<div style="color:rgba(255,255,255,0.5);font-size:13px;margin-bottom:16px;">Restart the app to apply changes</div>' +
                                    '<button id="qwen-restart-btn" style="' + btnStyle() + '">Restart Now</button>' +
                                '</div>';
                            document.getElementById('qwen-restart-btn').onclick = async function() {
                                var btn = document.getElementById('qwen-restart-btn');
                                btn.textContent = 'Restarting...';
                                btn.disabled = true;
                                await window.__TAURI__.core.invoke('restart_app');
                            };
                        } catch(e) {
                            unlisten();
                            content.innerHTML =
                                '<div style="' + cardStyle() + '">' +
                                    '<div style="color:rgb(239,68,68);font-size:13px;">Update failed: ' + e + '</div>' +
                                    '<button id="qwen-retry-btn" style="' + btnStyle() + 'margin-top:12px;">Retry</button>' +
                                '</div>';
                            document.getElementById('qwen-retry-btn').onclick = async function() {
                                startInstallUI();
                            };
                        }
                    }

                    var checkInterval = setInterval(function() {
                        if (window.location.href.indexOf('/settings') !== -1) {
                            injectUpdatesTab();
                        }
                    }, 500);

                    window.addEventListener('popstate', function() {
                        if (window.location.href.indexOf('/settings') !== -1) {
                            setTimeout(injectUpdatesTab, 300);
                        }
                    });

                    if (window.location.href.indexOf('/settings') !== -1) {
                        setTimeout(injectUpdatesTab, 500);
                    }

                    // Global update banner — listens for update-available event
                    window.__TAURI__.event.listen('update-available', function(event) {
                        if (document.getElementById('qwen-update-banner')) return;
                        var data = event.payload;
                        var style = document.createElement('style');
                        style.id = 'qwen-banner-styles';
                        style.textContent = '#qwen-banner-go:hover { background: rgb(117, 112, 257) !important; } #qwen-banner-dismiss:hover { background: rgba(255,255,255,0.08) !important; color: rgb(247,248,252) !important; }';
                        document.head.appendChild(style);
                        var banner = document.createElement('div');
                        banner.id = 'qwen-update-banner';
                        banner.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:9999;background:rgb(46,46,51);border-bottom:1px solid rgba(255,255,255,0.08);padding:10px 20px;display:flex;align-items:center;justify-content:space-between;gap:12px;font-family:system-ui,ui-sans-serif,-apple-system,BlinkMacSystemFont,Inter,NotoSansHans,sans-serif;box-shadow:0 4px 12px rgba(0,0,0,0.3);';
                        banner.innerHTML = '<div style="display:flex;align-items:center;gap:10px;flex:1;min-width:0;">' +
                            '<span style="font-size:16px;color:rgb(97,92,237);flex-shrink:0;">&#x2b07;</span>' +
                            '<div style="min-width:0;"><div style="font-size:13px;font-weight:500;color:rgb(247,248,252);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">Update ' + data.version + ' available</div>' +
                            '<div style="font-size:11px;color:rgba(255,255,255,0.5);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">' + data.notes.substring(0, 80) + '...</div></div></div>' +
                            '<div style="display:flex;gap:6px;flex-shrink:0;">' +
                            '<button id="qwen-banner-go" style="padding:5px 14px;background:rgb(97,92,237);color:rgb(247,248,252);border:none;border-radius:6px;font-size:12px;cursor:pointer;height:26px;font-family:inherit;transition:background 0.15s ease;">Go to Settings</button>' +
                            '<button id="qwen-banner-dismiss" style="padding:5px 10px;background:transparent;color:rgba(255,255,255,0.5);border:1px solid rgba(255,255,255,0.1);border-radius:6px;font-size:12px;cursor:pointer;height:26px;transition:background 0.15s ease, color 0.15s ease;">&#x2715;</button></div>';
                        document.body.appendChild(banner);
                        document.getElementById('qwen-banner-go').addEventListener('click', function() {
                            window.location.href = 'https://chat.qwen.ai/settings';
                        });
                        document.getElementById('qwen-banner-dismiss').addEventListener('click', function() {
                            var b = document.getElementById('qwen-update-banner');
                            if (b) b.remove();
                            var s = document.getElementById('qwen-banner-styles');
                            if (s) s.remove();
                        });
                    });
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

            // Periodic update check every 4 hours
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                use tokio::time::{sleep, Duration};
                sleep(Duration::from_secs(4 * 60 * 60)).await;
                loop {
                    check_for_updates(&app_handle, false).await;
                    sleep(Duration::from_secs(4 * 60 * 60)).await;
                }
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
            install_update_with_progress,
            get_update_info,
            restart_app,
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
    use tauri::Emitter;

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
            let version = update.version.clone();
            let notes = update
                .body
                .as_deref()
                .unwrap_or("No release notes")
                .replace('\n', "\\n")
                .replace('"', "\\\"");

            let _ = app.emit(
                "update-available",
                serde_json::json!({
                    "version": version,
                    "notes": notes
                }),
            );
        }
        Ok(None) => {
            if manual {
                log::info!("[Updater] Already up to date");
            }
        }
        Err(e) => {
            log::error!("[Updater] Update check failed: {}", e);
        }
    }
}

#[tauri::command]
async fn install_update_with_progress(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    use tauri::Emitter;

    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("No update available")?;

    log::info!("[Updater] Downloading update {}", update.version);

    let mut downloaded_bytes = 0u64;
    let mut total_bytes = 0u64;

    let bytes = update
        .download(
            |chunk_len, total| {
                downloaded_bytes += chunk_len as u64;
                if total_bytes == 0 {
                    total_bytes = total.unwrap_or(0);
                }
                let pct = if total_bytes > 0 {
                    (downloaded_bytes as f64 / total_bytes as f64) * 100.0
                } else {
                    0.0
                };
                let dl_mb = downloaded_bytes as f64 / 1048576.0;
                let total_mb = total_bytes as f64 / 1048576.0;
                let _ = app.emit(
                    "update-progress",
                    serde_json::json!({
                        "phase": "download",
                        "progress": pct,
                        "downloaded": format!("{:.1}", dl_mb),
                        "total": format!("{:.1}", total_mb),
                        "status": format!("Downloading... {:.0}%", pct)
                    }),
                );
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    log::info!("[Updater] Download complete, installing...");

    let _ = app.emit(
        "update-progress",
        serde_json::json!({
            "phase": "install",
            "progress": 100.0,
            "downloaded": "",
            "total": "",
            "status": "Installing..."
        }),
    );

    update.install(bytes).map_err(|e| e.to_string())?;

    log::info!("[Updater] Update installed successfully");
    Ok(())
}

#[tauri::command]
fn restart_app(app: tauri::AppHandle) -> Result<(), String> {
    log::info!("[Updater] Restarting app");
    app.restart();
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
            release_notes: update
                .body
                .as_deref()
                .unwrap_or("No release notes")
                .to_string(),
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
