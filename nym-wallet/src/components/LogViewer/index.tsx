import React, { FC, useEffect, useRef, useState } from 'react';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import {
  Box,
  Button,
  Chip,
  Paper,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
  useTheme,
} from '@mui/material';
import type { Theme } from '@mui/material/styles';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { useSnackbar } from 'notistack';
import { helpLogToggleWindow } from 'src/requests/logging';
import { Console } from 'src/utils/console';

// see https://github.com/tauri-apps/tauri-plugin-log/blob/dev/webview-src/index.ts#L4
enum LogLevel {
  Trace = 1,
  Debug,
  Info,
  Warn,
  Error,
}

const getLogLevelName = (value: LogLevel) => {
  switch (value) {
    case LogLevel.Trace:
      return 'Trace';
    case LogLevel.Debug:
      return 'Debug';
    case LogLevel.Info:
      return 'Info';
    case LogLevel.Warn:
      return 'Warn';
    case LogLevel.Error:
      return 'Error';
    default:
      return 'Unknown';
  }
};

const getLogLevelColor = (level: LogLevel, theme: Theme) => {
  switch (level) {
    case LogLevel.Trace:
      return {
        bg: '#e8f4f8',
        color: '#2c3e50',
        chipBg: '#e8f4f8',
      };
    case LogLevel.Debug:
      return {
        bg: '#e8f0f8',
        color: '#2c3e50',
        chipBg: '#e8f0f8',
      };
    case LogLevel.Info:
      return {
        bg: '#e8f8e8',
        color: '#2c3e50',
        chipBg: '#e8f8e8',
      };
    case LogLevel.Warn:
      return {
        bg: '#fff8e0',
        color: '#5d4037',
        chipBg: '#fff8e0',
      };
    case LogLevel.Error:
      return {
        bg: '#ffe8e8',
        color: '#c0392b',
        chipBg: '#ffe8e8',
      };
    default:
      return {
        bg: theme.palette.mode === 'dark' ? '#333' : '#f0f0f0',
        color: theme.palette.mode === 'dark' ? '#fff' : '#000',
        chipBg: theme.palette.mode === 'dark' ? '#444' : '#e0e0e0',
      };
  }
};

// see https://github.com/tauri-apps/tauri-plugin-log/blob/dev/webview-src/index.ts#L147
interface RecordPayload {
  level: LogLevel;
  message: string;
  timestamp?: number; // Adding timestamp for unique key generation
}

