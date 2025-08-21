import React, { useEffect, useState, useRef } from "react";
import init, {
  NymClient,
  encode_payload,
  decode_payload,
  defaultDebug,
  ClientConfig,
  no_cover_debug,
} from "@nymproject/sdk-full-fat";
import Box from "@mui/material/Box";
import CircularProgress from "@mui/material/CircularProgress";
import Paper from "@mui/material/Paper";
import Typography from "@mui/material/Typography";
import Stack from "@mui/material/Stack";
import TextField from "@mui/material/TextField";
import Button from "@mui/material/Button";

const nymApiUrl = "https://validator.nymtech.net/api";

// Initialize WASM module once at module level
let wasmInitialized = false;
const initWasm = async () => {
  if (!wasmInitialized) {
    await init();
    wasmInitialized = true;
    console.log("WASM module initialized");
  }
};

export const Traffic = () => {
  const [nymClient, setNymClient] = useState<any>(null);
  const [selfAddress, setSelfAddress] = useState<string>("");
  const [recipient, setRecipient] = useState<string>("");
  const [messageText, setMessageText] = useState<string>("");
  const [receivedMessage, setReceivedMessage] = useState<string>("");
  const [buttonEnabled, setButtonEnabled] = useState<boolean>(false);
  const [isConnecting, setIsConnecting] = useState<boolean>(true);
  const [error, setError] = useState<string>("");
  const clientRef = useRef<any>(null);

  const init = async () => {
    try {
      console.log("Starting Nym client initialization...");

      await initWasm();

      // // Use no_cover_debug to potentially avoid some complexity
      // const debugConfig = no_cover_debug();
      // console.log("Using no_cover_debug config:", debugConfig);

      // Create a handler that logs but never throws
      const safeHandler = (data: any) => {
        console.log("Handler received data type:", typeof data);
        if (data instanceof Uint8Array) {
          console.log("Received Uint8Array of length:", data.length);
        }
      };

      // Create config first
      const config = new ClientConfig({
        id: crypto.randomUUID(),
        nymApi: nymApiUrl,
        // debug: debugConfig, // Use no cover traffic config
      });

      // Use newWithConfig static method
      const client = await NymClient.newWithConfig(config, safeHandler, {
        forceTls: false, //true,
      });

      console.log("Client created via newWithConfig");

      if (client && typeof client.selfAddress === "function") {
        const address = client.selfAddress();
        setSelfAddress(address);
        console.log("Got address:", address);
      }

      clientRef.current = client;
      setNymClient(client);
      setIsConnecting(false);
    } catch (error: any) {
      console.error("Init failed:", error);
      setError(`Failed: ${error.message}`);
      setIsConnecting(false);
    }
  };

  const stop = async () => {
    if (clientRef.current) {
      try {
        // Check if client has a stop or cleanup method
        if (typeof clientRef.current.free === "function") {
          clientRef.current.free();
        }
        clientRef.current = null;
        setNymClient(null);
        console.log("Nym client cleaned up");
      } catch (error) {
        console.error("Error cleaning up client:", error);
      }
    }
  };

  const send = async () => {
    if (!nymClient || !recipient || !messageText) {
      return;
    }

    try {
      // Encode the message
      const messageBytes = new TextEncoder().encode(messageText);
      const encodedPayload = encode_payload("text/plain", messageBytes);

      // Send using the snake_case method
      if (typeof nymClient.send_regular_message === "function") {
        await nymClient.send_regular_message(encodedPayload, recipient);
        console.log("Message sent to:", recipient);
      } else if (typeof nymClient.send === "function") {
        // Fallback to simpler send method if it exists
        await nymClient.send(encodedPayload, recipient);
        console.log("Message sent (fallback method) to:", recipient);
      } else {
        console.error("No send method found on client");
        setError("Unable to send message - client missing send method");
      }
    } catch (error: any) {
      console.error("Failed to send message:", error);
      setError(`Send failed: ${error.message}`);
    }
  };

  useEffect(() => {
    init();
    return () => {
      stop();
    };
  }, []);

  useEffect(() => {
    setButtonEnabled(!!recipient && !!messageText);
  }, [recipient, messageText]);

  if (error) {
    return (
      <Box sx={{ display: "flex", justifyContent: "center", padding: 3 }}>
        <Paper sx={{ padding: 3, backgroundColor: "#ffebee" }}>
          <Typography color="error" variant="h6">
            Error
          </Typography>
          <Typography>{error}</Typography>
          <Button
            variant="outlined"
            onClick={() => window.location.reload()}
            sx={{ mt: 2 }}
          >
            Reload Page
          </Button>
        </Paper>
      </Box>
    );
  }

  if (isConnecting || !selfAddress) {
    return (
      <Box sx={{ display: "flex", justifyContent: "center", padding: 3 }}>
        <Stack alignItems="center" spacing={2}>
          <CircularProgress />
          <Typography>Connecting to Nym network...</Typography>
        </Stack>
      </Box>
    );
  }

  return (
    <Box padding={3}>
      <Paper style={{ marginTop: "1rem", padding: "2rem" }}>
        <Stack spacing={3}>
          <Typography variant="body1">My self address is:</Typography>
          <Typography
            variant="body1"
            sx={{ fontFamily: "monospace", wordBreak: "break-all" }}
          >
            {selfAddress || "loading"}
          </Typography>
          <Typography variant="h5">Communication through the Mixnet</Typography>
          <TextField
            type="text"
            placeholder="Recipient Address"
            value={recipient}
            onChange={(e) => setRecipient(e.target.value)}
            size="small"
            fullWidth
          />
          <TextField
            type="text"
            placeholder="Message to send"
            multiline
            rows={4}
            value={messageText}
            onChange={(e) => setMessageText(e.target.value)}
            size="small"
            fullWidth
          />
          <Button
            variant="outlined"
            onClick={() => send()}
            disabled={!buttonEnabled}
            sx={{ width: "fit-content" }}
          >
            Send
          </Button>
        </Stack>
        {receivedMessage && (
          <Stack spacing={3} style={{ marginTop: "2rem" }}>
            <Typography variant="h5">Message Received!</Typography>
            <Typography fontFamily="monospace" sx={{ wordBreak: "break-all" }}>
              {receivedMessage}
            </Typography>
          </Stack>
        )}
      </Paper>
    </Box>
  );
};
