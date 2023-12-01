declare module 'web-worker:*' {
  import { Worker } from 'worker_threads';

  const WorkerFactory: new () => Worker;
  export default WorkerFactory;
}
