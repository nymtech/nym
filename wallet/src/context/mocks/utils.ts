export const mockSleep = (delayMilliseconds: number) =>
  new Promise((resolve) => setTimeout(resolve, delayMilliseconds));
