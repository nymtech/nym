export const sleep = (delayMilliseconds: number) =>
  // eslint-disable-next-line no-promise-executor-return
  new Promise((resolve) => setTimeout(resolve, delayMilliseconds));
