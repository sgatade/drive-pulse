import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/api/dialog";
import "./styles.css";

interface FileEntry {
  path: string;
  size: number;
  modified: number;
  is_dir: boolean;
}

interface Snapshot {
  id: string;
  drive_path: string;
  timestamp: number;
  total_files: number;
  total_size: number;
  files: FileEntry[];
}

interface SnapshotSummary {
  id: string;
  drive_path: string;
  timestamp: number;
  total_files: number;
  total_size: number;
}

interface FileDiff {
  path: string;
  status: "added" | "deleted" | "modified" | "unchanged";
  old_size?: number;
  new_size?: number;
  old_modified?: number;
  new_modified?: number;
}

interface ComparisonResult {
  snapshot1_id: string;
  snapshot2_id: string;
  added: FileDiff[];
  deleted: FileDiff[];
  modified: FileDiff[];
  unchanged_count: number;
}

interface ScanProgress {
  files_scanned: number;
  current_path: string;
  total_size: number;
}

interface DriveInfo {
  path: string;
  label: string;
}

function App() {
  const [drivePath, setDrivePath] = useState("");
  const [availableDrives, setAvailableDrives] = useState<DriveInfo[]>([]);
  const [scanning, setScanning] = useState(false);
  const [saving, setSaving] = useState(false);
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null);
  const [snapshots, setSnapshots] = useState<SnapshotSummary[]>([]);
  const [selectedSnapshots, setSelectedSnapshots] = useState<string[]>([]);
  const [comparison, setComparison] = useState<ComparisonResult | null>(null);
  const [error, setError] = useState("");
  const [loadingHistory, setLoadingHistory] = useState(true);
  const [encrypt, setEncrypt] = useState(false);
  const [password, setPassword] = useState("");
  const [successMessage, setSuccessMessage] = useState("");

  useEffect(() => {
    loadHistory();
    loadDrives();

    // Listen for scan progress events
    const unlisten = listen<ScanProgress>("scan-progress", (event) => {
      setScanProgress(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const loadDrives = async () => {
    try {
      const drives = await invoke<DriveInfo[]>("get_available_drives");
      setAvailableDrives(drives);
    } catch (err) {
      console.error("Failed to load drives:", err);
    }
  };

  const loadHistory = async () => {
    setLoadingHistory(true);
    try {
      const history = await invoke<SnapshotSummary[]>("get_scan_history");
      setSnapshots(history);
    } catch (err) {
      setError(`Failed to load history: ${err}`);
    } finally {
      setLoadingHistory(false);
    }
  };

  const selectFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });
      if (selected && typeof selected === "string") {
        setDrivePath(selected);
      }
    } catch (err) {
      setError(`Failed to select folder: ${err}`);
    }
  };

  const handleScan = async () => {
    if (!drivePath) {
      setError("Please select a drive or folder to scan");
      return;
    }

    if (encrypt && !password) {
      setError("Please enter a password for encryption");
      return;
    }

    setScanning(true);
    setScanProgress(null);
    setError("");

    try {
      // Show saving indicator when file count is high
      const snapshot = await invoke<Snapshot>("scan_drive", {
        drivePath,
        encrypt,
        password: encrypt ? password : null,
      });

      setSaving(true);
      setScanProgress(null);

      // Small delay to show saving state
      await new Promise((resolve) => setTimeout(resolve, 100));

      // Reload history to show new snapshot
      await loadHistory();

      // Show success notification
      const message = `✅ Scan complete! ${snapshot.total_files.toLocaleString()} files scanned (${formatBytes(snapshot.total_size)})${encrypt ? " 🔒 Encrypted" : ""}`;
      setSuccessMessage(message);

      // Clear success message after 5 seconds
      setTimeout(() => setSuccessMessage(""), 5000);

      setDrivePath("");
      setPassword("");
    } catch (err) {
      setError(`Scan failed: ${err}`);
    } finally {
      setScanning(false);
      setSaving(false);
      setScanProgress(null);
    }
  };

  const showDataLocation = async () => {
    try {
      await invoke("open_data_directory");
    } catch (err) {
      setError(`Failed to open storage location: ${err}`);
    }
  };

  const toggleSnapshot = (id: string) => {
    setSelectedSnapshots((prev) => {
      if (prev.includes(id)) {
        return prev.filter((snapId) => snapId !== id);
      } else {
        if (prev.length >= 2) {
          return [prev[1], id];
        }
        return [...prev, id];
      }
    });
    setComparison(null);
  };

  const handleCompare = async () => {
    if (selectedSnapshots.length !== 2) {
      setError("Please select exactly 2 snapshots to compare");
      return;
    }

    setError("");
    try {
      // Check if encrypted snapshots need password
      const needsPassword = snapshots.find((s) => (s.id === selectedSnapshots[0] || s.id === selectedSnapshots[1]) && s.total_files === 0);

      let pwd = null;
      if (needsPassword) {
        pwd = prompt("Enter password for encrypted snapshot:");
        if (!pwd) return;
      }

      const result = await invoke<ComparisonResult>("compare_snapshots", {
        snapshot1Id: selectedSnapshots[0],
        snapshot2Id: selectedSnapshots[1],
        password: pwd,
      });
      setComparison(result);
    } catch (err) {
      setError(`Comparison failed: ${err}`);
    }
  };

  const handleDelete = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await invoke("delete_snapshot", { snapshotId: id });
      await loadHistory();
      setSelectedSnapshots((prev) => prev.filter((snapId) => snapId !== id));
      setComparison(null);
    } catch (err) {
      setError(`Delete failed: ${err}`);
    }
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 Bytes";
    const k = 1024;
    const sizes = ["Bytes", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + " " + sizes[i];
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  return (
    <div className="container">
      <div className="header">
        <h1>🔍 Drive Pulse</h1>
        <p>Scan drives and compare snapshots</p>
      </div>

      {error && <div className="error">{error}</div>}
      {successMessage && <div className="success">{successMessage}</div>}

      <div className="scan-section">
        <h2>New Scan</h2>
        <div className="scan-controls">
          <div className="drive-selector">
            <select value={drivePath} onChange={(e) => setDrivePath(e.target.value)} disabled={scanning}>
              <option value="">Select a drive...</option>
              {availableDrives.map((drive) => (
                <option key={drive.path} value={drive.path}>
                  {drive.label}
                </option>
              ))}
              <option value="custom">Custom Path...</option>
            </select>
            <button onClick={loadDrives} disabled={scanning} className="refresh-btn" title="Refresh drives list">
              🔄
            </button>
          </div>
          {drivePath === "custom" && (
            <>
              <input type="text" value={drivePath !== "custom" ? drivePath : ""} onChange={(e) => setDrivePath(e.target.value)} placeholder="Enter custom path..." />
              <button onClick={selectFolder}>Browse</button>
            </>
          )}
          <button onClick={handleScan} disabled={scanning || !drivePath || drivePath === "custom"}>
            {scanning ? "Scanning..." : "Scan Drive"}
          </button>
        </div>

        <div className="encryption-controls">
          <label className="checkbox-label">
            <input type="checkbox" checked={encrypt} onChange={(e) => setEncrypt(e.target.checked)} disabled={scanning} />
            <span>🔒 Encrypt snapshot</span>
          </label>
          {encrypt && <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="Enter encryption password..." disabled={scanning} className="password-input" />}
        </div>

        {scanning && scanProgress && (
          <div className="scan-progress">
            <div className="progress-info">
              <div>
                <strong>Files scanned:</strong> {scanProgress.files_scanned.toLocaleString()}
              </div>
              <div>
                <strong>Total size:</strong> {formatBytes(scanProgress.total_size)}
              </div>
            </div>
            <div className="current-file">
              <strong>Current:</strong> {scanProgress.current_path}
            </div>
          </div>
        )}
        {saving && (
          <div className="scan-progress">
            <div className="progress-info" style={{ justifyContent: "center" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
                <div className="spinner-small"></div>
                <strong>Saving snapshot to disk...</strong>
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="history-section">
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1rem" }}>
          <h2>Scan History ({snapshots.length})</h2>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <button onClick={loadHistory} disabled={loadingHistory} style={{ fontSize: "0.875rem", padding: "0.5rem 1rem" }}>
              {loadingHistory ? "⏳ Loading..." : "🔄 Refresh"}
            </button>
            <button onClick={showDataLocation} style={{ fontSize: "0.875rem", padding: "0.5rem 1rem" }}>
              📁 Show Storage
            </button>
          </div>
        </div>
        {selectedSnapshots.length > 0 && (
          <div style={{ marginTop: "1rem" }}>
            <button onClick={handleCompare} disabled={selectedSnapshots.length !== 2}>
              Compare Selected ({selectedSnapshots.length}/2)
            </button>
          </div>
        )}
        {loadingHistory ? (
          <div className="loading-container">
            <div className="spinner"></div>
            <p>Loading scan history...</p>
          </div>
        ) : snapshots.length === 0 ? (
          <div className="empty-state">
            <p>No scans yet. Select a drive above to create your first snapshot.</p>
          </div>
        ) : (
          <div className="snapshots-grid">
            {snapshots.map((snapshot) => (
              <div key={snapshot.id} className={`snapshot-card ${selectedSnapshots.includes(snapshot.id) ? "selected" : ""}`} onClick={() => toggleSnapshot(snapshot.id)}>
                <div className="snapshot-header">
                  <strong>{snapshot.drive_path}</strong>
                  <button className="delete-btn" onClick={(e) => handleDelete(snapshot.id, e)}>
                    Delete
                  </button>
                </div>
                <div className="snapshot-info">
                  <div>{formatDate(snapshot.timestamp)}</div>
                  <div>{snapshot.total_files.toLocaleString()} files</div>
                  <div>{formatBytes(snapshot.total_size)}</div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {comparison && (
        <div className="compare-section">
          <h2>Comparison Results</h2>
          <div className="diff-summary">
            <div className="diff-stat">
              <span className="diff-stat-number added">{comparison.added.length}</span>
              <div>Added</div>
            </div>
            <div className="diff-stat">
              <span className="diff-stat-number deleted">{comparison.deleted.length}</span>
              <div>Deleted</div>
            </div>
            <div className="diff-stat">
              <span className="diff-stat-number modified">{comparison.modified.length}</span>
              <div>Modified</div>
            </div>
            <div className="diff-stat">
              <span className="diff-stat-number">{comparison.unchanged_count}</span>
              <div>Unchanged</div>
            </div>
          </div>

          <div className="diff-details">
            {comparison.added.length > 0 && (
              <div>
                <h3 className="added">Added Files ({comparison.added.length})</h3>
                <div className="diff-list">
                  {comparison.added.map((diff, idx) => (
                    <div key={idx} className="diff-item added">
                      + {diff.path} ({formatBytes(diff.new_size || 0)})
                    </div>
                  ))}
                </div>
              </div>
            )}

            {comparison.deleted.length > 0 && (
              <div>
                <h3 className="deleted">Deleted Files ({comparison.deleted.length})</h3>
                <div className="diff-list">
                  {comparison.deleted.map((diff, idx) => (
                    <div key={idx} className="diff-item deleted">
                      - {diff.path} ({formatBytes(diff.old_size || 0)})
                    </div>
                  ))}
                </div>
              </div>
            )}

            {comparison.modified.length > 0 && (
              <div>
                <h3 className="modified">Modified Files ({comparison.modified.length})</h3>
                <div className="diff-list">
                  {comparison.modified.map((diff, idx) => (
                    <div key={idx} className="diff-item modified">
                      ~ {diff.path}
                      <br />
                      &nbsp;&nbsp;Size: {formatBytes(diff.old_size || 0)} → {formatBytes(diff.new_size || 0)}
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
