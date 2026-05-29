// rollup-plugin-web-worker-loader exposes worker files as virtual modules
// prefixed with `web-worker:`. At runtime these resolve to a `Worker` constructor.
declare module 'web-worker:*' {
  const WorkerCtor: new () => Worker;
  export default WorkerCtor;
}
