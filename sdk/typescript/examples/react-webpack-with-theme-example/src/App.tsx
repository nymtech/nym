import * as React from 'react';
import {
  Box,
  Button,
  Chip,
  CircularProgress,
  Container,
  FormControlLabel,
  FormGroup,
  InputAdornment,
  Link,
  Stack,
  Switch,
  TextField,
  Tooltip,
  Typography,
} from '@mui/material';
import DownloadForOfflineIcon from '@mui/icons-material/DownloadForOffline';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import CallMadeIcon from '@mui/icons-material/CallMade';
import CallReceivedIcon from '@mui/icons-material/CallReceived';
import PersonIcon from '@mui/icons-material/Person';
import PersonOffIcon from '@mui/icons-material/PersonOff';
import ErrorIcon from '@mui/icons-material/Error';
import VisibilityIcon from '@mui/icons-material/Visibility';
import VisibilityOffIcon from '@mui/icons-material/VisibilityOff';
import { NymLogo } from '@nymproject/react/logo/NymLogo';
import { NymThemeProvider } from '@nymproject/mui-theme';
import { useTheme } from '@mui/material/styles';
import { useClipboard } from 'use-clipboard-copy';
import { DropzoneDialog } from 'react-mui-dropzone';
import UploadFileIcon from '@mui/icons-material/UploadFile';
import ArticleIcon from '@mui/icons-material/Article';
import InsertDriveFileIcon from '@mui/icons-material/InsertDriveFile';
import { Headers } from '@nymproject/sdk';
import { ThemeToggle } from './ThemeToggle';
import { AppContextProvider, useAppContext } from './context';
import { BinaryMessageHeaders, MixnetContextProvider, useMixnetContext } from './context/mixnet';

export const AppTheme: React.FC = ({ children }) => {
  const { mode } = useAppContext();

  return <NymThemeProvider mode={mode}>{children}</NymThemeProvider>;
};

interface Log {
  kind: 'tx' | 'rx' | 'error';
  message?: string;
  filename?: string;
  fileDownloadUrl?: string;
  filesize?: number;
  timestamp: Date;
  headers?: Headers;
}

interface UploadState {
  dialogOpen: boolean;
  files: File[];
}

const ClientAddress: React.FC<{ label?: string; tooltip?: string; address?: string }> = ({
  label,
  address,
  tooltip,
}) => {
  const copy = useClipboard();

  if (!address) {
    return <Chip label="Anonymous" icon={<VisibilityOffIcon />} />;
  }

  const addressShort = `${address.slice(0, 24)}...`;

  return (
    <Tooltip arrow title={tooltip || ''}>
      <Chip
        clickable
        label={
          label ? (
            <>
              <strong>{label}</strong> {addressShort}
            </>
          ) : (
            <>{addressShort}</>
          )
        }
        onClick={() => {
          if (address) {
            copy.copy(address);
          }
        }}
        icon={<ContentCopyIcon />}
      />
    </Tooltip>
  );
};

