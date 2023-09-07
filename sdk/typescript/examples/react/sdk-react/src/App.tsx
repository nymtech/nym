import * as React from 'react';
import { MixnetContextProvider, useMixnetContext } from '@nymproject/sdk-react';
import type { EventHandlerFn, StringMessageReceivedEvent } from '@nymproject/sdk';

export const Content: FCWithChildren = () => {
  const { isReady, address, sendTextMessage, events } = useMixnetContext();
  const log = React.useRef<React.ReactNode[]>([]);
  const [trigger, setTrigger] = React.useState(new Date());
  const formElementRef = React.createRef<HTMLInputElement>();

  const appendLog = (type: React.ReactNode, node?: React.ReactNode) => {
    const timestamp = new Date().toLocaleTimeString();
    log.current.push(
      <div key={timestamp + log.current.length.toString()}>
        <div>
          {timestamp} {type}
        </div>
        {node || null}
        <hr />
      </div>,
    );
    setTrigger(new Date());
  };

  const handleMessage = React.useCallback<EventHandlerFn<StringMessageReceivedEvent>>((e) => {
    appendLog(
      <code>⬅️ Received</code>,
      <div>
        <pre>{e.args.payload}</pre>
      </div>,
    );
  }, []);

  React.useEffect(() => {
    if (!events || !isReady) {
      return undefined;
    }

    appendLog(<strong>Connected</strong>);

    const unsubscribeFn = events.subscribeToTextMessageReceivedEvent(handleMessage);

    // when unmounting unsubscribe will be called
    return unsubscribeFn;
  }, [isReady]);

  const handleSend = async () => {
    if (!formElementRef.current || !address) {
      return;
    }

    const message = formElementRef.current.value;

    appendLog(
      <code>➡️️ Sent</code>,
      <div>
        <pre>{message}</pre>
      </div>,
    );
    setTrigger(new Date());

    await sendTextMessage({ payload: message, recipient: address });
  };

  if (!isReady) {
    return (
      <div>
        <h2>Nym SDK Mixnet React Context</h2>
        <div>Loading...</div>
      </div>
    );
  }

  return (
    <div>
      <h2>Nym SDK Mixnet React Context</h2>
      <div style={{ margin: '2rem 0', padding: '1rem', background: '#eee', borderRadius: '5px' }}>
        <p>
          Client address: <code>{address}</code>
        </p>
        <p>
          <input ref={formElementRef} id="message" type="text" defaultValue="Test message" />
          <button onClick={handleSend}>Send to self</button>
        </p>
      </div>
      <h2>Logs</h2>
      {log.current.map((item) => (
        <>{item} </>
      ))}
    </div>
  );
};
export const App: FCWithChildren = () => (
  <MixnetContextProvider>
    <Content />
  </MixnetContextProvider>
);
