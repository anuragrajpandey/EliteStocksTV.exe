//! Spawns and controls `mpv` (bundled with the app, plus the `uosc` on-screen
//! controller skinned to match the EliteStocks TV design) and talks to it over
//! its JSON IPC protocol (https://mpv.io/manual/stable/#json-ipc) through a
//! named pipe. mpv owns its own real, GPU-accelerated fullscreen window - this
//! is the same technique used by mpv-front-end apps like mpv.net/Celluloid,
//! and avoids fragile WebView/Win32 window-compositing hacks.

use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[cfg(windows)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(windows)]
use tokio::net::windows::named_pipe::ClientOptions;

pub const PIPE_NAME: &str = r"\\.\pipe\elitestockstv-mpv";

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);
pub static MPV_CHILD: Lazy<Mutex<Option<Child>>> = Lazy::new(|| Mutex::new(None));

/// Item to hand mpv, e.g. one episode of a series.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct PlaylistItem {
    pub url: String,
    pub title: String,
}

fn mpv_executable(resource_dir: &std::path::Path) -> std::path::PathBuf {
    let bundled = resource_dir.join("mpv").join("mpv.exe");
    if bundled.exists() {
        bundled
    } else {
        std::path::PathBuf::from("mpv.exe")
    }
}

/// Launch mpv full-screen and load a playlist (single item for live TV / a
/// movie, or the full episode list for a series so "next episode" and the
/// uosc playlist menu work natively).
pub async fn spawn_mpv(
    resource_dir: &std::path::Path,
    items: &[PlaylistItem],
) -> anyhow::Result<()> {
    stop_mpv().await;
    if items.is_empty() {
        anyhow::bail!("No playable item given");
    }

    let exe = mpv_executable(resource_dir);
    let mpv_dir = exe.parent().map(|p| p.to_path_buf());

    let mut cmd = Command::new(&exe);
    cmd.arg(format!("--input-ipc-server={}", PIPE_NAME))
        .arg("--fullscreen=yes")
        .arg("--idle=yes")
        .arg("--force-window=yes")
        .arg("--keep-open=yes")
        .arg("--hwdec=auto-safe")
        .arg("--vo=gpu")
        .arg("--gpu-context=d3d11")
        .arg("--sub-auto=fuzzy")
        .arg("--cursor-autohide=1000")
        .arg("--terminal=no")
        .arg(format!("--force-media-title={}", items[0].title))
        .arg(&items[0].url);

    if let Some(dir) = &mpv_dir {
        cmd.current_dir(dir);
    }

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn()?;
    *MPV_CHILD.lock().await = Some(child);

    // give mpv a moment to open the IPC pipe
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // append the rest of the playlist (e.g. remaining episodes) so uosc's
    // playlist menu and "next episode" (playlist-next) work out of the box.
    // Per-entry title uses mpv's raw-string option syntax (%LEN%value) so it
    // doesn't need any comma/equals escaping.
    for item in items.iter().skip(1) {
        let title_opt = format!("force-media-title=%{}%{}", item.title.len(), item.title);
        let _ = command(vec![
            json!("loadfile"),
            json!(item.url),
            json!("append"),
            json!(0),
            json!(title_opt),
        ])
        .await;
    }

    Ok(())
}

/// Returns true once the mpv process has exited (e.g. user pressed quit / the
/// window was closed). Callers poll this and then notify the frontend so it
/// can switch back to the browse view.
pub async fn poll_exited() -> bool {
    let mut guard = MPV_CHILD.lock().await;
    if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(Some(_status)) => {
                *guard = None;
                true
            }
            _ => false,
        }
    } else {
        true
    }
}

pub async fn stop_mpv() {
    let mut guard = MPV_CHILD.lock().await;
    if let Some(mut child) = guard.take() {
        let _ = send_raw(json!({"command": ["quit"]})).await;
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        let _ = child.kill().await;
    }
}

pub async fn command(cmd: Vec<Value>) -> anyhow::Result<Value> {
    send_raw(json!({"command": cmd})).await
}

pub async fn set_property(name: &str, value: Value) -> anyhow::Result<Value> {
    send_raw(json!({"command": ["set_property", name, value]})).await
}

pub async fn get_property(name: &str) -> anyhow::Result<Value> {
    send_raw(json!({"command": ["get_property", name]})).await
}

#[cfg(windows)]
async fn send_raw(mut payload: Value) -> anyhow::Result<Value> {
    let id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
    if let Value::Object(ref mut map) = payload {
        map.insert("request_id".into(), json!(id));
    }
    let mut line = serde_json::to_string(&payload)?;
    line.push('\n');

    let mut attempts = 0;
    let mut client = loop {
        match ClientOptions::new().open(PIPE_NAME) {
            Ok(c) => break c,
            Err(e) => {
                attempts += 1;
                if attempts > 20 {
                    anyhow::bail!("Could not connect to mpv IPC pipe: {e}");
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    };

    client.write_all(line.as_bytes()).await?;

    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = client.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.contains(&b'\n') {
            break;
        }
    }

    let text = String::from_utf8_lossy(&buf);
    for line in text.lines() {
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            if v.get("request_id").and_then(|r| r.as_u64()) == Some(id) {
                return Ok(v);
            }
        }
    }
    Ok(json!({"error": "no matching response"}))
}

#[cfg(not(windows))]
async fn send_raw(_payload: Value) -> anyhow::Result<Value> {
    anyhow::bail!("mpv IPC is only implemented for Windows in this build")
}
