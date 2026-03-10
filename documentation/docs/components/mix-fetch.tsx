import React, { useState, useRef, useEffect } from "react";
import CircularProgress from "@mui/material/CircularProgress";
import Button from "@mui/material/Button";
import TextField from "@mui/material/TextField";
import Typography from "@mui/material/Typography";
import Box from "@mui/material/Box";
import { mixFetch, createMixFetch } from "@nymproject/mix-fetch-full-fat";
import Stack from "@mui/material/Stack";
import Paper from "@mui/material/Paper";
import type { SetupMixFetchOps } from "@nymproject/mix-fetch-full-fat";

const defaultUrl =
  "https://nymtech.net/.wellknown/network-requester/exit-policy.txt";
const args = { mode: "unsafe-ignore-cors" };
const mixFetchOptions: SetupMixFetchOps = {
  clientId: "docs-mixfetch-demo", // explicit ID
  preferredGateway: "q2A2cbooyC16YJzvdYaSMH9X3cSiieZNtfBr8cE8Fi1",
  mixFetchOverride: {
    requestTimeoutMs: 60_000,
  },
  forceTls: true, // force WSS
};

// Log entry type for the visible log panel
type LogLevel = "info" | "error" | "send" | "receive";
type LogEntry = { timestamp: string; message: string; level: LogLevel };

// Color map for log levels
const logColors: Record<LogLevel, string> = {
  info: "gray",
  error: "red",
  send: "blue",
  receive: "green",
};

// Label map for log levels
const logLabels: Record<LogLevel, string> = {
  info: "INFO",
  error: "ERROR",
  send: "SEND",
  receive: "RECV",
};

