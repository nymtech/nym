import React, { FC, useEffect, useRef, useState } from 'react';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { Box, Paper, Chip, Table, TableBody, TableCell, TableContainer, TableHead, TableRow } from '@mui/material';

// see https://github.com/tauri-apps/tauri-plugin-log/blob/dev/webview-@src/index.ts#L4
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

// see https://github.com/tauri-apps/tauri-plugin-log/blob/dev/webview-@src/index.ts#L147
interface RecordPayload {
  level: LogLevel;
  message: string;
}

export const LogViewer: FC = () => {
  const unlisten = useRef<UnlistenFn>();
  const messages = useRef<RecordPayload[]>([]);
  const [messageCount, setMessageCount] = useState(0);

  useEffect(() => {
    listen('log://log', (event) => {
      // eslint-disable-next-line no-console
      console.log(event.payload);
      const payload = event.payload as RecordPayload;
      messages.current.unshift(payload);
      setMessageCount((prev) => prev + 1);
    }).then((fn) => {
      unlisten.current = fn;
    });

    return () => {
      if (unlisten.current) {
        unlisten.current();
      }
    };
  }, []);

  return (
    <Box sx={{ height: '100vh', width: '100vw', display: 'grid', gridTemplateRows: '1fr auto' }}>
      <Box sx={{ overflowY: 'hidden', p: 2 }}>
        <TableContainer component={Paper} sx={{ maxHeight: '100%' }}>
          <Table size="small" stickyHeader>
            <TableHead>
              <TableRow>
                <TableCell>Severity</TableCell>
                <TableCell>Log message</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {messages.current.map((m) => (
                <TableRow sx={{ '&:last-child td, &:last-child th': { border: 0 } }}>
                  <TableCell sx={{ padding: 1 }}>
                    <Chip label={getLogLevelName(m.level)} variant="outlined" size="small" />
                  </TableCell>
                  <TableCell sx={{ padding: 1, fontFamily: 'Monospace' }}>{m.message}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      </Box>
      <Box
        sx={{
          p: 1,
          textAlign: 'right',
          fontSize: 'small',
          borderTop: '2px solid',
          borderTopColor: (theme) => theme.palette.divider,
        }}
      >
        {messageCount} log entries since opening this window
      </Box>
    </Box>
  );
};
