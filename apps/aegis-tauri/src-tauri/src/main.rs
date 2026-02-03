use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::State;

#[derive(Deserialize, Clone)]
struct Config {
    server: ServerConfig,
    recording: RecordingConfig,
    paths: PathsConfig,
    app: AppConfig,
}

#[derive(Deserialize, Clone)]
struct ServerConfig {
    default_addr: String,
    stop_wait_ms: u64,
    stop_retry_count: u32,
    stop_retry_interval_ms: u64,
}

#[derive(Deserialize, Clone)]
struct RecordingConfig {
    segment_duration_seconds: u64,
    poll_interval_ms: u64,
    video: VideoConfig,
}

#[derive(Deserialize, Clone)]
struct VideoConfig {
    codec: String,
    resolution: ResolutionConfig,
    fps: u32,
    bitrate_bps: u64,
}

#[derive(Deserialize, Clone)]
struct ResolutionConfig {
    width: u32,
    height: u32,
}

#[derive(Deserialize, Clone)]
struct PathsConfig {
    default_save_dir: String,
    temp_dir: Option<String>,
}

#[derive(Deserialize, Clone)]
struct AppConfig {
    platform: String,
    version: String,
}

struct AppState {
    config: Arc<Config>,
    child: Arc<Mutex<Option<Child>>>,
    addr: Arc<Mutex<String>>,
    session_dir: Arc<Mutex<Option<String>>>,
    recorder: Arc<Mutex<Option<Child>>>,
    recorder_output: Arc<Mutex<Option<PathBuf>>>,
    recording_active: Arc<AtomicBool>,
    recorder_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

#[derive(Serialize)]
struct Status {
    running: bool,
    addr: String,
    session_dir: Option<String>,
}

#[tauri::command]
fn get_status(state: State<AppState>) -> Result<Status, String> {
    let running = state.child.lock().map_err(|_| "lock error")?.is_some();
    let addr = state.addr.lock().map_err(|_| "lock error")?.clone();
    let session_dir = state.session_dir.lock().map_err(|_| "lock error")?.clone();
    Ok(Status {
        running,
        addr,
        session_dir,
    })
}

fn load_config() -> Result<Config, String> {
    let config_path = find_config_path()?;
    let content = std::fs::read_to_string(&config_path)
        .map_err(|err| format!("read config {}: {err}", config_path.display()))?;
    let config: Config = serde_json::from_str(&content)
        .map_err(|err| format!("parse config: {err}"))?;
    Ok(config)
}

fn find_config_path() -> Result<PathBuf, String> {
    // Try workspace root first
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(PathBuf::from);
    
    if let Some(root) = workspace_root {
        let candidate = root.join("config/config.json");
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    
    // Try current directory
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("config/config.json");
        if candidate.exists() {
            return Ok(candidate);
        }
        let candidate = cwd.join("../config/config.json");
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    
    Err("config/config.json not found".to_string())
}

fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

#[tauri::command]
fn start_session(
    platform: Option<String>,
    app_version: Option<String>,
    addr: Option<String>,
    save_dir: Option<String>,
    state: State<AppState>,
) -> Result<Status, String> {
    {
        let mut child_guard = state.child.lock().map_err(|_| "lock error")?;
        if let Some(child) = child_guard.as_mut() {
            if let Ok(Some(_)) = child.try_wait() {
                *child_guard = None;
            }
        }
        if child_guard.is_some() {
            return Err("server already running".to_string());
        }

        let _ = stop_recorder_only(&state);

        let config = state.config.clone();
        let platform = platform.unwrap_or_else(|| config.app.platform.clone());
        let app_version = app_version.unwrap_or_else(|| config.app.version.clone());
        let addr = addr.unwrap_or_else(|| config.server.default_addr.clone());
        *state.addr.lock().map_err(|_| "lock error")? = addr.clone();
        *state.session_dir.lock().map_err(|_| "lock error")? = None;

        let save_dir = if let Some(sd) = save_dir {
            expand_path(&sd)
        } else {
            expand_path(&config.paths.default_save_dir)
        };
        
        let core_server = find_core_server()?;
        let mut cmd = Command::new(core_server);
        cmd.arg(&platform).arg(&app_version);
        cmd.arg(save_dir.to_string_lossy().to_string());
        cmd.arg(addr.clone());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|err| format!("start server: {err}"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or("failed to capture server stderr")?;

        let session_dir_handle = state.session_dir.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                if let Some(found) = line.strip_prefix("Session started at ") {
                    let path = found.split_whitespace().next().unwrap_or(found).to_string();
                    if let Ok(mut guard) = session_dir_handle.lock() {
                        *guard = Some(path);
                    }
                }
            }
        });

        *state
            .recorder_output
            .lock()
            .map_err(|_| "lock error")? = None;
        state
            .recording_active
            .store(true, Ordering::SeqCst);
        start_recorder_loop(&state, addr.clone(), (*state.config).clone())?;

        *child_guard = Some(child);
    }

