use tauri::{Emitter, Manager};
use url::Url;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tauri::command]
pub async fn get_app_version() -> Result<String, String> {
    Ok(APP_VERSION.to_string())
}

#[tauri::command]
pub async fn get_platform_info() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH
    }))
}

#[tauri::command]
pub async fn open_devtool(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.open_devtools();
    }
    Ok(())
}

#[tauri::command]
pub async fn toggle_hidden_devtools(app: tauri::AppHandle) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_devtools_open() {
            window.close_devtools();
            log::info!("DevTools closed");
            Ok(false)
        } else {
            window.open_devtools();
            log::info!("DevTools opened");
            Ok(true)
        }
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn minimize_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.minimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn maximize_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_maximized().unwrap_or(false) {
            window.unmaximize().map_err(|e| e.to_string())?;
        } else {
            window.maximize().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn close_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(target_os = "macos")]
        {
            window.hide().map_err(|e| e.to_string())?;
        }
        #[cfg(not(target_os = "macos"))]
        {
            window.close().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn open_external_link(app: tauri::AppHandle, url: String) -> Result<bool, String> {
    if url.starts_with("qwen://") {
        log::info!(
            "[DeepLink] Caught qwen:// URL in open_external_link: {}",
            url
        );
        handle_deep_link_url(&app, &url).await;
        return Ok(true);
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Ok(false);
    }

    log::info!("[Link] Opening URL in system browser: {}", url);
    open::that(&url).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn switch_theme(app: tauri::AppHandle, theme: String) -> Result<(), String> {
    log::info!("[Theme] switch_theme: {}", theme);
    app.emit(
        "event_from_main",
        serde_json::json!({
            "type": "theme_changed",
            "payload": theme
        }),
    )
    .map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        if let Some(window) = app.get_webview_window("main") {
            let is_dark = theme == "dark";
            window
                .set_theme(Some(if is_dark {
                    tauri::Theme::Dark
                } else {
                    tauri::Theme::Light
                }))
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn switch_ln(app: tauri::AppHandle, ln: String) -> Result<(), String> {
    log::info!("[Language] switch_ln: {}", ln);
    app.emit(
        "event_from_main",
        serde_json::json!({
            "type": "language_changed",
            "payload": ln
        }),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn update_title_bar_for_system_theme(
    app: tauri::AppHandle,
    is_dark: bool,
) -> Result<(), String> {
    app.emit(
        "event_from_main",
        serde_json::json!({
            "type": "system_theme_changed",
            "payload": is_dark
        }),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_language() -> Result<String, String> {
    Ok("en-US".to_string())
}

pub async fn setup_deep_link(app: &tauri::AppHandle) {
    use tauri_plugin_deep_link::DeepLinkExt;

    #[cfg(any(target_os = "linux", all(debug_assertions, target_os = "windows")))]
    {
        if let Err(e) = app.deep_link().register_all() {
            log::error!("[DeepLink] Failed to register schemes: {}", e);
        }
    }

    log::info!("[DeepLink] Protocol handler registered (qwen://)");

    let app_handle = app.clone();
    if let Ok(Some(urls)) = app_handle.deep_link().get_current() {
        for url in urls {
            log::info!("[DeepLink] App started with URL: {}", url);
            let app = app_handle.clone();
            let url_str = url.to_string();
            tauri::async_runtime::spawn(async move {
                handle_deep_link_url(&app, &url_str).await;
            });
        }
    }
}

pub async fn handle_deep_link_url(app: &tauri::AppHandle, url: &str) {
    match Url::parse(url) {
        Ok(parsed) => {
            let host = parsed.host_str().unwrap_or("");
            let query: std::collections::HashMap<_, _> = parsed.query_pairs().collect();

            match host {
                "open" | "auth" | "login" | "callback" => {
                    let token = query
                        .get("token")
                        .or_else(|| query.get("code"))
                        .or_else(|| query.get("sid"))
                        .or_else(|| query.get("ticket"));

                    if let Some(token) = token {
                        log::info!("[DeepLink] Auth token received, length: {}", token.len());
                        if let Some(window) = app.get_webview_window("main") {
                            let token = token.to_string();

                            let _ = window.show();
                            let _ = window.set_focus();

                            let current_url =
                                window.url().map(|u| u.to_string()).unwrap_or_default();
                            log::info!("[DeepLink] Current webview URL: {}", current_url);

                            if !current_url.contains("chat.qwen.ai") {
                                log::info!("[DeepLink] Navigating to chat.qwen.ai first");
                                let _ =
                                    window.eval("window.location.href = 'https://chat.qwen.ai';");
                                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                            }

                            let _ = window.eval(format!(
                                r#"(function() {{
                                    try {{
                                        document.cookie = "token={token}; domain=.qwen.ai; path=/; max-age=2592000";
                                        document.cookie = "sid={token}; domain=.qwen.ai; path=/; max-age=2592000";
                                        document.cookie = "ticket={token}; domain=.qwen.ai; path=/; max-age=2592000";
                                        localStorage.setItem("token", "{token}");
                                        localStorage.setItem("sid", "{token}");
                                        localStorage.setItem("ticket", "{token}");
                                        localStorage.setItem("auth_token", "{token}");
                                        localStorage.setItem("qwen_auth_token", "{token}");
                                        sessionStorage.setItem("token", "{token}");
                                        sessionStorage.setItem("sid", "{token}");
                                        console.log("[DeepLink] Token and cookies injected");
                                    }} catch(e) {{
                                        console.error("[DeepLink] Injection failed:", e);
                                    }}
                                }})();"#
                            ));

                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            let _ = window.eval("window.location.reload();");
                        }
                    } else {
                        log::warn!(
                            "[DeepLink] No token/code/sid/ticket in URL. Query params: {:?}",
                            query.keys().collect::<Vec<_>>()
                        );
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
                _ => {
                    log::info!("[DeepLink] Unknown host: {}", host);
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        }
        Err(e) => {
            log::error!("[DeepLink] Failed to parse URL: {}", e);
        }
    }
}