export const LogViewer: FC = () => {
  const theme = useTheme();
  const { enqueueSnackbar } = useSnackbar();
  const unlisten = useRef<UnlistenFn | null>(null);
  const [messages, setMessages] = useState<RecordPayload[]>([]);
  const [messageCount, setMessageCount] = useState(0);
  const tableRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;

    const setupListener = async () => {
      const unlistenFn = await getCurrentWebview().listen<RecordPayload>('log://log', (event) => {
        const { payload } = event;
        const payloadWithTimestamp = {
          ...payload,
          timestamp: Date.now(),
        };

        setMessages((prev) => [payloadWithTimestamp, ...prev]);
        setMessageCount((prev) => prev + 1);

        if (tableRef.current) {
          tableRef.current.scrollTop = 0;
        }
      });
      if (cancelled) {
        unlistenFn();
      } else {
        unlisten.current = unlistenFn;
      }
    };

    setupListener();

    return () => {
      cancelled = true;
      if (unlisten.current) {
        unlisten.current();
        unlisten.current = null;
      }
    };
  }, []);

  const handleCloseLogs = async () => {
    try {
      await helpLogToggleWindow();
    } catch (e) {
      Console.error(e);
      enqueueSnackbar('Could not close the log window', { variant: 'error' });
    }
  };

  const formatLine = (m: RecordPayload) => `${getLogLevelName(m.level)}\t${m.message}`;

  const handleCopyAll = async () => {
    if (messages.length === 0) return;
    try {
      const chronological = [...messages].reverse();
      await writeText(chronological.map(formatLine).join('\n'));
    } catch (e) {
      Console.error(e);
      enqueueSnackbar('Could not copy logs to the clipboard', { variant: 'error' });
    }
  };

  const handleCopyLatest = async () => {
    const latest = messages[0];
    if (!latest) return;
    try {
      await writeText(formatLine(latest));
    } catch (e) {
      Console.error(e);
      enqueueSnackbar('Could not copy to the clipboard', { variant: 'error' });
    }
  };

  const handleClearList = () => {
    setMessages([]);
    setMessageCount(0);
  };

  return (
    <Box
      sx={{
        height: '100vh',
        width: '100vw',
        display: 'grid',
        gridTemplateRows: '1fr auto',
        bgcolor: theme.palette.mode === 'dark' ? '#1e1e1e' : '#f5f5f5',
        color: theme.palette.mode === 'dark' ? '#ffffff' : '#000000',
      }}
    >
      <Box sx={{ overflowY: 'hidden', p: 2 }}>
        <TableContainer
          component={Paper}
          sx={{
            maxHeight: '100%',
            bgcolor: '#ffffff',
            boxShadow: theme.shadows[3],
            borderRadius: '4px',
          }}
          ref={tableRef}
        >
          <Table size="small" stickyHeader>
            <TableHead>
              <TableRow>
                <TableCell
                  sx={{
                    bgcolor: '#f0f0f0',
                    color: '#333333',
                    fontWeight: 'bold',
                    width: '120px',
                    borderBottom: '1px solid #e0e0e0',
                  }}
                >
                  Severity
                </TableCell>
                <TableCell
                  sx={{
                    bgcolor: '#f0f0f0',
                    color: '#333333',
                    fontWeight: 'bold',
                    borderBottom: '1px solid #e0e0e0',
                  }}
                >
                  Log message
                </TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {messages.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={2} sx={{ border: 'none', py: 4, bgcolor: '#fafafa' }}>
                    <Typography
                      variant="body2"
                      color="text.secondary"
                      sx={{ textAlign: 'center', maxWidth: 560, mx: 'auto' }}
                    >
                      No log lines yet. The wallet streams new lines here in real time - older log history is not
                      replayed. Use the wallet or set RUST_LOG for more detail. Close with the button below or use Open
                      logs again in Settings - Advanced (same action toggles this window).
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : (
                messages.map((m, index) => {
                  const levelColors = getLogLevelColor(m.level, theme);
                  return (
                    <TableRow
                      key={`log-${m.timestamp || index}`}
                      sx={{
                        bgcolor: levelColors.bg,
                        '&:hover': {
                          filter: 'brightness(0.95)',
                        },
                      }}
                    >
                      <TableCell
                        sx={{
                          padding: 1,
                          borderBottom: `1px solid ${theme.palette.divider}`,
                          width: '120px',
                          bgcolor: 'transparent',
                        }}
                      >
                        <Chip
                          label={getLogLevelName(m.level)}
                          variant="filled"
                          size="small"
                          sx={{
                            bgcolor: levelColors.chipBg,
                            color: levelColors.color,
                            fontWeight: 'medium',
                            minWidth: '70px',
                            border: '1px solid rgba(0,0,0,0.1)',
                          }}
                        />
                      </TableCell>
                      <TableCell
                        sx={{
                          padding: 1,
                          fontFamily: 'monospace',
                          fontSize: '0.875rem',
                          borderBottom: `1px solid ${theme.palette.divider}`,
                          color: levelColors.color,
                          bgcolor: 'transparent',
                        }}
                      >
                        {m.message}
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Box>
      <Box
        sx={{
          p: 1,
          borderTop: '1px solid #e0e0e0',
          bgcolor: '#ffffff',
          color: '#666666',
        }}
      >
        <Stack spacing={1}>
          <Stack direction="row" alignItems="center" justifyContent="space-between" spacing={2} flexWrap="wrap">
            <Typography variant="caption" sx={{ fontSize: '0.75rem' }}>
              {messageCount} log entries since opening this window
            </Typography>
            <Button variant="outlined" size="small" onClick={handleCloseLogs}>
              Close log window
            </Button>
          </Stack>
          <Stack direction="row" spacing={1} flexWrap="wrap" useFlexGap>
            <Button variant="text" size="small" onClick={handleCopyLatest} disabled={messages.length === 0}>
              Copy latest
            </Button>
            <Button variant="text" size="small" onClick={handleCopyAll} disabled={messages.length === 0}>
              Copy all
            </Button>
            <Button variant="text" size="small" onClick={handleClearList} disabled={messages.length === 0}>
              Clear list
            </Button>
          </Stack>
        </Stack>
      </Box>
    </Box>
  );
};