export const Content: React.FC = () => {
  const theme = useTheme();
  const { isReady, address, connect, events, sendTextMessage, sendBinaryMessage } = useMixnetContext();

  const [revealSenderAddress, setRevealSenderAddress] = React.useState(false);
  const [sendToSelf, setSendToSelf] = React.useState(false);
  const [recipient, setRecipient] = React.useState<string>();
  const handleRecipientChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setRecipient(event.target.value);
  };

  const [message, setMessage] = React.useState<string>('This is a test message');
  const handleMessageChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setMessage(event.target.value);
  };

  const log = React.useRef<Log[]>([]);
  const [_logTrigger, setLogTrigger] = React.useState(Date.now());

  const [uploadState, setUploadState] = React.useState<UploadState>({
    dialogOpen: false,
    files: [],
  });

  const handleUploadClick = () => {
    setUploadState((prev) => ({ ...prev, dialogOpen: true }));
  };

  const handleUploadClose = () => {
    setUploadState((prev) => ({ ...prev, dialogOpen: false }));
  };

  const handleUploadSave = (files: File[]) => {
    setUploadState({ files, dialogOpen: false });
  };

  const handleUploadDeleted = (file: File) => () => {
    setUploadState((prev) => ({ ...prev, files: prev.files.filter((f) => f !== file) }));
  };

  React.useEffect(() => {
    if (isReady) {
      const validatorApiUrl = 'https://validator.nymtech.net/api';
      const preferredGatewayIdentityKey = 'E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM';

      connect({
        clientId: 'Example Client',
        validatorApiUrl,
        preferredGatewayIdentityKey,
      });
    }
  }, [isReady]);

  React.useEffect(() => {
    if (events) {
      const unsubcribe = events.subscribeToTextMessageReceivedEvent((e) => {
        log.current.push({
          kind: 'rx',
          timestamp: new Date(),
          message: e.args.payload,
          headers: e.args.headers,
        });
        setLogTrigger(Date.now());
      });

      // cleanup on unmount
      return unsubcribe;
    }

    // no cleanup
    return undefined;
  }, [events]);

  const addErrorLog = (errorMessage: string, ...args: any[]) => {
    log.current.push({
      kind: 'error',
      timestamp: new Date(),
      message: errorMessage,
    });
    console.error(errorMessage, args);
    setLogTrigger(Date.now());
  };

  React.useEffect(() => {
    if (events) {
      const unsubcribe = events.subscribeToBinaryMessageReceivedEvent((e) => {
        if (!e.args.headers) {
          console.error('Expected headers, got undefined 😢', e.args);
          return;
        }

        const headers = e.args.headers as BinaryMessageHeaders;

        const blob = new Blob([new Uint8Array(e.args.payload)], { type: headers.mimeType });
        log.current.push({
          kind: 'rx',
          timestamp: new Date(),
          filename: headers.filename,
          fileDownloadUrl: URL.createObjectURL(blob),
          filesize: e.args.payload.length,
          headers: e.args.headers,
        });
        setLogTrigger(Date.now());
      });

      // cleanup on unmount
      return unsubcribe;
    }

    // no cleanup
    return undefined;
  }, [events]);

  const handleSend = async () => {
    if (!message) {
      addErrorLog('No message set');
      return;
    }
    if (!recipient) {
      addErrorLog('No recipient set');
      return;
    }

    // copy the details of any files waiting to be sent, and then reset the state
    const files = [...uploadState.files];
    setUploadState((prev) => ({ ...prev, files: [] }));

    log.current.push({
      kind: 'tx',
      timestamp: new Date(),
      message,
    });
    setLogTrigger(Date.now());
    const senderAddress = revealSenderAddress ? address : undefined;
    await sendTextMessage({ payload: message, recipient, headers: { senderAddress } });

    await Promise.all(
      files.map(async (f) => {
        log.current.push({
          kind: 'tx',
          timestamp: new Date(),
          filename: f.name,
          filesize: f.size,
        });
        setLogTrigger(Date.now());
        const buffer = await f.arrayBuffer();
        try {
          return await sendBinaryMessage({
            payload: new Uint8Array(buffer),
            recipient,
            headers: { filename: f.name, mimeType: f.type, senderAddress },
          });
        } catch (e) {
          addErrorLog('Failed to send file', f.name);
        }
        return undefined;
      }),
    );
  };

  const logKindToColor = React.useCallback(
    (kind: 'tx' | 'rx' | 'error') => {
      switch (kind) {
        case 'tx':
          return theme.palette.success.main;
        case 'rx':
          return theme.palette.info.main;
        case 'error':
          return theme.palette.error.main;
        default:
          return theme.palette.text.primary;
      }
    },
    [theme],
  );

  return (
    <Container sx={{ py: 4 }}>
      <Box display="flex" flexDirection="row-reverse" pb={2}>
        <ThemeToggle />
      </Box>
      <NymLogo height={50} />
      <h1>Nym Mixnet Chat App</h1>
      <Box mb={5}>
        <Typography>
          This is an example app that uses React, Typescript, Webpack and the Nym theme + components with the WASM
          Mixnet Client.
        </Typography>
      </Box>
      <Box mb={4}>
        <Stack direction="row" spacing={2} alignItems="center">
          {!isReady ? (
            <>
              <CircularProgress size={theme.typography.fontSize * 1.5} />
              <Typography>Connecting...</Typography>
            </>
          ) : (
            <>
              <Chip color="success" icon={<CheckCircleIcon />} label="Connected" variant="outlined" />
              <ClientAddress address={address} tooltip="Copy your client address to the clipboard" />
            </>
          )}
        </Stack>
        {isReady && address && (
          <Stack direction="column" mt={6} spacing={4}>
            {!sendToSelf ? (
              <TextField
                id="recipient"
                label="Recipient address"
                required
                value={recipient}
                onChange={handleRecipientChange}
                InputLabelProps={{ shrink: true }}
                InputProps={{
                  endAdornment: (
                    <InputAdornment position="end">
                      <Tooltip title="Use your own address to send messages to yourself" arrow>
                        <PersonIcon
                          sx={{ cursor: 'pointer' }}
                          onClick={() => {
                            if (address) {
                              setSendToSelf(true);
                              setRecipient(address);
                            }
                          }}
                        />
                      </Tooltip>
                    </InputAdornment>
                  ),
                }}
              />
            ) : (
              <TextField
                id="recipientSendToSelf"
                label="Send to your address"
                value={address}
                onChange={() => undefined}
                InputLabelProps={{ shrink: true }}
                InputProps={{
                  readOnly: true,
                  endAdornment: (
                    <InputAdornment position="end">
                      <Tooltip title="Click to use another address" arrow>
                        <PersonOffIcon
                          sx={{ cursor: 'pointer' }}
                          onClick={() => {
                            setSendToSelf(false);
                          }}
                        />
                      </Tooltip>
                    </InputAdornment>
                  ),
                }}
              />
            )}
            <TextField
              id="message"
              required
              label="Enter some text to send"
              multiline
              rows={4}
              value={message}
              onChange={handleMessageChange}
            />

            <Box>
              <Stack direction="row" spacing={1}>
                <Button variant="outlined" onClick={handleUploadClick} color="secondary">
                  <UploadFileIcon sx={{ mr: 1 }} />
                  Attach file
                </Button>
                {uploadState.files.map((file) => (
                  <Chip key={file.name} label={file.name} onDelete={handleUploadDeleted(file)} />
                ))}
              </Stack>
              <DropzoneDialog
                open={uploadState.dialogOpen}
                onSave={handleUploadSave}
                showPreviews
                maxFileSize={5_000_000}
                onClose={handleUploadClose}
              />
            </Box>

            <Stack direction="row" spacing={2}>
              <Button variant="contained" sx={{ width: 100 }} onClick={handleSend}>
                Send
              </Button>
              <FormGroup>
                <FormControlLabel
                  control={
                    <Switch
                      color={revealSenderAddress ? 'warning' : 'default'}
                      onClick={() => setRevealSenderAddress((prevState) => !prevState)}
                    />
                  }
                  label={
                    revealSenderAddress ? (
                      <Stack direction="row" spacing={1}>
                        <VisibilityIcon color="warning" />
                        <Typography color={theme.palette.warning.main}>Reveal your address to the recipient</Typography>
                      </Stack>
                    ) : (
                      <Stack direction="row" spacing={1}>
                        <VisibilityOffIcon />
                        <Typography>Hide your address from the recipient</Typography>
                      </Stack>
                    )
                  }
                />
              </FormGroup>
            </Stack>
          </Stack>
        )}
      </Box>
      {log.current.map((item) => (
        <Box key={item.kind + item.timestamp.toISOString()}>
          <Stack direction="row" spacing={2} alignItems="start" sx={{ color: logKindToColor(item.kind) }}>
            {item.kind === 'tx' && <CallMadeIcon />}
            {item.kind === 'rx' && <CallReceivedIcon />}
            {item.kind === 'error' && <ErrorIcon />}
            <Chip variant="outlined" label={item.timestamp.toLocaleTimeString()} />
            {item.message && (
              <>
                <ArticleIcon />
                <Typography>{item.message}</Typography>
              </>
            )}
            {item.filename && (
              <>
                <InsertDriveFileIcon />
                {!item.fileDownloadUrl ? (
                  <Typography>
                    {item.filename} ({item.filesize!} bytes)
                  </Typography>
                ) : (
                  <>
                    <Tooltip title="Open in another tab" arrow>
                      <Link color="inherit" target="_blank" href={item.fileDownloadUrl}>
                        <Typography>
                          {item.filename} ({item.filesize!} bytes)
                        </Typography>
                      </Link>
                    </Tooltip>
                    <Tooltip title="Download the file" arrow>
                      <Link
                        color="inherit"
                        href={item.fileDownloadUrl}
                        onClick={(e) => {
                          e.preventDefault();
                          downloadBlob(item.fileDownloadUrl!, item.filename!);
                        }}
                      >
                        <DownloadForOfflineIcon />
                      </Link>
                    </Tooltip>
                  </>
                )}
              </>
            )}
            {item.kind === 'rx' &&
              (item.headers?.senderAddress ? (
                <ClientAddress
                  label="Sender"
                  tooltip="Click to copy the message sender's address"
                  address={item.headers?.senderAddress}
                />
              ) : (
                <ClientAddress label="Sender" />
              ))}
          </Stack>
        </Box>
      ))}
    </Container>
  );
};

export const App: React.FC = () => (
  <AppContextProvider>
    <MixnetContextProvider>
      <AppTheme>
        <Content />
      </AppTheme>
    </MixnetContextProvider>
  </AppContextProvider>
);

function downloadBlob(fileDownloadUrl: string, filename: string) {
  // Create a link element
  const link = document.createElement('a');

  // Set link's href to point to the Blob URL
  link.href = fileDownloadUrl;
  link.download = filename;

  // Append link to the body
  document.body.appendChild(link);

  // Dispatch click event on the link
  // This is necessary as link.click() does not work on the latest firefox
  link.dispatchEvent(
    new MouseEvent('click', {
      bubbles: true,
      cancelable: true,
      view: window,
    }),
  );

  // Remove link from body
  document.body.removeChild(link);
}
