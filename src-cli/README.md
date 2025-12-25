# Drive Pulse CLI

A command-line interface for Drive Pulse that allows you to scan drives, view scan history, compare scans, and export comparisons.

## Installation

```bash
cd src-cli
cargo build --release
```

The compiled binary will be available at `target/release/drive-pulse-cli`.

## Usage

### Interactive Mode

Run the CLI without arguments to enter interactive mode:

```bash
drive-pulse-cli
```

### Commands

#### Run a Scan

```bash
drive-pulse-cli scan [path]
```

If no path is provided, you'll be prompted to enter one.

#### List Scan History

```bash
drive-pulse-cli list
```

#### View Scan Details

```bash
drive-pulse-cli view [scan_id]
```

If no scan ID is provided, you'll be prompted to select from available scans.

#### Compare Two Scans

```bash
drive-pulse-cli compare [scan1_id] [scan2_id]
```

If scan IDs are not provided, you'll be prompted to select them.

#### Export Comparison

```bash
drive-pulse-cli export [scan1_id] [scan2_id] [format] -o [output_file]
```

- Format: `json` or `csv`
- If parameters are not provided, you'll be prompted for them

Example:

```bash
drive-pulse-cli export abc123 def456 json -o comparison.json
drive-pulse-cli export abc123 def456 csv -o comparison.csv
```

## Data Storage

Scan data is stored in the `~/.drive-pulse` directory as JSON files.

## Features

- ✅ Run new drive scans
- ✅ List scan history with details
- ✅ View individual scan details
- ✅ Compare two scans to see changes
- ✅ Export comparisons to JSON or CSV
- ✅ Interactive mode for easy navigation
- ✅ Command-line arguments for automation
