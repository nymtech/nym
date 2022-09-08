import React, { FC, useEffect, useRef, useState } from 'react';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { Box, Paper, Stack, Table, TableBody, TableCell, TableContainer, TableRow } from '@mui/material';

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

// see https://github.com/tauri-apps/tauri-plugin-log/blob/dev/webview-src/index.ts#L147
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
    <Stack direction="column" p={2}>
      <Box>{messageCount} logs</Box>
      <Box>
        <hr />
      </Box>
      <TableContainer component={Paper}>
        <Table size="small">
          <TableBody>
            {messages.current.map((m) => (
              <TableRow sx={{ '&:last-child td, &:last-child th': { border: 0 } }}>
                <TableCell sx={{ padding: 0 }}>
                  <strong>{getLogLevelName(m.level)}</strong>
                </TableCell>
                <TableCell sx={{ padding: 0 }}>
                  <pre>{m.message}</pre>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </TableContainer>
    </Stack>
  );
};
