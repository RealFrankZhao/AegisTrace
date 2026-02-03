use aegis_core::SessionWriter;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex, RwLock,
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::State;

#[derive(Deserialize, Clone)]
struct Config {
    recording: RecordingConfig,
    paths: PathsConfig,
    app: AppConfig,
}

#[derive(Deserialize, Clone)]
struct RecordingConfig {
    segment_duration_seconds: u64,
    poll_interval_ms: u64,
    #[allow(dead_code)]
    video: VideoConfig,
}

#[derive(Deserialize, Clone)]
#[allow(dead_code)]
struct VideoConfig {
    codec: String,
    resolution: ResolutionConfig,
    fps: u32,
    bitrate_bps: u64,
}

#[derive(Deserialize, Clone)]
#[allow(dead_code)]
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

// Optimized state structure with reduced lock contention
struct AppState {
    config: Arc<Config>,
    session_writer: Arc<RwLock<Option<SessionWriter>>>, // RwLock for read-heavy access
    session_dir: Arc<RwLock<Option<PathBuf>>>, // Cache session dir to avoid lock on writer
    recorder: Arc<Mutex<Option<Child>>>,
    recorder_output: Arc<Mutex<Option<PathBuf>>>,
    recording_active: Arc<AtomicBool>,
    recorder_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    segment_index: Arc<AtomicU64>, // Atomic counter for segments
    recorder_path: Arc<RwLock<Option<PathBuf>>>, // Cache recorder path
}

#[derive(Serialize)]
struct Status {
    running: bool,
    session_dir: Option<String>,
}

