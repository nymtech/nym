import figma from '@figma/code-connect';
import { FamilyPage } from './FamilyPage';

// Wire 2: Code Connect mapping for FamilyPage (NYM-1199).
// Maps to the Node Families composite frame in Nym 2.0 (2026-05-13).
// FamilyPage takes no props; it reads all state from FamiliesContext.
//
// Node 1861:1889 is the Family-scoped composite. No individual 420px screen
// frame exists in the file yet; update node-id when Yana cuts a production frame.
//
// Publish (Tier-1 gate, needs Hux approval before running):
//   FIGMA_ACCESS_TOKEN=<pat-with-code-connect-write> npx figma connect publish

figma.connect(FamilyPage, 'https://www.figma.com/design/moIK1E6AaXhFz8lI1pZVrI/Nym.2.0?node-id=1861-1889', {
  example: () => <FamilyPage />,
});
