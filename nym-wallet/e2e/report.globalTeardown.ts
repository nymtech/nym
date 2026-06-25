import { buildGallery } from './shared/report';

// Stitches the per-step screenshots into e2e-report/index.html after the run.
export default async function globalTeardown(): Promise<void> {
  buildGallery();
}