#[tauri::command]
fn get_status(state: State<AppState>) -> Result<Status, String> {
    let running = state.session_writer.read().map_err(|_| "lock error")?.is_some();
    let session_dir = state
        .session_dir
        .read()
        .map_err(|_| "lock error")?
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());
    Ok(Status {
        running,
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
    save_dir: Option<String>,
    state: State<AppState>,
) -> Result<Status, String> {
    {
        let mut writer_guard = state.session_writer.write().map_err(|_| "lock error")?;
        if writer_guard.is_some() {
            return Err("session already running".to_string());
        }

        let _ = stop_recorder_only(&state);

        let config = state.config.clone();
        let platform = platform.unwrap_or_else(|| config.app.platform.clone());
        let app_version = app_version.unwrap_or_else(|| config.app.version.clone());

        let save_dir = if let Some(sd) = save_dir {
            expand_path(&sd)
        } else {
            expand_path(&config.paths.default_save_dir)
        };

        // Directly create SessionWriter (no TCP server needed)
        eprintln!("Creating session writer...");
        let writer = SessionWriter::start_session(&save_dir, &platform, &app_version)
            .map_err(|err| format!("start session: {err}"))?;
        eprintln!("Session writer created successfully");

        let session_dir = writer.session_dir().to_path_buf();
        eprintln!("Session directory: {}", session_dir.display());
        *state.session_dir.write().map_err(|_| "lock error")? = Some(session_dir.clone());

        *state.recorder_output.lock().map_err(|_| "lock error")? = None;
        state.recording_active.store(true, Ordering::SeqCst);
        state.segment_index.store(1, Ordering::SeqCst);

        // Check and cache recorder path before starting (fail fast)
        eprintln!("Checking recorder path...");
        {
            let mut cached = state.recorder_path.write().map_err(|_| "lock error")?;
            if cached.is_none() || cached.as_ref().map(|p| !p.exists()).unwrap_or(true) {
                let path = find_native_recorder().map_err(|err| {
                    eprintln!("Recorder path error: {}", err);
                    format!("Cannot find recorder: {}. Please build it with: cd collectors/macos/native_recorder && swift build -c release", err)
                })?;
                eprintln!("Recorder path found: {}", path.display());
                *cached = Some(path);
            }
        }

        // Start recorder loop asynchronously to avoid blocking UI
        eprintln!("Starting recorder loop in background...");
        // Clone the Arc-wrapped fields we need for the thread
        let recording_active = state.recording_active.clone();
        let recorder_state = state.recorder.clone();
        let recorder_output = state.recorder_output.clone();
        let segment_index = state.segment_index.clone();
        let writer_arc = Arc::clone(&state.session_writer);
        let recorder_path = Arc::clone(&state.recorder_path);
        let recorder_thread = Arc::clone(&state.recorder_thread);
        let session_dir_clone = session_dir.clone();
        let config_clone = (*state.config).clone();
        
        let handle = thread::spawn(move || {
            start_recorder_loop_thread(
                recording_active,
                recorder_state,
                recorder_output,
                segment_index,
                writer_arc,
                recorder_path,
                recorder_thread,
                session_dir_clone,
                config_clone,
            )
        });
        
        // Store the thread handle
        if let Ok(mut guard) = state.recorder_thread.lock() {
            *guard = Some(handle);
        }
        eprintln!("Recorder loop started");

        *writer_guard = Some(writer);
    }

    get_status(state)
}

#[tauri::command]
fn stop_session(reason: Option<String>, state: State<AppState>) -> Result<Status, String> {
    let reason = reason.unwrap_or_else(|| "user".to_string());

    // Stop recording immediately
    state.recording_active.store(false, Ordering::SeqCst);

    // Stop current recorder process
    let _ = stop_recorder_only(&state);

    // Wait for recorder thread to finish processing files
    if let Ok(mut handle_guard) = state.recorder_thread.lock() {
        if let Some(handle) = handle_guard.take() {
            let join_handle = thread::spawn(move || {
                let _ = handle.join();
            });

            let wait_start = SystemTime::now();
            let max_wait = Duration::from_secs(8);

            while !join_handle.is_finished() {
                if wait_start.elapsed().unwrap_or(Duration::from_secs(0)) > max_wait {
                    eprintln!("Warning: Recorder thread did not finish within {}s", max_wait.as_secs());
                    break;
                }
                thread::sleep(Duration::from_millis(200));
            }

            let _ = join_handle.join();
        }
    }

    // Stop session and finalize bundle
    if let Ok(mut writer_guard) = state.session_writer.write() {
        if let Some(mut writer) = writer_guard.take() {
            writer.stop_session(&reason)
                .map_err(|err| format!("stop session: {err}"))?;
        }
    }

    // Clear cached session dir
    *state.session_dir.write().map_err(|_| "lock error")? = None;

    get_status(state)
}

// Thread function that runs the recorder loop
fn start_recorder_loop_thread(
    recording_active: Arc<AtomicBool>,
    recorder_state: Arc<Mutex<Option<Child>>>,
    recorder_output: Arc<Mutex<Option<PathBuf>>>,
    segment_index: Arc<AtomicU64>,
    writer: Arc<RwLock<Option<SessionWriter>>>,
    recorder_path: Arc<RwLock<Option<PathBuf>>>,
    recorder_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    session_dir: PathBuf,
    config: Config,
) {
    // Get recorder path (should be cached by now)
    let recorder_path = match recorder_path.read() {
        Ok(cached) => {
            if let Some(ref path) = *cached {
                path.clone()
            } else {
                eprintln!("Error: recorder path not cached");
                return;
            }
        }
        Err(_) => {
            eprintln!("Error: failed to read recorder path");
            return;
        }
    };
    
    eprintln!("Starting recorder with path: {}", recorder_path.display());

    let segment_duration = config.recording.segment_duration_seconds;
    let poll_interval = config.recording.poll_interval_ms;

    let handle = thread::spawn(move || {
        while recording_active.load(Ordering::SeqCst) {
            let current_segment = segment_index.load(Ordering::SeqCst);
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|value| value.as_secs())
                .unwrap_or(0);

            let temp_dir = if let Some(ref td) = config.paths.temp_dir {
                expand_path(td)
            } else {
                std::env::temp_dir()
            };

            let output_path = temp_dir.join(format!("aegis_screen_{current_segment}_{timestamp}.mov"));
            let _ = std::fs::remove_file(&output_path);

            // Start recorder process
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

            // Optimized: More efficient polling with exponential backoff
            let mut poll_count = 0;
            loop {
                if !recording_active.load(Ordering::SeqCst) {
                    // Graceful stop
                    if let Ok(mut guard) = recorder_state.lock() {
                        if let Some(child) = guard.as_mut() {
                            if let Some(mut stdin) = child.stdin.take() {
                                let _ = stdin.write_all(b"\n");
                                drop(stdin);
                            }
                        }
                    }

                    let wait_start = SystemTime::now();
                    let max_graceful_wait = Duration::from_secs(5);
                    let mut graceful_stopped = false;

                    while wait_start.elapsed().unwrap_or(Duration::from_secs(0)) < max_graceful_wait {
                        if let Ok(mut guard) = recorder_state.lock() {
                            if let Some(child) = guard.as_mut() {
                                if child.try_wait().ok().flatten().is_some() {
                                    graceful_stopped = true;
                                    break;
                                }
                            }
                        }
                        thread::sleep(Duration::from_millis(100));
                    }

                    if !graceful_stopped {
                        if let Ok(mut guard) = recorder_state.lock() {
                            if let Some(mut child) = guard.take() {
                                let _ = child.kill();
                                let _ = child.wait();
                            }
                        }
                    }
                    break;
                }

                // Check if recording finished
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

                // Adaptive polling: faster when just started, slower later
                let sleep_ms = if poll_count < 10 {
                    poll_interval
                } else {
                    poll_interval * 2
                };
                thread::sleep(Duration::from_millis(sleep_ms));
                poll_count += 1;
            }

            // Wait for process to finish
            if let Ok(mut guard) = recorder_state.lock() {
                if let Some(mut child) = guard.take() {
                    let _ = child.wait();
                }
            }

            // Process file asynchronously to avoid blocking next segment
            let output_path_clone = output_path.clone();
            let session_dir_clone = session_dir.clone();
            let writer_clone = Arc::clone(&writer);
            let segment_idx = current_segment;

            thread::spawn(move || {
                process_video_segment(
                    &output_path_clone,
                    &session_dir_clone,
                    segment_idx,
                    timestamp,
                    writer_clone,
                );
            });

            segment_index.fetch_add(1, Ordering::SeqCst);

            if !recording_active.load(Ordering::SeqCst) {
                break;
            }
        }
    });

    if let Ok(mut guard) = recorder_thread.lock() {
        *guard = Some(handle);
    }
    eprintln!("Recorder thread spawned successfully");
}


