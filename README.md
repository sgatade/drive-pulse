# Drive Pulse 🔍

A cross-platform application for scanning drives and comparing snapshots over time. Built with Tauri + Rust for high performance and React for a modern UI.

**Available in two flavors:**
- 🖥️ **GUI Application** - Desktop app with React UI
- 💻 **CLI Tool** - Command-line interface for scripting and automation

## Features

- 📁 **Drive Scanning** - Fast recursive scanning of any drive or folder
- 💾 **Snapshot Storage** - Save complete file listings with metadata (size, modified date)
- 🔒 **Optional Encryption** - AES-256-GCM encryption with password protection
- ⚡ **Binary Format** - 3-5x faster read/write using bincode
- 📊 **Visual Comparison** - Compare any two snapshots to see what changed
- 🔍 **Detailed Diff View** - See added, deleted, and modified files
- 🗑️ **Snapshot Management** - Delete old snapshots to save space
- 🖥️ **Cross-Platform** - Works on Windows, macOS, and Linux
- 💻 **CLI Support** - Full command-line interface with Tab autocomplete

## Prerequisites

Before running the app, make sure you have:

- [Node.js](https://nodejs.org/) (v16 or higher)
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites) for your OS

## Getting Started

### 1. Install Dependencies

```bash
npm install
```

### 2. Run in Development Mode

```bash
npm run tauri dev
```

This will start the Vite dev server and launch the Tauri application window.

### 3. Build for Production

```bash
npm run tauri build
```

The compiled application will be in `src-backend/target/release/bundle/`.

## CLI Usage

The standalone CLI tool provides full functionality without a GUI:

### Build the CLI

```bash
cd src-cli
cargo build --release
```

The CLI binary will be in `src-cli/target/release/drive-pulse-cli`.

### CLI Commands

```bash
# Interactive mode (with Tab autocomplete for paths)
./drive-pulse-cli

# Run a scan
./drive-pulse-cli scan [path]

# List scan history
./drive-pulse-cli list

# View scan details
./drive-pulse-cli view [scan_id]

# Compare two scans
./drive-pulse-cli compare [scan1_id] [scan2_id]

# Export comparison results
./drive-pulse-cli export [scan1_id] [scan2_id] [format] -o output.csv
```

### CLI Features

- 🎯 **Tab Autocomplete** - Press Tab while typing paths for autocomplete
- 📊 **Real-time Progress** - In-place progress indicator showing files scanned
- 🌍 **UTF-8 Safe** - Handles international characters (Japanese, Chinese, emoji, etc.)
- 📋 **Export Options** - JSON and CSV export formats
- 🎨 **Colored Output** - Beautiful tables and status messages

## How It Works

### Architecture

- **Frontend**: React + TypeScript + Vite
  - Modern, responsive UI
  - Type-safe component architecture
- **Backend**: Rust + Tauri
  - `scan_drive` - Recursively walks directories using `walkdir` crate
  - `get_scan_history` - Loads snapshot summaries from disk
  - `compare_snapshots` - Efficiently compares two snapshots using HashMaps
  - `delete_snapshot` - Removes snapshot files

### Data Storage

Snapshots are stored as JSON files in the application data directory:

- **Windows**: `%APPDATA%\drive-pulse\snapshots\`
- **macOS**: `~/Library/Application Support/drive-pulse/snapshots/`
- **Linux**: `~/.local/share/drive-pulse/snapshots/`

## Usage

1. **Create a Snapshot**

   - Click "Browse" to select a drive or folder
   - Click "Scan Drive" to create a snapshot
   - Scanning is done in the background

2. **View Scan History**

   - All snapshots appear in the history section
   - Shows drive path, timestamp, file count, and total size

3. **Compare Snapshots**

   - Click on 2 snapshots to select them
   - Click "Compare Selected"
   - View added, deleted, and modified files

4. **Manage Snapshots**
   - Click "Delete" on any snapshot card to remove it

## Project Structure

```
drive-pulse/
├── src-frontend/          # React frontend
│   ├── App.tsx            # Main app component
│   ├── main.tsx           # React entry point
│   └── styles.css         # Global styles
├── src-backend/           # Rust backend (Tauri app + shared library)
│   ├── src/
│   │   ├── main.rs        # Tauri app entry
│   │   ├── commands.rs    # Tauri commands
│   │   └── lib.rs         # Shared library code
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── src-cli/               # Standalone CLI tool
│   ├── main.rs            # CLI entry point
│   ├── backend.rs         # CLI-specific backend code
│   └── Cargo.toml         # CLI dependencies
├── index.html             # HTML template
├── package.json           # Node dependencies
└── vite.config.ts         # Vite configuration
```

## Technologies Used

- **[Tauri](https://tauri.app/)** - Desktop app framework
- **[Rust](https://www.rust-lang.org/)** - Backend language
- **[React](https://react.dev/)** - UI framework
- **[TypeScript](https://www.typescriptlang.org/)** - Type safety
- **[Vite](https://vitejs.dev/)** - Build tool
- **[walkdir](https://docs.rs/walkdir/)** - Efficient directory traversal
- **[serde](https://serde.rs/)** - Serialization/deserialization
- **[bincode](https://docs.rs/bincode/)** - Binary serialization
- **[aes-gcm](https://docs.rs/aes-gcm/)** - Encryption
- **[rustyline](https://docs.rs/rustyline/)** - CLI with Tab autocomplete
- **[dialoguer](https://docs.rs/dialoguer/)** - Interactive CLI prompts
- **[prettytable-rs](https://docs.rs/prettytable-rs/)** - Beautiful CLI tables

## Performance

### Tauri vs Electron

Tauri apps are significantly lighter than Electron:

- **App size**: ~5-10 MB (vs ~150MB for Electron)
- **Memory usage**: ~50-100 MB (vs ~300-500MB for Electron)
- **Startup time**: Near instant

### Binary Format

- **JSON**: 100MB snapshot = ~30 seconds to write
- **Bincode**: Same 100MB snapshot = ~6-8 seconds to write (3-5x faster)

The Rust backend provides excellent performance for file system operations, easily handling drives with hundreds of thousands of files.

## License

See [LICENSE](LICENSE) file for details.
Drive Pulse is a on-demand scanner for your drives that can retain snapshots and compare them to show the changes between the current and the previous scans.
