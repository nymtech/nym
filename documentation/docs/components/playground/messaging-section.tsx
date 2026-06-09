import React, { useEffect, useRef, useState } from 'react';
import { box, row, legend, sub, input, Button, LogPanel, StatusText, useLogs, type Status } from './ui';

// Raw mixnet messaging demo, styled to match the mix-* playground sections.
// Uses @nymproject/sdk-full-fat (a separate wasm client from the smolmix tunnel)
// to create a Nym client, send a message to a Nym address, and receive it.
//
// The SDK is imported dynamically on Connect, not at module scope: that keeps
// the page SSR/SSG-safe and means the second wasm runtime + gateway connection
// only load when the visitor opts in.

const nymApiUrl = 'https://validator.nymtech.net/api';
const preferredGateway = 'q2A2cbooyC16YJzvdYaSMH9X3cSiieZNtfBr8cE8Fi1';

// Minimal shape of the bits of NymMixnetClient we use, to type the `any` from
// the dynamic import.
interface MessagingClient {
  client: {
    start(opts: {
      clientId: string;
      nymApiUrl: string;
      forceTls?: boolean;
      preferredGateway?: string;
    }): Promise<void>;
    stop(): Promise<void>;
    send(args: { payload: { message: string; mimeType: string }; recipient: string }): Promise<void>;
  };
  events: {
    subscribeToLoaded(cb: (e: { args: unknown }) => void): void;
    subscribeToConnected(cb: (e: { args: { address: string } }) => void): void;
    subscribeToTextMessageReceivedEvent(cb: (e: { args: { payload: string } }) => void): void;
  };
}

export function MessagingDemo() {
  const { log, lines } = useLogs();
  const [status, setStatus] = useState<Status>({ text: 'Not connected', colour: 'gray' });
  const [connected, setConnected] = useState(false);
  const [busy, setBusy] = useState(false);
  const [selfAddress, setSelfAddress] = useState('');
  const [recipient, setRecipient] = useState('');
  const [message, setMessage] = useState('hello through the mixnet');
  const clientRef = useRef<MessagingClient | null>(null);

  useEffect(() => {
    return () => {
      clientRef.current?.client.stop().catch(() => {});
    };
  }, []);

  async function connect() {
    setBusy(true);
    setStatus({ text: 'Loading SDK...', colour: 'orange' });
    log('msg', 'Loading @nymproject/sdk-full-fat (wasm)...');
    try {
      // @ts-ignore -- published separately; dynamic import keeps the wasm off SSR and lazy
      const mod = await import('@nymproject/sdk-full-fat');
      const nym = (await mod.createNymMixnetClient()) as unknown as MessagingClient;
      clientRef.current = nym;

      nym.events.subscribeToLoaded(() => log('msg', 'client wasm loaded', 'green'));
      nym.events.subscribeToConnected((e) => {
        const addr = e.args.address;
        setSelfAddress(addr);
        setRecipient((r) => r || addr); // default to sending to yourself
        setConnected(true);
        setStatus({ text: 'Connected', colour: 'green' });
        log('msg', `connected; self address: ${addr}`, 'green');
      });
      nym.events.subscribeToTextMessageReceivedEvent((e) => {
        log('msg', `received: ${e.args.payload}`, 'green');
      });

      log('msg', 'Starting client and connecting to a gateway...');
      setStatus({ text: 'Connecting to mixnet...', colour: 'orange' });
      await nym.client.start({ clientId: crypto.randomUUID(), nymApiUrl, forceTls: true, preferredGateway });
    } catch (err) {
      setStatus({ text: 'Failed', colour: 'red' });
      log('msg', `error: ${err instanceof Error ? err.message : String(err)}`, 'red');
      setBusy(false);
    }
    setBusy(false);
  }

  async function send() {
    const nym = clientRef.current;
    if (!nym || !recipient || !message) return;
    try {
      await nym.client.send({ payload: { message, mimeType: 'text/plain' }, recipient });
      log('msg', `sent: ${message}`);
    } catch (err) {
      log('msg', `send failed: ${err instanceof Error ? err.message : String(err)}`, 'red');
    }
  }

  return (
    <div style={box}>
      <div style={legend}>Raw mixnet messaging</div>
      <div style={sub}>
        Creates a client with <code>@nymproject/sdk-full-fat</code> and sends a message to a Nym
        address through the mixnet (defaults to your own address). This loads a separate wasm runtime
        and connects its own client, so it is opt-in.
      </div>
      <div style={{ ...row, marginTop: '0.75rem' }}>
        <Button onClick={connect} disabled={busy || connected}>
          {busy ? 'Connecting...' : 'Connect'}
        </Button>
        <StatusText status={status} />
      </div>
      {selfAddress && (
        <div style={{ ...sub, wordBreak: 'break-all', margin: '0 0 0.5rem' }}>
          Your address: <code>{selfAddress}</code>
        </div>
      )}
      <div style={row}>
        <input
          style={input}
          value={recipient}
          onChange={(e) => setRecipient(e.target.value)}
          placeholder="recipient Nym address"
          disabled={!connected}
        />
      </div>
      <div style={row}>
        <input
          style={input}
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          placeholder="message"
          disabled={!connected}
        />
        <Button onClick={send} disabled={!connected}>
          Send
        </Button>
      </div>
      <LogPanel lines={lines('msg')} placeholder="Press Connect to create a mixnet client." />
    </div>
  );
}