export const MixFetch = () => {
  // MixFetch initialization state
  const [status, setStatus] = useState<"idle" | "starting" | "ready" | "error">(
    "idle"
  );
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Log panel state
  const [logs, setLogs] = useState<LogEntry[]>([]);

  // Single fetch state
  const [url, setUrl] = useState<string>(defaultUrl);
  const [html, setHtml] = useState<string>();
  const [busy, setBusy] = useState<boolean>(false);

  // Concurrent fetch state
  const [concurrentResults, setConcurrentResults] = useState<string[]>([]);
  const [concurrentBusy, setConcurrentBusy] = useState<boolean>(false);

  // Auto-scroll within the log panel when new entries are added (without scrolling the page)
  const logContainerRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
    }
  }, [logs]);

  // Helper to add a timestamped log entry
  const addLog = (message: string, level: LogLevel) => {
    const timestamp = new Date().toISOString().substring(11, 23); // HH:MM:SS.mmm
    setLogs((prev) => [...prev, { timestamp, message, level }]);
  };

  // Initialize MixFetch explicitly via createMixFetch
  const handleStart = async () => {
    try {
      setStatus("starting");
      setErrorMsg(null);
      addLog("Starting MixFetch...", "info");
      await createMixFetch(mixFetchOptions);
      setStatus("ready");
      addLog("MixFetch is ready!", "info");
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setStatus("error");
      setErrorMsg(msg);
      addLog(`Error: ${msg}`, "error");
    }
  };

  // Single URL fetch — mixFetch reuses the existing singleton
  const handleFetch = async () => {
    try {
      setBusy(true);
      setHtml(undefined);
      addLog(`Sending request to ${url}...`, "send");
      const response = await mixFetch(url, args, mixFetchOptions);
      const resHtml = await response.text();
      setHtml(resHtml);
      addLog(`Response received (${resHtml.length} bytes)`, "receive");
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      addLog(`Fetch error: ${msg}`, "error");
    } finally {
      setBusy(false);
    }
  };

  // Send 5 concurrent requests to different URLs on the same domain
  const handleConcurrentFetch = async () => {
    const baseUrl = "https://jsonplaceholder.typicode.com/posts/";
    const count = 5;
    try {
      setConcurrentBusy(true);
      setConcurrentResults([]);
      addLog(
        `Starting ${count} concurrent requests to ${baseUrl}1-${count}...`,
        "send"
      );
      // Fire off all requests concurrently using Promise.all
      const requests = Array.from({ length: count }, (_, i) => {
        const targetUrl = `${baseUrl}${i + 1}`;
        return mixFetch(targetUrl, args, mixFetchOptions)
          .then((res) => res.json())
          .then((json: { id: number; title: string }) => {
            const entry = `[${json.id}] ${json.title}`;
            addLog(entry, "receive");
            return entry;
          });
      });
      const results = await Promise.all(requests);
      setConcurrentResults(results);
      addLog(`All ${count} concurrent requests completed!`, "info");
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      addLog(`Concurrent fetch error: ${msg}`, "error");
    } finally {
      setConcurrentBusy(false);
    }
  };

  // Are fetch controls enabled?
  const isReady = status === "ready";

  // Status text + color for the startup indicator
  const statusText: Record<typeof status, string> = {
    idle: "Not started",
    starting: "Starting...",
    ready: "Ready",
    error: `Error: ${errorMsg}`,
  };
  const statusColor: Record<typeof status, string> = {
    idle: "gray",
    starting: "orange",
    ready: "green",
    error: "red",
  };

  return (
    <div style={{ marginTop: "1rem" }}>
      {/* --- Start MixFetch Section --- */}
      <Paper sx={{ p: 2, mb: 2 }} variant="outlined">
        <Stack direction="row" alignItems="center" spacing={2}>
          <Button
            variant="contained"
            disabled={status === "starting" || status === "ready"}
            onClick={handleStart}
          >
            Start MixFetch
          </Button>
          {status === "starting" && <CircularProgress size={20} />}
          <Typography
            fontFamily="monospace"
            fontSize="small"
            sx={{ color: statusColor[status] }}
          >
            {statusText[status]}
          </Typography>
        </Stack>
      </Paper>

      {/* --- Fetch Controls (disabled until ready) --- */}
      <Box
        sx={{
          opacity: isReady ? 1 : 0.5,
          pointerEvents: isReady ? "auto" : "none",
        }}
      >
        {/* Single fetch */}
        <Stack direction="row">
          <TextField
            disabled={busy}
            fullWidth
            label="URL"
            type="text"
            variant="outlined"
            defaultValue={defaultUrl}
            onChange={(e) => setUrl(e.target.value)}
          />
          <Button
            variant="outlined"
            disabled={busy}
            sx={{ marginLeft: "1rem" }}
            onClick={handleFetch}
          >
            Fetch
          </Button>
        </Stack>
        {busy && (
          <Box mt={2}>
            <CircularProgress />
          </Box>
        )}
        {html && (
          <>
            <Box mt={2}>
              <strong>Response</strong>
            </Box>
            <Paper sx={{ p: 2, mt: 1 }} elevation={4}>
              <Typography fontFamily="monospace" fontSize="small">
                {html}
              </Typography>
            </Paper>
          </>
        )}

        {/* Concurrent fetch demo */}
        <Box mt={3}>
          <strong>Concurrent Requests</strong>
          <Box mt={1}>
            <Button
              variant="outlined"
              disabled={concurrentBusy}
              onClick={handleConcurrentFetch}
            >
              Send 5 Concurrent Requests (posts/1-5)
            </Button>
          </Box>
        </Box>
        {concurrentBusy && (
          <Box mt={2}>
            <CircularProgress />
          </Box>
        )}
        {concurrentResults.length > 0 && (
          <Paper sx={{ p: 2, mt: 2 }} elevation={4}>
            {concurrentResults.map((result, i) => (
              <Typography key={i} fontFamily="monospace" fontSize="small">
                {result}
              </Typography>
            ))}
          </Paper>
        )}
      </Box>

      {/* --- Log Panel --- */}
      {logs.length > 0 && (
        <Paper
          ref={logContainerRef}
          sx={{ p: 2, mt: 3, maxHeight: 200, overflow: "auto" }}
          variant="outlined"
        >
          <strong>Log</strong>
          {logs.map((entry, i) => (
            <Typography
              key={i}
              fontFamily="monospace"
              fontSize="small"
              sx={{ color: logColors[entry.level] }}
            >
              {entry.timestamp} [{logLabels[entry.level]}] {entry.message}
            </Typography>
          ))}
        </Paper>
      )}
    </div>
  );
};
