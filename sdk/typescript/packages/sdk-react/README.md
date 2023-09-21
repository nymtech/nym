# Nym SDK React

This package provides useful React components to interact with the Nym Mixnet:

- a provider and hook to send messages over the Mixnet

## Install


Install this package in your project:

```
npm install @nymproject/sdk-react
```

Install the SDK:

```
npm install @nymproject/sdk
```

If you need to use one of the other variants you will need to use the following for NPM in `package.json` (remember to set the exact version):

```json
{
   "overrides": {
     "@nymproject/sdk": {
        "@nymproject/sdk-full-fat": "1"
     }
   }
 }
```

For `yarn` the syntax is slightly different (again in package.json):

```json
{
    "resolutions": {
      "@nymproject/sdk-full-fat": "1"
    }
}
```

## Usage

Add the provider near the top of your React app:

```jsx
import { MixnetContextProvider } from '@nymproject/sdk-react';

export const App = () => (
    <MixnetContextProvider>
        <Content />
    </MixnetContextProvider>
);
```

And then use the hook in your components:

```tsx
import { useMixnetContext } from '@nymproject/sdk-react';
import type { EventHandlerFn, StringMessageReceivedEvent } from '@nymproject/sdk';

export const SomeComponent = () => {
    const { isReady, address, sendTextMessage, events } = useMixnetContext();

    const handleMessage = React.useCallback<EventHandlerFn<StringMessageReceivedEvent>>((e) => {
        console.log('Received message: ', e.args.payload);
    }, []);
    
    // when the context is ready, register some events
    React.useEffect(() => {
        if (!events || !isReady) {
            return undefined;
        }

        // subscribe and get the unsubscribe function 
        const unsubscribeFn = events.subscribeToTextMessageReceivedEvent(handleMessage);

        // when unmounting unsubscribe will be called
        return unsubscribeFn;
    }, [isReady]);
    
    // send a message to yourself
    const handleSendToSelf = async (message: string) => {
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
      <button onClick={() => handleSendToSelf('This is a test')}>Send a message</button>
    );
}
```

## Coming Soon

In future releases of this package you will be able to use:

- a provider and hook to manage Coconut credentials
- a provider and hook to sign and broadcast messages to send to the Nyx blockchain RPC nodes
