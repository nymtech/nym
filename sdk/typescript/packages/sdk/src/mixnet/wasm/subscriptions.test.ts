import { createSubscriptions } from './subscriptions';
import { EventKinds, MimeTypes, StringMessageReceivedEvent } from './types';

describe('wasm subscription manager', () => {
  test('works with default values', () => {
    const { getSubscriptions, fireEvent, addSubscription } = createSubscriptions();

    expect(getSubscriptions(EventKinds.StringMessageReceived)).toHaveLength(0);

    // the event should fire and not fail
    fireEvent(EventKinds.StringMessageReceived, {});

    // mock a handler, fire events and check that it was called
    const mockHandler = jest.fn();
    addSubscription(EventKinds.StringMessageReceived, mockHandler);
    fireEvent(EventKinds.StringMessageReceived, {});
    expect(mockHandler).toHaveBeenCalled();
  });

  test('adding and removing subscriptions works as expected', () => {
    const { addSubscription, getSubscriptions, fireEvent } = createSubscriptions();

    expect(getSubscriptions(EventKinds.StringMessageReceived)).toHaveLength(0);

    const callStats: number[] = [0, 0, 0];

    const showDebug = false;

    const handler1 = (e: StringMessageReceivedEvent) => {
      if (showDebug) {
        console.log('handler1', e);
      }
      callStats[0] += 1;
    };
    const handler2 = (e: StringMessageReceivedEvent) => {
      if (showDebug) {
        console.log('handler2', e);
      }
      callStats[1] += 1;
    };
    const handler3 = (e: StringMessageReceivedEvent) => {
      if (showDebug) {
        console.log('handler3', e);
      }
      callStats[2] += 1;
    };

    const unsubcribeFn1 = addSubscription(EventKinds.StringMessageReceived, handler1);
    const unsubcribeFn2 = addSubscription(EventKinds.StringMessageReceived, handler2);
    const unsubcribeFn3 = addSubscription(EventKinds.StringMessageReceived, handler3);

    const event: StringMessageReceivedEvent = {
      kind: EventKinds.StringMessageReceived,
      args: {
        payload: 'Testing',
        mimeType: MimeTypes.TextPlain,
        payloadRaw: new Uint8Array(),
      },
    };

    // fire and expect all handlers to get message
    fireEvent(EventKinds.StringMessageReceived, event);
    expect(callStats[0]).toBe(1);
    expect(callStats[1]).toBe(1);
    expect(callStats[2]).toBe(1);
    expect(getSubscriptions(EventKinds.StringMessageReceived)).toHaveLength(3);

    // unscribe and fire again
    unsubcribeFn2();
    fireEvent(EventKinds.StringMessageReceived, event);
    expect(callStats[0]).toBe(2);
    expect(callStats[1]).toBe(1);
    expect(callStats[2]).toBe(2);
    expect(getSubscriptions(EventKinds.StringMessageReceived)).toHaveLength(2);

    // unscribe and fire again
    unsubcribeFn3();
    fireEvent(EventKinds.StringMessageReceived, event);
    expect(callStats[0]).toBe(3);
    expect(callStats[1]).toBe(1);
    expect(callStats[2]).toBe(2);
    expect(getSubscriptions(EventKinds.StringMessageReceived)).toHaveLength(1);

    // unscribe and fire again
    unsubcribeFn1();
    fireEvent(EventKinds.StringMessageReceived, event);
    expect(callStats[0]).toBe(3);
    expect(callStats[1]).toBe(1);
    expect(callStats[2]).toBe(2);
    expect(getSubscriptions(EventKinds.StringMessageReceived)).toHaveLength(0);

    // nothing is subscribed, so fire again and check
    fireEvent(EventKinds.StringMessageReceived, event);
    expect(callStats[0]).toBe(3);
    expect(callStats[1]).toBe(1);
    expect(callStats[2]).toBe(2);
  });
});