// Extracted file processing to separate function for better organization
fn process_video_segment(
    output_path: &Path,
    session_dir: &Path,
    segment_index: u64,
    timestamp: u64,
    writer: Arc<RwLock<Option<SessionWriter>>>,
) {
    // Wait a bit for file system to sync
    thread::sleep(Duration::from_millis(500));

    if !output_path.exists() {
        eprintln!("Warning: Segment {} file does not exist", segment_index);
        return;
    }

    let metadata = match std::fs::metadata(output_path) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("Warning: Cannot read segment {} metadata: {}", segment_index, err);
            return;
        }
    };

    let file_size = metadata.len();
    if file_size == 0 {
        eprintln!("Warning: Segment {} file is empty", segment_index);
        return;
    }

    let rel_path = format!("files/screen_{segment_index}_{timestamp}.mov");
    let dest_path = session_dir.join(&rel_path);

    // Create parent directory if needed
    if let Some(parent) = dest_path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("Failed to create directory for segment {}: {}", segment_index, err);
            return;
        }
    }

    // Copy file
    if let Err(err) = fs::copy(output_path, &dest_path) {
        eprintln!("Failed to copy video file for segment {}: {}", segment_index, err);
        return;
    }

    // Add event (write lock for writer)
    if let Ok(mut guard) = writer.write() {
        if let Some(w) = guard.as_mut() {
            if let Err(err) = w.append_event(
                "file_added",
                serde_json::json!({
                    "rel_path": rel_path,
                    "kind": "screen_recording"
                }),
            ) {
                eprintln!("Failed to add event for segment {}: {}", segment_index, err);
            } else {
                eprintln!("Added video segment {} (size: {} bytes)", segment_index, file_size);
            }
        }
    }
}

fn stop_recorder_only(state: &State<AppState>) -> Result<(), String> {
    let mut recorder_guard = state.recorder.lock().map_err(|_| "lock error")?;
    if let Some(mut recorder) = recorder_guard.take() {
        if let Some(mut stdin) = recorder.stdin.take() {
            let _ = stdin.write_all(b"\n");
            drop(stdin);
        }

        let max_wait = Duration::from_millis(1000);
        let wait_start = SystemTime::now();

        while wait_start.elapsed().unwrap_or(Duration::from_secs(0)) < max_wait {
            if recorder.try_wait().ok().flatten().is_some() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(200));
        }

        if recorder.try_wait().ok().flatten().is_none() {
            let _ = recorder.kill();
        }

        let wait_start = SystemTime::now();
        let max_final_wait = Duration::from_secs(2);
        while recorder.try_wait().ok().flatten().is_none() {
            if wait_start.elapsed().unwrap_or(Duration::from_secs(0)) > max_final_wait {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = recorder.wait();
    }
    Ok(())
}

// Optimized: Cache result to avoid repeated lookups
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

    tauri::Builder::default()
        .manage(AppState {
            config: Arc::new(config),
            session_writer: Arc::new(RwLock::new(None)),
            session_dir: Arc::new(RwLock::new(None)),
            recorder: Arc::new(Mutex::new(None)),
            recorder_output: Arc::new(Mutex::new(None)),
            recording_active: Arc::new(AtomicBool::new(false)),
            recorder_thread: Arc::new(Mutex::new(None)),
            segment_index: Arc::new(AtomicU64::new(1)),
            recorder_path: Arc::new(RwLock::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            start_session,
            stop_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
