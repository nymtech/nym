import { resetReport } from './shared/report';

// Clears the previous visual report before a run so the uploaded artifact is current.
export default async function globalSetup(): Promise<void> {
  resetReport();
}
