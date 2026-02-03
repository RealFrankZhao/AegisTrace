# AEGISTRACE

AEGISTRACE is a cross-platform evidence capture system focused on a unified bundle format, tamper detection, and independent verification. It provides a GUI interface for easy session management and native screen recording capabilities.

## Overview

AEGISTRACE captures system events (app focus, file operations, input statistics, screenshots) and screen recordings into a tamper-proof evidence bundle. The bundle format is consistent across all platforms, and can be independently verified using the included verifier tool.

### Key Features

- **Unified Evidence Bundle Format**: Consistent structure across macOS, Windows, and Linux
- **Tamper Detection**: Hash chain verification ensures evidence integrity
- **Native Screen Recording**: Platform-specific implementations (macOS: Swift/AVFoundation)
- **GUI Interface**: Tauri-based GUI for easy start/stop control
- **Independent Verification**: CLI verifier can validate bundles without the original system
- **Segmented Recording**: Automatic 10-minute video segments to manage file sizes

## Current Implementation Status

### ✅ Completed (macOS)

- **Phase 0**: Core specification, Rust core writer, Verifier v0
- **Phase 1**: Collector MVP with CLI event sending
- **Phase 2**: Hash chain tamper-proofing, Verifier v1
- **Phase 3**: macOS native screen recording (H.265/HEVC, 720p@30fps)
- **Phase 4**: Screenshots and input statistics
- **Phase 5**: Packaging and CI workflows
- **GUI**: Tauri v2 interface with start/stop controls

### 🚧 In Progress / Planned

- Windows collector implementation
- Linux collector implementation
- Network domain tracking (optional, privacy-focused)

## Repository Layout

```
AEGISTRACE/
  apps/
    aegis-tauri/                 # Tauri v2 GUI application
      ui/                        # Frontend (HTML/CSS/JS)
      src-tauri/                 # Tauri backend (Rust)
  crates/
    aegis-core/                  # Core bundle writer (events, hash chain, manifest)
    aegis-core-server/           # TCP server for collector IPC
    aegis-collector-cli/         # CLI tool for sending events
    aegis-verifier/              # Evidence bundle verifier
  collectors/
    macos/
      native_recorder/           # Swift native screen recorder
      run_demo.sh                # macOS demo script
    windows/                     # Windows collector (planned)
    linux/                       # Linux collector (planned)
  spec/
    evidence_bundle.md           # Evidence bundle specification
  scripts/
    build_macos.sh               # macOS build script
    build_linux.sh               # Linux build script
    build_windows.ps1            # Windows build script
    macos_app_bundle.sh          # macOS .app bundle creator
  docs/
    AEGISTRACE_fullstack_guide.txt  # Full technical guide
    PROJECT_OVERVIEW.md             # Current implementation overview
```

## Evidence Bundle Format

Each evidence bundle follows a standardized structure:

```
Evidence_YYYYMMDD_HHMMSS/
  session.json                  # Session metadata
  events.jsonl                  # Event log (one JSON per line)
  manifest.json                 # File manifest with hashes
  files/
    screen_1_20260203_120000.mov    # Screen recording segments
    screen_2_20260203_121000.mov
    shots/
      000001.jpg                # Screenshots
```

### Event Structure

Each event in `events.jsonl` contains:

- `seq`: Strictly increasing sequence number (starts at 1)
- `ts`: UTC timestamp (ISO8601)
- `type`: Event type (e.g., `session_started`, `app_focus_changed`, `file_added`)
- `payload`: Event-specific JSON data
- `prev_hash`: Hash of previous event (for tamper detection)
- `hash`: Hash of current event (SHA-256)

### Event Types

- `session_started`: Session initialization
- `session_stopped`: Session termination
- `app_focus_changed`: Application/window focus changes
- `file_added`: File added to bundle (screen recordings, screenshots)
- `shot_saved`: Screenshot saved
- `input_stats`: Input statistics (key counts, intervals)

## Installation & Build

### Prerequisites

