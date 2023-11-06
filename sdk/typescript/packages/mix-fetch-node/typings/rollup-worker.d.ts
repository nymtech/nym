declare module 'web-worker:*' {
  import { Worker } from 'node:worker_threads';

  const WorkerFactory: new () => Worker;
  export default WorkerFactory;
}
