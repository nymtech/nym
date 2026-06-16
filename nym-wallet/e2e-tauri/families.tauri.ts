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

describe('Families flows — native webview', () => {
  it('owner lifecycle: create → invite → accept → kick → disband', async () => {
    const fid = FAMILY_IDS.ownerFlow;

    await byId(TID.createFamilyName).waitForDisplayed({ timeout: 30_000 });
    await byId(TID.createFamilyName).setValue('Flow Family');
    await byId(TID.createFamilyDescription).setValue('A family created in a flow test.');
    await byId(TID.createFamilySubmit).click();
    await byId(TID.ownerManagementPage).waitForDisplayed();

    await byId(TID.inviteNodeId).setValue(String(ownerFlow));
    await byId(TID.inviteNodeSubmit).click();
    await byId(TID.inviteNodeConfirm).click();
    await byId(TID.pendingInvite(ownerFlow)).waitForDisplayed();

    await byId(TID.tabOperator).click();
    await inSection(ownerFlow, TID.acceptCard(fid)).click();
    await byId(TID.acceptConfirm(fid)).click();

    await byId(TID.tabOwner).click();
    await byId(TID.memberJoined(ownerFlow)).waitForDisplayed();
    await byId(TID.memberJoinedKick(ownerFlow)).click();
    await byId(TID.memberJoinedKickConfirm(ownerFlow)).click();
    await byId(TID.memberJoined(ownerFlow)).waitForExist({ reverse: true });

    await byId(TID.deleteButton).click();
    await byId(TID.deleteConfirm).click();
    await byId(TID.createFamilyName).waitForDisplayed();
  });

  it('operator lifecycle: accept → leave, then reject', async () => {
    const fid = FAMILY_IDS.operatorFlow;
    await browser.url(appUrl('operator'));

    await byId(TID.tabOperator).click();
    await inSection(operatorAccept, TID.acceptCard(fid)).click();
    await byId(TID.acceptConfirm(fid)).click();
    await inSection(operatorAccept, TID.leaveButton).waitForDisplayed();

    await inSection(operatorAccept, TID.leaveButton).click();
    await byId(TID.leaveConfirm).click();

    await inSection(operatorReject, TID.rejectCard(fid)).click();
    await byId(TID.rejectConfirm(fid)).click();
    await byId(TID.inviteGroupEmpty(operatorReject)).waitForDisplayed();
  });
});
