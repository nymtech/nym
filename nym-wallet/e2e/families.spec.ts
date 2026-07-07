import { test, expect, Page } from '@playwright/test';
import { FAMILY_IDS, FAMILY_NODES, familyMockUrl, TID } from './shared/families';
import { shot } from './shared/report';

/**
 * Primary e2e of the owner + operator Node Families journeys (design D1), driven against
 * the mock-wired app shell (`main.mock.html`, design D2) on the dev server. Selectors are
 * shared with the optional WebdriverIO leg via `./shared/families` (parity requirement)
 * and target the ids that actually render (see the note in that file).
 *
 * Each step also captures a screenshot via `shot()` → assembled into a static visual flow
 * report (`e2e-report/index.html`) for smoke inspection, uploaded by the CI build job.
 *
 * Confirmation dialogs portal to the document body; in a single browser DOM they're reached
 * at page scope. Per-node invite content is scoped via the `operator-node-<n>` wrapper, and
 * invite buttons are keyed by family id.
 */

const { ownerFlow, operatorAccept, operatorReject, operatorNone } = FAMILY_NODES;

const openOperatorTab = (page: Page) => page.getByTestId(TID.tabOperator).click();

test.describe('Families flows (mock-wired app shell)', () => {
  test('owner lifecycle: create → invite → accept → kick → disband', async ({ page }, testInfo) => {
    const fid = FAMILY_IDS.ownerFlow;
    await page.goto(familyMockUrl('owner'));

    // create
    await expect(page.getByTestId(TID.createFamilyName)).toBeVisible({ timeout: 30_000 });
    await shot(page, testInfo, 'create family entry');
    await page.getByTestId(TID.createFamilyName).fill('Flow Family');
    await page.getByTestId(TID.createFamilyDescription).fill('A family created in a flow test.');
    await page.getByTestId(TID.createFamilySubmit).click();
    await expect(page.getByTestId(TID.ownerManagementPage)).toBeVisible();
    await shot(page, testInfo, 'family created');

    // invite the self-controlled node
    await page.getByTestId(TID.inviteNodeId).fill(String(ownerFlow));
    await page.getByTestId(TID.inviteNodeSubmit).click();
    await page.getByTestId(TID.inviteNodeConfirm).click();
    await expect(page.getByTestId(TID.pendingInvite(ownerFlow))).toBeVisible();
    await shot(page, testInfo, 'node invited (pending)');

    // accept it from the operator tab (same account controls the node)
    await openOperatorTab(page);
    await page.getByTestId(TID.operatorNodeSection(ownerFlow)).getByTestId(TID.acceptCard(fid)).click();
    await page.getByTestId(TID.acceptConfirm(fid)).click();

    // kick it from the owner tab — the joined member appears, then is removed
    await page.getByTestId(TID.tabOwner).click();
    await expect(page.getByTestId(TID.memberJoined(ownerFlow))).toBeVisible();
    await shot(page, testInfo, 'member joined');
    await page.getByTestId(TID.memberJoinedKick(ownerFlow)).click();
    await page.getByTestId(TID.memberJoinedKickConfirm(ownerFlow)).click();
    await expect(page.getByTestId(TID.memberJoined(ownerFlow))).toHaveCount(0);
    await shot(page, testInfo, 'member kicked');

    // disband the now-empty family via settings
    await page.getByTestId(TID.familySettingsButton).click();
    await expect(page.getByTestId(TID.familySettingsPage)).toBeVisible();
    await page.getByTestId(TID.deleteButton).click();
    await page.getByTestId(TID.deleteConfirm).click();
    await expect(page.getByTestId(TID.createFamilyName)).toBeVisible();
    await shot(page, testInfo, 'family disbanded');
  });

  test('operator lifecycle: accept → leave, then reject', async ({ page }, testInfo) => {
    const fid = FAMILY_IDS.operatorFlow;
    await page.goto(familyMockUrl('operator'));
    await openOperatorTab(page);
    await shot(page, testInfo, 'pending node invites');

    // accept the invite on the accept-node (scope by node; invite buttons are keyed by family id)
    const acceptSection = page.getByTestId(TID.operatorNodeSection(operatorAccept));
    await acceptSection.getByTestId(TID.acceptCard(fid)).click();
    await page.getByTestId(TID.acceptConfirm(fid)).click();
    // joined → Leave appears on the My family tab
    await page.getByTestId(TID.tabOwner).click();
    await expect(page.getByTestId(TID.myNodeFamily(operatorAccept))).toBeVisible();
    await shot(page, testInfo, 'invite accepted');

    // leave the family
    await page.getByTestId(TID.leaveButton).click();
    await page.getByTestId(TID.leaveConfirm).click();
    await shot(page, testInfo, 'family left');

    // reject the invite on the reject-node → its group ends empty
    await openOperatorTab(page);
    await page.getByTestId(TID.operatorNodeSection(operatorReject)).getByTestId(TID.rejectCard(fid)).click();
    await page.getByTestId(TID.rejectConfirm(fid)).click();
    await expect(page.getByTestId(TID.inviteGroupEmpty(operatorReject))).toBeVisible();
    await shot(page, testInfo, 'invite rejected');
  });

  test('operator page shows multi-node invite states', async ({ page }, testInfo) => {
    await page.goto(familyMockUrl('operator-seeded'));
    await openOperatorTab(page);
    // node with an active invite renders its section; node with none shows the empty state
    await expect(page.getByTestId(TID.operatorNodeSection(operatorAccept))).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId(TID.inviteGroupEmpty(operatorNone))).toBeVisible();
    await shot(page, testInfo, 'multi-node invite states');
  });
});
