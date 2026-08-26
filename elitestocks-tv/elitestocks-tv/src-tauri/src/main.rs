#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod mpv;
mod xtream;

use mpv::PlaylistItem;
use serde_json::Value;
use std::sync::Mutex as StdMutex;
use std::time::Duration;
use tauri::{Emitter, Manager, State};
use xtream::XtreamSession;

#[derive(Default)]
struct AppState {
    session: StdMutex<Option<XtreamSession>>,
}

// ---------- Auth ----------

#[tauri::command]
async fn xtream_login(
    state: State<'_, AppState>,
    server: String,
    username: String,
    password: String,
) -> Result<Value, String> {
    let server = normalize_server(&server);
    let session = XtreamSession {
        server,
        username,
        password,
    };
    let info = session.authenticate().await.map_err(|e| e.to_string())?;
    *state.session.lock().unwrap() = Some(session);
    Ok(info)
}

#[tauri::command]
fn xtream_logout(state: State<'_, AppState>) {
    *state.session.lock().unwrap() = None;
}

fn normalize_server(s: &str) -> String {
    let s = s.trim();
    let s = if s.starts_with("http://") || s.starts_with("https://") {
        s.to_string()
    } else {
        format!("http://{}", s)
    };
    s.trim_end_matches('/').to_string()
}

fn get_session(state: &State<'_, AppState>) -> Result<XtreamSession, String> {
    state
        .session
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "Not signed in".to_string())
}

// ---------- Catalog ----------

#[tauri::command]
async fn get_live_categories(state: State<'_, AppState>) -> Result<Value, String> {
    get_session(&state)?
        .get_live_categories()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_live_streams(
    state: State<'_, AppState>,
    category_id: Option<String>,
) -> Result<Value, String> {
    get_session(&state)?
        .get_live_streams(category_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_vod_categories(state: State<'_, AppState>) -> Result<Value, String> {
    get_session(&state)?
        .get_vod_categories()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_vod_streams(
    state: State<'_, AppState>,
    category_id: Option<String>,
) -> Result<Value, String> {
    get_session(&state)?
        .get_vod_streams(category_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_vod_info(state: State<'_, AppState>, vod_id: String) -> Result<Value, String> {
    get_session(&state)?
        .get_vod_info(&vod_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_series_categories(state: State<'_, AppState>) -> Result<Value, String> {
    get_session(&state)?
        .get_series_categories()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_series_list(
    state: State<'_, AppState>,
    category_id: Option<String>,
) -> Result<Value, String> {
    get_session(&state)?
        .get_series(category_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_series_info(state: State<'_, AppState>, series_id: String) -> Result<Value, String> {
    get_session(&state)?
        .get_series_info(&series_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn build_stream_url(
    state: State<'_, AppState>,
    kind: String,
    stream_id: String,
    ext: String,
) -> Result<String, String> {
    let session = get_session(&state)?;
    Ok(session.stream_url(&kind, &stream_id, &ext))
}

// ---------- Player (mpv) ----------

#[tauri::command]
async fn player_play(
    app: tauri::AppHandle,
    items: Vec<PlaylistItem>,
) -> Result<(), String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?;

    mpv::spawn_mpv(&resource_dir, &items)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }

    // Watch for mpv exiting (user closed the player) and notify the frontend
    // so it can show the browse UI again.
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(700)).await;
            if mpv::poll_exited().await {
                if let Some(w) = handle.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
                let _ = handle.emit("player-closed", ());
                break;
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn player_command(cmd: Vec<Value>) -> Result<Value, String> {
    mpv::command(cmd).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn player_set_property(name: String, value: Value) -> Result<Value, String> {
    mpv::set_property(&name, value)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn player_get_property(name: String) -> Result<Value, String> {
    mpv::get_property(&name).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn player_stop(app: tauri::AppHandle) -> Result<(), String> {
    mpv::stop_mpv().await;
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
}

// ---------- Window chrome (custom titlebar since decorations are disabled) ----------

#[tauri::command]
fn window_minimize(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.minimize();
    }
}

#[tauri::command]
fn window_toggle_maximize(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_maximized().unwrap_or(false) {
            let _ = w.unmaximize();
        } else {
            let _ = w.maximize();
        }
    }
}

#[tauri::command]
fn window_close(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.close();
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            xtream_login,
            xtream_logout,
            get_live_categories,
            get_live_streams,
            get_vod_categories,
            get_vod_streams,
            get_vod_info,
            get_series_categories,
            get_series_list,
            get_series_info,
            build_stream_url,
            player_play,
            player_command,
            player_set_property,
            player_get_property,
            player_stop,
            window_minimize,
            window_toggle_maximize,
            window_close,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let handle = window.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    mpv::stop_mpv().await;
                    let _ = handle;
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running EliteStocks TV");
}
