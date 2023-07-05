import * as React from 'react';
import { createNymMixnetClient, IWebWorkerEvents, MimeTypes, NymClientConfig, NymMixnetClient } from '@nymproject/sdk';

export interface BinaryMessageHeaders {
  filename: string;
  mimeType: string;
}

export const parseBinaryMessageHeaders = (headers: string): BinaryMessageHeaders =>
  JSON.parse(headers) as BinaryMessageHeaders;

interface State {
  // data
  isReady: boolean;
  address?: string;
  events?: IWebWorkerEvents;

  // methods
  connect: (config: NymClientConfig) => Promise<void>;
  sendTextMessage: (args: { payload: string; recipient: string }) => Promise<void>;
  sendBinaryMessage: (args: { payload: Uint8Array; recipient: string; headers: BinaryMessageHeaders }) => Promise<void>;
}

const MixnetContext = React.createContext<State | undefined>(undefined);

export const useMixnetContext = (): State => {
  const context = React.useContext<State | undefined>(MixnetContext);

  if (!context) {
    throw new Error('Please include a `import { MixnetContextProvider } from "./context"` before using this hook');
  }

  return context;
};

export const MixnetContextProvider: FCWithChildren = ({ children }) => {
  const [isReady, setReady] = React.useState<boolean>(false);
  const [address, setAddress] = React.useState<string>();

  const nym = React.useRef<NymMixnetClient | null>(null);

  React.useEffect(() => {
    // on mount of the provider, create the client
    (async () => {
      nym.current = await createNymMixnetClient();
      if (nym.current?.events) {
        nym.current.events.subscribeToConnected((e) => {
          setAddress(e.args.address);
        });
      }
      setReady(true);
    })();

    //
  }, []);

  const connect = async (config: NymClientConfig) => {
    if (!nym.current?.client) {
      console.error('Nym client has not initialised. Please wrap in useEffect on `isReady` prop of this context.');
      return;
    }
    await nym.current.client.start(config);
  };

  const sendTextMessage = async (args: { payload: string; recipient: string }) => {
    if (!nym.current?.client) {
      console.error('Nym client has not initialised. Please wrap in useEffect on `isReady` prop of this context.');
      return;
    }
    await nym.current.client.send({
      recipient: args.recipient,
      payload: { message: args.payload, mimeType: MimeTypes.TextPlain },
    });
  };

  const sendBinaryMessage = async (args: { payload: Uint8Array; recipient: string; headers: BinaryMessageHeaders }) => {
    if (!nym.current?.client) {
      console.error('Nym client has not initialised. Please wrap in useEffect on `isReady` prop of this context.');
      return;
    }
    // convert headers to JSON
    await nym.current.client.send({
      recipient: args.recipient,
      payload: { message: args.payload, mimeType: 'application/octet-stream', headers: JSON.stringify(args.headers) },
    });
  };

  const value = React.useMemo<State>(
    () => ({
      isReady,
      events: nym.current?.events,
      address,
      connect,
      sendTextMessage,
      sendBinaryMessage,
    }),
    [isReady, nym.current, address],
  );

  return <MixnetContext.Provider value={value}>{children}</MixnetContext.Provider>;
};
