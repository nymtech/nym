// `web-worker:` virtual modules emitted by `rollup-plugin-web-worker-loader`.
// Script-style ambient declaration (no imports/exports in this file) so the
// wildcard pattern works, since module augmentations don't support wildcards.
declare module 'web-worker:*' {
  const WorkerCtor: new () => Worker;
  export default WorkerCtor;
}