    get_status(state)
}

#[tauri::command]
fn stop_session(reason: Option<String>, state: State<AppState>) -> Result<Status, String> {
    let addr = state.addr.lock().map_err(|_| "lock error")?.clone();
    let reason = reason.unwrap_or_else(|| "user".to_string());

    state
        .recording_active
        .store(false, Ordering::SeqCst);
    let _ = stop_recorder_only(&state);
    if let Ok(mut handle_guard) = state.recorder_thread.lock() {
        if let Some(handle) = handle_guard.take() {
            let _ = handle.join();
        }
    }
    send_message(
        &addr,
        json!({
            "type": "stop",
            "payload": { "reason": reason }
        }),
    );

    let config = state.config.clone();
    if let Ok(mut child_guard) = state.child.lock() {
        if let Some(mut child) = child_guard.take() {
            if child.try_wait().ok().flatten().is_none() {
                std::thread::sleep(Duration::from_millis(config.server.stop_wait_ms));
            }
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }

    get_status(state)
}

fn send_message(addr: &str, payload: serde_json::Value) {
    if let Ok(mut stream) = TcpStream::connect(addr) {
        let _ = serde_json::to_writer(&mut stream, &payload);
        let _ = stream.write_all(b"\n");
    }
}

fn start_recorder_loop(state: &State<AppState>, addr: String, config: Config) -> Result<(), String> {
    let recorder_path = find_native_recorder()?;
    let recording_active = state.recording_active.clone();
    let recorder_state = state.recorder.clone();
    let recorder_output = state.recorder_output.clone();
    let segment_duration = config.recording.segment_duration_seconds;
    let poll_interval = config.recording.poll_interval_ms;

    let handle = thread::spawn(move || {
        let mut segment_index: u64 = 1;
        while recording_active.load(Ordering::SeqCst) {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|value| value.as_secs())
                .unwrap_or(0);
            let temp_dir = if let Some(ref td) = config.paths.temp_dir {
                expand_path(td)
            } else {
                std::env::temp_dir()
            };
            let output_path =
                temp_dir.join(format!("aegis_screen_{segment_index}_{timestamp}.mov"));
            let _ = std::fs::remove_file(&output_path);

            let mut cmd = Command::new(&recorder_path);
            cmd.arg(output_path.to_string_lossy().to_string())
                .arg(segment_duration.to_string());
            cmd.stdin(Stdio::piped());
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());

            let child = match cmd.spawn() {
                Ok(child) => child,
                Err(err) => {
                    eprintln!("start recorder failed: {err}");
                    break;
                }
            };

            if let Ok(mut guard) = recorder_state.lock() {
                *guard = Some(child);
            }
            if let Ok(mut guard) = recorder_output.lock() {
                *guard = Some(output_path.clone());
            }

            loop {
                let done = recorder_state
                    .lock()
                    .ok()
                    .and_then(|mut guard| {
                        guard
                            .as_mut()
                            .and_then(|child| child.try_wait().ok().flatten())
                    })
                    .is_some();
                if done {
                    break;
                }
                if !recording_active.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(poll_interval));
                    continue;
                }
                thread::sleep(Duration::from_millis(poll_interval));
            }

            if let Ok(mut guard) = recorder_state.lock() {
                if let Some(mut child) = guard.take() {
                    let _ = child.wait();
                }
            }

            if output_path.exists() {
                if let Ok(metadata) = std::fs::metadata(&output_path) {
                    if metadata.len() > 0 {
                        send_message(
                            &addr,
                            json!({
                                "type": "file_added",
                                "payload": {
                                    "source_path": output_path.to_string_lossy(),
                                    "rel_path": format!("files/screen_{segment_index}_{timestamp}.mov"),
                                    "kind": "screen_recording"
                                }
                            }),
                        );
                    }
                }
            }

            segment_index += 1;
            if !recording_active.load(Ordering::SeqCst) {
                break;
            }
        }
    });

    if let Ok(mut guard) = state.recorder_thread.lock() {
        *guard = Some(handle);
    }
    Ok(())
}

