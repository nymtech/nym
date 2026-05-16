import { isMixFetchDebugEnabled, runIfDebugEnabled } from './debug';

describe('runIfDebugEnabled', () => {
  let originalDebugFlag: boolean | undefined;

  beforeEach(() => {
    originalDebugFlag = globalThis.__MIX_FETCH_DEBUG__;
  });

  afterEach(() => {
    if (originalDebugFlag === undefined) {
      delete globalThis.__MIX_FETCH_DEBUG__;
    } else {
      globalThis.__MIX_FETCH_DEBUG__ = originalDebugFlag;
    }
  });

  test('is disabled by default', () => {
    delete globalThis.__MIX_FETCH_DEBUG__;

    expect(isMixFetchDebugEnabled()).toBe(false);
  });

  test('does not run the callback when the debug flag is disabled', () => {
    globalThis.__MIX_FETCH_DEBUG__ = false;
    const callback = jest.fn();

    runIfDebugEnabled(callback);

    expect(callback).not.toHaveBeenCalled();
  });

  test('runs the callback when the debug flag is enabled', () => {
    globalThis.__MIX_FETCH_DEBUG__ = true;
    const callback = jest.fn();

    runIfDebugEnabled(callback);

    expect(callback).toHaveBeenCalledTimes(1);
  });
});
