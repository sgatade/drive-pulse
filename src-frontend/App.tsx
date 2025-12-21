import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/api/dialog";
import { Container, Box, Typography, Button, Select, MenuItem, TextField, FormControlLabel, Checkbox, Paper, Grid, Card, CardContent, CardActions, IconButton, Alert, LinearProgress, Chip, Stack, InputLabel, FormControl, Divider, CircularProgress, Table, TableBody, TableCell, TableContainer, TableHead, TableRow } from "@mui/material";
import { Search as SearchIcon, Refresh as RefreshIcon, Delete as DeleteIcon, Folder as FolderIcon, CompareArrows as CompareArrowsIcon, Lock as LockIcon, Storage as StorageIcon, FolderOpen as FolderOpenIcon, Add as AddIcon, Remove as RemoveIcon, Edit as EditIcon } from "@mui/icons-material";

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
  scan_duration: number;
  files: FileEntry[];
}

interface SnapshotSummary {
  id: string;
  drive_path: string;
  timestamp: number;
  total_files: number;
  total_size: number;
  scan_duration: number;
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
  const [loadingDrives, setLoadingDrives] = useState(true);
  const [encrypt, setEncrypt] = useState(false);
  const [password, setPassword] = useState("");
  const [successMessage, setSuccessMessage] = useState("");
  const scanningRef = useRef(false);