- **Rust**: Latest stable version (https://rustup.rs/)
- **macOS**: Xcode Command Line Tools
- **Tauri CLI** (for GUI): `cargo install tauri-cli`

### Build All Components

```bash
# Build all Rust crates
cargo build --release

# Build macOS native recorder
cd collectors/macos/native_recorder
swift build -c release
```

### Build Scripts

```bash
# macOS
./scripts/build_macos.sh

# Linux
./scripts/build_linux.sh

# Windows
./scripts/build_windows.ps1
```

## Usage

### GUI Application (Recommended)

1. **Start the GUI**:
   ```bash
   cd apps/aegis-tauri
   cargo tauri dev
   ```

2. **Use the Interface**:
   - Click "Start Session" to begin recording
   - Click "Stop Session" to end recording and finalize the bundle
   - Status and logs are displayed in the GUI

3. **Output Location**: Evidence bundles are saved to `~/Downloads/Evidence_YYYYMMDD_HHMMSS/`

### CLI Usage

#### Start Core Server

```bash
cargo run -p aegis-core-server --release
```

The server listens on `127.0.0.1:7878` by default.

#### Send Events (Collector CLI)

```bash
# App focus change
cargo run -p aegis-collector-cli -- focus "com.apple.Safari" "Safari" "Window Title"

# File added
cargo run -p aegis-collector-cli -- file /path/to/source files/screen.mp4 screen_recording

# Screenshot saved
cargo run -p aegis-collector-cli -- shot /path/to/shot.jpg files/shots/000001.jpg

# Input statistics
cargo run -p aegis-collector-cli -- input 10000 150 5 2

# Stop session
cargo run -p aegis-collector-cli -- stop "User requested"
```

#### macOS Demo Script

```bash
./collectors/macos/run_demo.sh
```

This script demonstrates a complete session with screen recording, screenshots, and events.

### Verify Evidence Bundle

```bash
cargo run -p aegis-verifier -- verify /path/to/Evidence_YYYYMMDD_HHMMSS
```

The verifier checks:
- Bundle structure (required files exist)
- Event sequence continuity
- Hash chain integrity
- File existence and hashes
- Manifest consistency

Output: `PASS` or `FAIL` with specific error details.

## Screen Recording

### macOS Native Recorder

The macOS recorder uses Swift and AVFoundation:

- **Codec**: H.265/HEVC
- **Resolution**: 1280x720 (scaled from display, maintaining aspect ratio)
- **Frame Rate**: 30 fps
- **Bitrate**: ~2 Mbps
- **Format**: `.mov`
- **Segmentation**: Automatic 10-minute segments

### Recording Segments

Videos are automatically segmented every 10 minutes:
- Filename format: `files/screen_<segment_number>_<timestamp>.mov`
- Each segment triggers a `file_added` event
- Segments are numbered sequentially (1, 2, 3, ...)

## Packaging

### macOS App Bundle

Create a standalone `.app` bundle:

```bash
./scripts/macos_app_bundle.sh
```

Output: `dist/macos/AEGISTRACE.app`

### Release Builds

GitHub Actions automatically builds release artifacts when tags are pushed:

```bash
git tag v1.0.0
git push origin v1.0.0
```

Artifacts are available in the GitHub Releases page.

## Technical Details

### IPC Protocol

Collectors communicate with the core server via TCP:

- **Address**: `127.0.0.1:7878`
- **Format**: JSON messages (one per line)
- **Example**:
  ```json
  {"type":"app_focus_changed","payload":{"app_id":"com.apple.Safari","app_name":"Safari"}}
  ```

### Hash Chain

Events are linked via a hash chain:
- Each event's `hash` is computed from: `SHA256({seq, ts, type, payload, prev_hash})`
- The first event's `prev_hash` is empty
- Tampering with any event breaks the chain

### Manifest

The `manifest.json` contains:
- `events_hash`: SHA-256 of entire `events.jsonl`
- `final_hash`: Hash of the last event
- `files`: Array of file records with `rel_path` and `hash`

## Development

### Running Tests

```bash
cargo test
```

### Project Structure

- **Core Logic**: `crates/aegis-core/` - Bundle writing, hash chain
- **Server**: `crates/aegis-core-server/` - TCP IPC server
- **Verifier**: `crates/aegis-verifier/` - Bundle validation
- **GUI**: `apps/aegis-tauri/` - Tauri application
- **Collectors**: `collectors/<platform>/` - Platform-specific collectors

### Adding New Event Types

1. Define the event type in `spec/evidence_bundle.md`
2. Update `aegis-core` to handle the new type
3. Update collectors to send the new event
4. Update verifier if needed

## Documentation

- **Full Technical Guide**: `docs/AEGISTRACE_fullstack_guide.txt`
- **Project Overview**: `docs/PROJECT_OVERVIEW.md`
- **Evidence Bundle Spec**: `spec/evidence_bundle.md`
- **Collector README**: `collectors/README.md`

## Privacy & Compliance

- **Self-verification only**: Designed for personal device evidence
- **Explicit permissions**: All sensitive permissions require system authorization dialogs
- **Input metadata only**: Input statistics are recorded, not plaintext
- **Network domain-level**: Network tracking (if implemented) records domains only, not content
- **Independent verification**: Bundles can be verified without the original system

## License

[Add your license here]

## Contributing

[Add contribution guidelines here]

## Roadmap

See `docs/AEGISTRACE_fullstack_guide.txt` for the complete development roadmap.

Current priorities:
- Windows collector implementation
- Linux collector implementation
- Enhanced GUI features
- Network domain tracking (optional)