fn stop_recorder_only(state: &State<AppState>) -> Result<(), String> {
    let config = state.config.clone();
    let mut recorder_guard = state.recorder.lock().map_err(|_| "lock error")?;
    if let Some(mut recorder) = recorder_guard.take() {
        if let Some(mut stdin) = recorder.stdin.take() {
            let _ = stdin.write_all(b"\n");
        }
        for _ in 0..config.server.stop_retry_count {
            if recorder.try_wait().ok().flatten().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(config.server.stop_retry_interval_ms));
        }
        if recorder.try_wait().ok().flatten().is_none() {
            let _ = recorder.kill();
        }
        let _ = recorder.wait();
    }
    Ok(())
}

fn find_core_server() -> Result<PathBuf, String> {
    if let Some(explicit) = std::env::var_os("AEGIS_CORE_SERVER") {
        return Ok(PathBuf::from(explicit));
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let candidate = parent.join("aegis-core-server");
            if candidate.exists() {
                return Ok(candidate);
            }
            let resources = parent.join("../Resources/aegis-core-server");
            if resources.exists() {
                return Ok(resources);
            }
        }
    }

    if let Ok(path_var) = std::env::var("PATH") {
        for entry in path_var.split(':') {
            let candidate = Path::new(entry).join("aegis-core-server");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(PathBuf::from);
    let mut candidates = Vec::new();
    if let Some(root) = workspace_root {
        candidates.push(root.join("target/debug/aegis-core-server"));
        candidates.push(root.join("target/release/aegis-core-server"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("../../target/debug/aegis-core-server"));
        candidates.push(cwd.join("../../target/release/aegis-core-server"));
        candidates.push(cwd.join("../target/debug/aegis-core-server"));
        candidates.push(cwd.join("../target/release/aegis-core-server"));
    }
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("aegis-core-server not found. Set AEGIS_CORE_SERVER or build target/debug/aegis-core-server.".to_string())
}

fn find_native_recorder() -> Result<PathBuf, String> {
    if let Some(explicit) = std::env::var_os("AEGIS_NATIVE_RECORDER") {
        return Ok(PathBuf::from(explicit));
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let candidate = parent.join("aegis-native-recorder");
            if candidate.exists() {
                return Ok(candidate);
            }
            let resources = parent.join("../Resources/aegis-native-recorder");
            if resources.exists() {
                return Ok(resources);
            }
        }
    }

    if let Ok(path_var) = std::env::var("PATH") {
        for entry in path_var.split(':') {
            let candidate = Path::new(entry).join("aegis-native-recorder");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(PathBuf::from);
    if let Some(root) = workspace_root {
        let candidate =
            root.join("collectors/macos/native_recorder/.build/release/aegis-native-recorder");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("aegis-native-recorder not found. Set AEGIS_NATIVE_RECORDER or build collectors/macos/native_recorder.".to_string())
}

fn main() {
    let config = load_config().expect("Failed to load config");
    let default_addr = config.server.default_addr.clone();
    
    tauri::Builder::default()
        .manage(AppState {
            config: Arc::new(config),
            child: Arc::new(Mutex::new(None)),
            addr: Arc::new(Mutex::new(default_addr)),
            session_dir: Arc::new(Mutex::new(None)),
            recorder: Arc::new(Mutex::new(None)),
            recorder_output: Arc::new(Mutex::new(None)),
            recording_active: Arc::new(AtomicBool::new(false)),
            recorder_thread: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            start_session,
            stop_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