  useEffect(() => {
    loadHistory();
    loadDrives();

    const unlisten = listen<ScanProgress>("scan-progress", (event) => {
      // Only update progress if we're currently scanning
      if (scanningRef.current) {
        setScanProgress(event.payload);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const loadDrives = async () => {
    setLoadingDrives(true);
    try {
      const drives = await invoke<DriveInfo[]>("get_available_drives");
      setAvailableDrives(drives);
    } catch (err) {
      console.error("Failed to load drives:", err);
    } finally {
      setLoadingDrives(false);
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
    scanningRef.current = true;
    setScanProgress(null);
    setError("");

    try {
      const snapshot = await invoke<Snapshot>("scan_drive", {
        drivePath,
        encrypt,
        password: encrypt ? password : null,
      });

      // Explicitly clear scanning state and progress before saving
      scanningRef.current = false;
      setScanning(false);
      setScanProgress(null);
      setSaving(true);

      await new Promise((resolve) => setTimeout(resolve, 100));
      await loadHistory();

      const message = `Scan complete! ${snapshot.total_files.toLocaleString()} files scanned (${formatBytes(snapshot.total_size)})${encrypt ? " - Encrypted" : ""}`;
      setSuccessMessage(message);

      setTimeout(() => setSuccessMessage(""), 5000);

      setDrivePath("");
      setPassword("");
    } catch (err) {
      setError(`Scan failed: ${err}`);
    } finally {
      scanningRef.current = false;
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

  const formatDuration = (seconds: number) => {
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    if (minutes < 60) return `${minutes}m ${secs}s`;
    const hours = Math.floor(minutes / 60);
    const mins = minutes % 60;
    return `${hours}h ${mins}m`;
  };

  return (
    <Container maxWidth="xl" sx={{ py: 2, px: 2 }}>
      {/* Header */}
      <Box sx={{ mb: 2 }}>
        <Typography variant="h4" component="h1" sx={{ display: "flex", alignItems: "center", gap: 1, mb: 0.5 }}>
          <SearchIcon /> Drive Pulse
        </Typography>
        <Typography variant="body2" color="text.secondary">
          Scan drives and compare snapshots
        </Typography>
      </Box>

      {/* Alerts */}
      {error && (
        <Alert severity="error" onClose={() => setError("")} sx={{ mb: 1.5 }}>
          {error}
        </Alert>
      )}
      {successMessage && (
        <Alert severity="success" onClose={() => setSuccessMessage("")} sx={{ mb: 1.5 }}>
          {successMessage}
        </Alert>
      )}

      {/* Scan Section */}
      <Paper elevation={2} sx={{ p: 2, mb: 2 }}>
        <Typography variant="h6" sx={{ mb: 1.5 }}>
          New Scan
        </Typography>

        <Stack spacing={1.5}>
          <Box sx={{ display: "flex", gap: 1, alignItems: "center", flexWrap: "wrap" }}>
            <FormControl size="small" sx={{ minWidth: 200 }}>
              <InputLabel>Select Drive</InputLabel>
              <Select
                value={drivePath}
                onChange={(e) => {
                  const selectedValue = e.target.value as string;
                  console.log("onChange fired:", selectedValue);
                  setDrivePath(selectedValue);
                }}
                disabled={scanning || loadingDrives}
                label="Select Drive"
                size="small"
              >
                <MenuItem value="">
                  {loadingDrives ? (
                    <>
                      <CircularProgress size={16} sx={{ mr: 1 }} />
                      Loading drives...
                    </>
                  ) : (
                    "Select a drive..."
                  )}
                </MenuItem>
                {availableDrives.map((drive) => (
                  <MenuItem key={drive.path} value={drive.path}>
                    {drive.label}
                  </MenuItem>
                ))}
                <MenuItem value="custom">Custom Path...</MenuItem>
              </Select>
            </FormControl>

            <IconButton onClick={loadDrives} disabled={scanning || loadingDrives} color="primary" title="Refresh drives list">
              {loadingDrives ? <CircularProgress size={24} /> : <RefreshIcon />}
            </IconButton>

            {drivePath === "custom" && (
              <>
                <TextField value={drivePath !== "custom" ? drivePath : ""} onChange={(e) => setDrivePath(e.target.value)} placeholder="Enter custom path..." sx={{ minWidth: 250 }} />
                <Button onClick={selectFolder} variant="outlined" startIcon={<FolderOpenIcon />}>
                  Browse
                </Button>
              </>
            )}

            <FormControlLabel
              control={<Checkbox checked={encrypt} onChange={(e) => setEncrypt(e.target.checked)} disabled={scanning} />}
              label={
                <Box sx={{ display: "flex", alignItems: "center", gap: 0.5 }}>
                  <LockIcon fontSize="small" /> Encrypt
                </Box>
              }
            />

            {encrypt && <TextField type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="Encryption password..." disabled={scanning} size="small" sx={{ minWidth: 200 }} />}

            <Button onClick={handleScan} disabled={scanning || !drivePath || drivePath === "custom"} variant="contained" startIcon={<SearchIcon />}>
              {scanning ? "Scanning..." : "Scan Drive"}
            </Button>
          </Box>

          {/* Progress */}
          {scanningRef.current && scanProgress && (
            <Paper variant="outlined" sx={{ p: 2 }}>
              <LinearProgress sx={{ mb: 2 }} />
              <Stack spacing={1}>
                <Typography variant="body2">
                  <strong>Files scanned:</strong> {scanProgress.files_scanned.toLocaleString()}
                </Typography>
                <Typography variant="body2">
                  <strong>Total size:</strong> {formatBytes(scanProgress.total_size)}
                </Typography>
                <Typography variant="caption" color="text.secondary" sx={{ wordBreak: "break-all" }}>
                  <strong>Current:</strong> {scanProgress.current_path}
                </Typography>
              </Stack>
            </Paper>
          )}

          {saving && (
            <Paper variant="outlined" sx={{ p: 2, display: "flex", alignItems: "center", gap: 2, justifyContent: "center" }}>
              <CircularProgress size={24} />
              <Typography variant="body2">
                <strong>Saving snapshot to disk...</strong>
              </Typography>
            </Paper>
          )}
        </Stack>
      </Paper>

      {/* History Section */}
      <Paper elevation={2} sx={{ p: 2, mb: 2 }}>
        <Box sx={{ display: "flex", justifyContent: "space-between", alignItems: "center", mb: 1.5 }}>
          <Typography variant="h6">Scan History ({snapshots.length})</Typography>
          <Stack direction="row" spacing={1}>
            <Button onClick={handleCompare} disabled={selectedSnapshots.length !== 2} variant="contained" startIcon={<CompareArrowsIcon />} size="small">
              Compare ({selectedSnapshots.length}/2)
            </Button>
            <Button onClick={loadHistory} disabled={loadingHistory} startIcon={<RefreshIcon />} size="small">
              Refresh
            </Button>
            <Button onClick={showDataLocation} startIcon={<FolderOpenIcon />} size="small">
              Show Storage
            </Button>
          </Stack>
        </Box>

        {loadingHistory ? (
          <Box sx={{ textAlign: "center", py: 2 }}>
            <CircularProgress />
            <Typography variant="body2" sx={{ mt: 2 }}>
              Loading scan history...
            </Typography>
          </Box>
        ) : snapshots.length === 0 ? (
          <Box sx={{ textAlign: "center", py: 2, color: "text.secondary" }}>
            <Typography>No scans yet. Select a drive above to create your first snapshot.</Typography>
          </Box>
        ) : (
          <TableContainer>
            <Table size="small">
              <TableHead>
                <TableRow>
                  <TableCell padding="checkbox"></TableCell>
                  <TableCell>Drive Path</TableCell>
                  <TableCell>Timestamp</TableCell>
                  <TableCell align="right">Files</TableCell>
                  <TableCell align="right">Size</TableCell>
                  <TableCell align="right">Duration</TableCell>
                  <TableCell align="center">Actions</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {snapshots.map((snapshot) => (
                  <TableRow
                    key={snapshot.id}
                    hover
                    onClick={() => toggleSnapshot(snapshot.id)}
                    sx={{
                      cursor: "pointer",
                      bgcolor: selectedSnapshots.includes(snapshot.id) ? "primary.50" : "inherit",
                      "&.MuiTableRow-hover:hover": {
                        bgcolor: selectedSnapshots.includes(snapshot.id) ? "primary.100" : "action.hover",
                      },
                    }}
                  >
                    <TableCell padding="checkbox">
                      <Checkbox checked={selectedSnapshots.includes(snapshot.id)} onChange={() => toggleSnapshot(snapshot.id)} onClick={(e) => e.stopPropagation()} />
                    </TableCell>
                    <TableCell>
                      <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                        <StorageIcon fontSize="small" color="action" />
                        {snapshot.drive_path}
                      </Box>
                    </TableCell>
                    <TableCell>{formatDate(snapshot.timestamp)}</TableCell>
                    <TableCell align="right">{snapshot.total_files.toLocaleString()}</TableCell>
                    <TableCell align="right">{formatBytes(snapshot.total_size)}</TableCell>
                    <TableCell align="right">{formatDuration(snapshot.scan_duration)}</TableCell>
                    <TableCell align="center">
                      <IconButton onClick={(e) => handleDelete(snapshot.id, e)} color="error" size="small">
                        <DeleteIcon />
                      </IconButton>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </TableContainer>
        )}
      </Paper>

      {/* Comparison Results */}
      {comparison && (
        <Paper elevation={2} sx={{ p: 2 }}>
          <Typography variant="h6" sx={{ mb: 1.5 }}>
            Comparison Results
          </Typography>

          <Grid container spacing={1} sx={{ mb: 2 }}>
            <Grid item xs={6} sm={3}>
              <Paper sx={{ p: 1.5, textAlign: "center", bgcolor: "success.50" }}>
                <Typography variant="h5" color="success.main">
                  {comparison.added.length}
                </Typography>
                <Typography variant="caption">Added</Typography>
              </Paper>
            </Grid>
            <Grid item xs={6} sm={3}>
              <Paper sx={{ p: 1.5, textAlign: "center", bgcolor: "error.50" }}>
                <Typography variant="h5" color="error.main">
                  {comparison.deleted.length}
                </Typography>
                <Typography variant="body2">Deleted</Typography>
              </Paper>
            </Grid>
            <Grid item xs={6} sm={3}>
              <Paper sx={{ p: 1.5, textAlign: "center", bgcolor: "warning.50" }}>
                <Typography variant="h5" color="warning.main">
                  {comparison.modified.length}
                </Typography>
                <Typography variant="caption">Modified</Typography>
              </Paper>
            </Grid>
            <Grid item xs={6} sm={3}>
              <Paper sx={{ p: 1.5, textAlign: "center", bgcolor: "grey.100" }}>
                <Typography variant="h5">{comparison.unchanged_count}</Typography>
                <Typography variant="caption">Unchanged</Typography>
              </Paper>
            </Grid>
          </Grid>

          <Stack spacing={2}>
            {comparison.added.length > 0 && (
              <Box>
                <Typography variant="subtitle1" color="success.main" sx={{ display: "flex", alignItems: "center", gap: 0.5, mb: 1 }}>
                  <AddIcon fontSize="small" /> Added Files ({comparison.added.length})
                </Typography>
                <Paper variant="outlined" sx={{ p: 1.5, maxHeight: 250, overflow: "auto" }}>
                  {comparison.added.map((diff, idx) => (
                    <Typography key={idx} variant="body2" sx={{ py: 0.5, fontFamily: "monospace" }}>
                      + {diff.path} ({formatBytes(diff.new_size || 0)})
                    </Typography>
                  ))}
                </Paper>
              </Box>
            )}

            {comparison.deleted.length > 0 && (
              <Box>
                <Typography variant="subtitle1" color="error.main" sx={{ display: "flex", alignItems: "center", gap: 0.5, mb: 1 }}>
                  <Remove fontSize="small" /> Deleted Files ({comparison.deleted.length})
                </Typography>
                <Paper variant="outlined" sx={{ p: 1.5, maxHeight: 250, overflow: "auto" }}>
                  {comparison.deleted.map((diff, idx) => (
                    <Typography key={idx} variant="body2" sx={{ py: 0.5, fontFamily: "monospace" }}>
                      - {diff.path} ({formatBytes(diff.old_size || 0)})
                    </Typography>
                  ))}
                </Paper>
              </Box>
            )}

            {comparison.modified.length > 0 && (
              <Box>
                <Typography variant="subtitle1" color="warning.main" sx={{ display: "flex", alignItems: "center", gap: 0.5, mb: 1 }}>
                  <EditIcon fontSize="small" /> Modified Files ({comparison.modified.length})
                </Typography>
                <Paper variant="outlined" sx={{ p: 1.5, maxHeight: 250, overflow: "auto" }}>
                  {comparison.modified.map((diff, idx) => (
                    <Box key={idx} sx={{ py: 0.5 }}>
                      <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                        ~ {diff.path}
                      </Typography>
                      <Typography variant="caption" sx={{ pl: 2, color: "text.secondary", fontFamily: "monospace" }}>
                        Size: {formatBytes(diff.old_size || 0)} → {formatBytes(diff.new_size || 0)}
                      </Typography>
                    </Box>
                  ))}
                </Paper>
              </Box>
            )}
          </Stack>
        </Paper>
      )}
    </Container>
  );
}

export default App;
