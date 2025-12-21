# Drive Pulse 🔍

A cross-platform desktop application for scanning drives and comparing snapshots over time. Built with Tauri + Rust for high performance and React for a modern UI.

## Features

- 📁 **Drive Scanning** - Fast recursive scanning of any drive or folder
- 💾 **Snapshot Storage** - Save complete file listings with metadata (size, modified date)
- 📊 **Visual Comparison** - Compare any two snapshots to see what changed
- 🔍 **Detailed Diff View** - See added, deleted, and modified files
- 🗑️ **Snapshot Management** - Delete old snapshots to save space
- ⚡ **High Performance** - Rust backend for blazing fast file operations
- 🖥️ **Cross-Platform** - Works on Windows, macOS, and Linux

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

The compiled application will be in `src-tauri/target/release/bundle/`.

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
├── src/                    # React frontend
│   ├── App.tsx            # Main app component
│   ├── main.tsx           # React entry point
│   └── styles.css         # Global styles
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── main.rs        # Tauri app entry
│   │   ├── commands.rs    # Tauri commands
│   │   └── models.rs      # Data structures
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
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

## Performance

Tauri apps are significantly lighter than Electron:

- **App size**: ~5-10 MB (vs ~150MB for Electron)
- **Memory usage**: ~50-100 MB (vs ~300-500MB for Electron)
- **Startup time**: Near instant

The Rust backend provides excellent performance for file system operations, easily handling drives with tens of thousands of files.

## License

See [LICENSE](LICENSE) file for details.
Drive Pulse is a on-demand scanner for your drives that can retain snapshots and compare them to show the changes between the current and the previous scans.
