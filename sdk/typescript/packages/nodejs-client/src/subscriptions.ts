import type { EventHandlerFn } from './types';
import { EventKinds } from './types';

type ISubscriptions = {
  [key: string]: Array<EventHandlerFn<unknown>>;
};

/**
 * Creates a subscription manager.
 */
export const createSubscriptions = () => {
  // stores the subscriptions for events
  const subscriptions: ISubscriptions = {};

  /**
   * Helper method to get typed subscriptions.
   */
  const getSubscriptions = <E>(key: EventKinds): Array<EventHandlerFn<E>> => {
    if (!subscriptions[key]) {
      subscriptions[key] = [];
    }
    return subscriptions[key] as Array<EventHandlerFn<E>>;
  };

  /**
   * Remove a subscription.
   */
  const removeSubscription = <E>(key: EventKinds, handler: EventHandlerFn<E>) => {
    if (!subscriptions[key]) {
      subscriptions[key] = [];
    }
    const items: Array<EventHandlerFn<unknown>> = (subscriptions[key] as Array<EventHandlerFn<unknown>>).filter(
      (h) => h !== handler,
    );
    subscriptions[key] = items;
  };

  /**
   * Add typed subscription.
   */
  const addSubscription = <E>(key: EventKinds, handler: EventHandlerFn<E>) => {
    getSubscriptions(key).push(handler as EventHandlerFn<unknown>);

    return () => {
      removeSubscription(key, handler);
    };
  };

  /**
   * Fires an event.
   */
  const fireEvent = <E>(key: EventKinds, event: E) => {
    getSubscriptions(key).forEach((handler) => {
      try {
        handler(event);
      } catch (e: any) {
        // eslint-disable-next-line no-console
        console.error(`Unhandled exception in handler for ${key}: `, e);
      }
    });
  };

  return {
    getSubscriptions,
    addSubscription,
    removeSubscription,
    fireEvent,
    subscriptions,
  };
};
