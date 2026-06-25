import { FAMILY_IDS, FAMILY_NODES, TID } from '../e2e/shared/families';

/**
 * Native-webview replay of the owner + operator journeys (design D4), sharing selectors +
 * fixtures with the primary Playwright suite via `e2e/shared/families` (parity requirement).
 *
 * The mock binary boots into `main.mock.html` (owner persona) — so the owner journey runs on
 * launch with no navigation. The operator journey navigates the webview to the operator persona
 * first. On Linux/WebKitGTK the Tauri asset scheme is `tauri://localhost/` (the mock config sets
 * `useHttpsScheme: false`); adjust `appUrl` if a platform serves a different scheme.
 */

const appUrl = (persona: 'owner' | 'operator' | 'operator-seeded') =>
  `tauri://localhost/main.mock.html?persona=${persona}`;

const byId = (id: string) => $(`[data-testid="${id}"]`);
const inSection = (node: number, id: string) => byId(TID.operatorNodeSection(node)).$(`[data-testid="${id}"]`);

const { ownerFlow, operatorAccept, operatorReject } = FAMILY_NODES;

/** WebKitGTK webdriver often reports "click intercepted" without scroll / clickable waits. */
const clickTestId = async (id: string) => {
  const el = byId(id);
  await el.waitForDisplayed({ timeout: 30_000 });
  await el.scrollIntoView();
  await el.waitForClickable({ timeout: 15_000 });
  await el.click();
};

const clickInSection = async (node: number, id: string) => {
  const el = inSection(node, id);
  await el.waitForDisplayed({ timeout: 30_000 });
  await el.scrollIntoView();
  await el.waitForClickable({ timeout: 15_000 });
  await el.click();
};

/** Confirm modals portal to `document.body`; wait for the confirm control to unmount before the next click. */
const confirmAction = async (confirmTestId: string) => {
  await clickTestId(confirmTestId);
  await byId(confirmTestId).waitForExist({ reverse: true, timeout: 15_000 });
};

describe('Families flows — native webview', () => {
  it('owner lifecycle: create → invite → accept → kick → disband', async () => {
    const fid = FAMILY_IDS.ownerFlow;

    await byId(TID.createFamilyName).waitForDisplayed({ timeout: 30_000 });
    await byId(TID.createFamilyName).setValue('Flow Family');
    await byId(TID.createFamilyDescription).setValue('A family created in a flow test.');
    await clickTestId(TID.createFamilySubmit);
    await byId(TID.ownerManagementPage).waitForDisplayed();

    await byId(TID.inviteNodeId).setValue(String(ownerFlow));
    await clickTestId(TID.inviteNodeSubmit);
    await confirmAction(TID.inviteNodeConfirm);
    await byId(TID.pendingInvite(ownerFlow)).waitForDisplayed();

    await clickTestId(TID.tabOperator);
    await clickInSection(ownerFlow, TID.acceptCard(fid));
    await confirmAction(TID.acceptConfirm(fid));

    await clickTestId(TID.tabOwner);
    await byId(TID.memberJoined(ownerFlow)).waitForDisplayed({ timeout: 15_000 });
    await clickTestId(TID.memberJoinedKick(ownerFlow));
    await confirmAction(TID.memberJoinedKickConfirm(ownerFlow));
    await byId(TID.memberJoined(ownerFlow)).waitForExist({ reverse: true });

    // Match Playwright: dissolve via Family Settings (delete is not on the management page).
    await clickTestId(TID.familySettingsButton);
    await byId(TID.familySettingsPage).waitForDisplayed();
    await clickTestId(TID.deleteButton);
    await confirmAction(TID.deleteConfirm);
    await byId(TID.createFamilyName).waitForDisplayed();
  });

  it('operator lifecycle: accept → leave, then reject', async () => {
    const fid = FAMILY_IDS.operatorFlow;
    await browser.url(appUrl('operator'));

    await clickTestId(TID.tabOperator);
    await clickInSection(operatorAccept, TID.acceptCard(fid));
    await confirmAction(TID.acceptConfirm(fid));

    // Leave lives on the My family tab (MyNodeFamilySection), not inside the operator invite group.
    await clickTestId(TID.tabOwner);
    await byId(TID.myNodeFamily(operatorAccept)).waitForDisplayed({ timeout: 15_000 });
    await clickTestId(TID.leaveButton);
    await confirmAction(TID.leaveConfirm);

    await clickTestId(TID.tabOperator);
    await clickInSection(operatorReject, TID.rejectCard(fid));
    await confirmAction(TID.rejectConfirm(fid));
    await byId(TID.inviteGroupEmpty(operatorReject)).waitForDisplayed();
  });
});
