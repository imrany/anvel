// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use actix_cors::Cors;
use actix_web::{http, web, App, HttpServer};
use dirs;
use std::{net::Ipv4Addr, path::PathBuf};
use tauri::{AppHandle, Manager};

#[path = "./routes.rs"]
mod routes;
use routes::{
    directory_content, download, get_ip_address, open_file, ping, pong, receive, send, websocket,
    AppState,
};

fn get_root_directory() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        Some(PathBuf::from("/"))
    }

    #[cfg(windows)]
    {
        Some(PathBuf::from(r"C:\"))
    }

    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

#[tauri::command]
async fn serve_anvel() -> Result<(), String> {
    let download_dir = dirs::download_dir().ok_or("Failed to get download directory")?;
    let shared_dir = download_dir.join("Anvel shared");

    tokio::fs::create_dir_all(&shared_dir)
        .await
        .map_err(|e| format!("Failed to create shared directory: {}", e))?;

    let app_state = web::Data::new(AppState {
        root_dir: get_root_directory().ok_or("Failed to get root directory")?,
        home_dir: dirs::home_dir().ok_or("Failed to get home directory")?,
        download_dir: dirs::download_dir().ok_or("Failed to get download directory")?,
        shared_dir,
    });

    let port: u16 = 8082;
    let ipv4: (Ipv4Addr, u16) = (Ipv4Addr::new(0, 0, 0, 0), port);

    // Spawn the server in a separate task instead of awaiting it directly
    tokio::task::spawn_local(async move {
        let server = HttpServer::new(move || {
            let cors = Cors::default()
                .allow_any_origin()
                .allowed_methods(vec!["GET", "POST", "PATCH", "PUT", "DELETE"])
                .allowed_headers(vec![
                    http::header::AUTHORIZATION,
                    http::header::ACCEPT,
                    http::header::CONTENT_TYPE,
                ])
                .max_age(3600);

            App::new().wrap(cors).app_data(app_state.clone()).service(
                web::scope("/api")
                    .service(directory_content)
                    .service(get_ip_address)
                    .service(open_file)
                    .service(send)
                    .service(receive)
                    .service(download)
                    .service(ping)
                    .service(pong)
                    .service(websocket),
            )
        })
        .bind(ipv4)
        .expect(&format!("Failed to bind server to {}:{}", ipv4.0, ipv4.1))
        .run();

        if let Err(e) = server.await {
            eprintln!("Server error: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn open_window(
    app: AppHandle,
    file_path: String,
    label: String,
    title: String,
) -> Result<String, String> {
    // Check if window already exists
    if let Some(existing_window) = app.get_webview_window(&label) {
        // Focus and show existing window
        existing_window
            .set_focus()
            .map_err(|e| format!("Failed to focus window: {}", e))?;
        existing_window
            .show()
            .map_err(|e| format!("Failed to show window: {}", e))?;
        return Ok(format!("Window '{}' already exists and was focused", label));
    }

    // Create new webview window (Tauri v2)
    tauri::webview::WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::App(file_path.into()),
    )
    .title(&title)
    .build()
    .map_err(|e| format!("Failed to create window: {}", e))?;

    Ok(format!("Window '{}' opened successfully", label))
}

#[tauri::command]
async fn close_window(app: AppHandle, label: String) -> Result<String, String> {
    if let Some(window) = app.get_webview_window(&label) {
        window
            .close()
            .map_err(|e| format!("Failed to close window: {}", e))?;
        Ok(format!("Window '{}' closed successfully", label))
    } else {
        Err(format!("Window '{}' not found", label))
    }
}

#[tauri::command]
async fn toggle_window(
    app: AppHandle,
    file_path: String,
    label: String,
    title: String,
) -> Result<String, String> {
    if let Some(window) = app.get_webview_window(&label) {
        // Window exists, close it
        window
            .close()
            .map_err(|e| format!("Failed to close window: {}", e))?;
        Ok(format!("Window '{}' closed", label))
    } else {
        // Window doesn't exist, create it
        tauri::webview::WebviewWindowBuilder::new(
            &app,
            &label,
            tauri::WebviewUrl::App(file_path.into()),
        )
        .title(&title)
        .build()
        .map_err(|e| format!("Failed to create window: {}", e))?;
        Ok(format!("Window '{}' opened", label))
    }
}

#[tauri::command]
async fn hide_window(app: AppHandle, label: String) -> Result<String, String> {
    if let Some(window) = app.get_webview_window(&label) {
        window
            .hide()
            .map_err(|e| format!("Failed to hide window: {}", e))?;
        Ok(format!("Window '{}' hidden", label))
    } else {
        Err(format!("Window '{}' not found", label))
    }
}

#[tauri::command]
async fn show_window(app: AppHandle, label: String) -> Result<String, String> {
    if let Some(window) = app.get_webview_window(&label) {
        window
            .show()
            .map_err(|e| format!("Failed to show window: {}", e))?;
        window
            .set_focus()
            .map_err(|e| format!("Failed to focus window: {}", e))?;
        Ok(format!("Window '{}' shown", label))
    } else {
        Err(format!("Window '{}' not found", label))
    }
}

#[tauri::command]
async fn minimize_window(app: AppHandle, label: String) -> Result<String, String> {
    if let Some(window) = app.get_webview_window(&label) {
        window
            .minimize()
            .map_err(|e| format!("Failed to minimize window: {}", e))?;
        Ok(format!("Window '{}' minimized", label))
    } else {
        Err(format!("Window '{}' not found", label))
    }
}

#[tauri::command]
async fn maximize_window(app: AppHandle, label: String) -> Result<String, String> {
    if let Some(window) = app.get_webview_window(&label) {
        window
            .maximize()
            .map_err(|e| format!("Failed to maximize window: {}", e))?;
        Ok(format!("Window '{}' maximized", label))
    } else {
        Err(format!("Window '{}' not found", label))
    }
}

#[tauri::command]
async fn unmaximize_window(app: AppHandle, label: String) -> Result<String, String> {
    if let Some(window) = app.get_webview_window(&label) {
        window
            .unmaximize()
            .map_err(|e| format!("Failed to unmaximize window: {}", e))?;
        Ok(format!("Window '{}' unmaximized", label))
    } else {
        Err(format!("Window '{}' not found", label))
    }
}

#[tauri::command]
async fn set_window_size(
    app: AppHandle,
    label: String,
    width: f64,
    height: f64,
) -> Result<String, String> {
    if let Some(window) = app.get_webview_window(&label) {
        window
            .set_size(tauri::Size::Physical(tauri::PhysicalSize {
                width: width as u32,
                height: height as u32,
            }))
            .map_err(|e| format!("Failed to set window size: {}", e))?;
        Ok(format!(
            "Window '{}' resized to {}x{}",
            label, width, height
        ))
    } else {
        Err(format!("Window '{}' not found", label))
    }
}

#[tauri::command]
async fn set_window_position(
    app: AppHandle,
    label: String,
    x: f64,
    y: f64,
) -> Result<String, String> {
    if let Some(window) = app.get_webview_window(&label) {
        window
            .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                x: x as i32,
                y: y as i32,
            }))
            .map_err(|e| format!("Failed to set window position: {}", e))?;
        Ok(format!("Window '{}' moved to ({}, {})", label, x, y))
    } else {
        Err(format!("Window '{}' not found", label))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            greet,
            serve_anvel,
            open_window,
            close_window,
            toggle_window,
            hide_window,
            show_window,
            minimize_window,
            maximize_window,
            unmaximize_window,
            set_window_size,
            set_window_position
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
