import * as React from 'react';
import {
  Box,
  Chip,
  CircularProgress,
  Container,
  Stack,
  Tooltip,
  Typography,
  TextField,
  Button,
  InputAdornment,
} from '@mui/material';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import CallMadeIcon from '@mui/icons-material/CallMade';
import CallReceivedIcon from '@mui/icons-material/CallReceived';
import PersonIcon from '@mui/icons-material/Person';
import PersonOffIcon from '@mui/icons-material/PersonOff';
import { NymLogo } from '@nymproject/react/logo/NymLogo';
import { NymThemeProvider } from '@nymproject/mui-theme';
import { useTheme } from '@mui/material/styles';
import { useClipboard } from 'use-clipboard-copy';
import { ThemeToggle } from './ThemeToggle';
import { AppContextProvider, useAppContext } from './context';
import { MixnetContextProvider, useMixnetContext } from './context/mixnet';

export const AppTheme: React.FC = ({ children }) => {
  const { mode } = useAppContext();

  return <NymThemeProvider mode={mode}>{children}</NymThemeProvider>;
};

interface Log {
  kind: 'tx' | 'rx';
  message: string;
  timestamp: Date;
}

export const Content: React.FC = () => {
  const theme = useTheme();
  const { isReady, address, connect, events, sendTextMessage } = useMixnetContext();
  const copy = useClipboard();

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

  React.useEffect(() => {
    if (isReady) {
      // // mixnet v1
      // const validatorApiUrl = 'https://validator.nymtech.net/api';
      // const preferredGatewayIdentityKey = 'E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM';

      // mixnet v2
      const validatorApiUrl = 'https://qwerty-validator-api.qa.nymte.ch/api'; // "http://localhost:8081";
      const preferredGatewayIdentityKey = undefined; // '36vfvEyBzo5cWEFbnP7fqgY39kFw9PQhvwzbispeNaxL';

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
          message: e.args.message,
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
      console.error('No message set');
      return;
    }
    if (!recipient) {
      console.error('No recipient set');
      return;
    }

    log.current.push({
      kind: 'tx',
      timestamp: new Date(),
      message,
    });
    setLogTrigger(Date.now());
    await sendTextMessage({ message, recipient });
  };

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
              {address && (
                <Tooltip arrow title="Copy your client address to the clipboard">
                  <Chip
                    clickable
                    label={`${address.slice(0, 24)}...`}
                    onClick={() => {
                      if (address) {
                        copy.copy(address);
                      }
                    }}
                    icon={<ContentCopyIcon />}
                  />
                </Tooltip>
              )}
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
            <Button variant="contained" sx={{ width: 100 }} onClick={handleSend}>
              Send
            </Button>
          </Stack>
        )}
      </Box>
      {log.current.map((item) => (
        <Box key={item.kind + item.timestamp.toISOString()}>
          <Stack
            direction="row"
            spacing={2}
            alignItems="start"
            sx={{ color: item.kind === 'tx' ? theme.palette.success.main : theme.palette.info.main }}
          >
            {item.kind === 'tx' ? <CallMadeIcon /> : <CallReceivedIcon />}
            <Chip variant="outlined" label={item.timestamp.toLocaleTimeString()} />
            <Typography>{item.message}</Typography>
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
