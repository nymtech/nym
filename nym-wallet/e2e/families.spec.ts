import { test, expect } from '@playwright/test';

/**
 * e2e coverage of the owner + operator flows (tasks.md §8.4), driven against the
 * Storybook flow stories. Each flow story's `play` function runs automatically when
 * the story iframe loads, so we navigate to the story and assert the post-flow DOM.
 */

const storyUrl = (id: string) => `/iframe.html?id=${id}&viewMode=story`;

test.describe('Families flows', () => {
  test('owner lifecycle: create → invite → accept → kick → disband', async ({ page }) => {
    await page.goto(storyUrl('families-flows--owner-lifecycle'));
    // After disband the family is gone, so the create entry point returns.
    await expect(page.getByTestId('create-family-name')).toBeVisible({ timeout: 30_000 });
  });

  test('operator lifecycle: accept → leave, then reject', async ({ page }) => {
    await page.goto(storyUrl('families-flows--operator-lifecycle'));
    // After rejecting the last invite, the reject-node group is empty.
    await expect(page.getByTestId('node-invite-group-204-empty')).toBeVisible({ timeout: 30_000 });
  });

  test('operator page shows multi-node invite states', async ({ page }) => {
    await page.goto(storyUrl('families-pages-operatorinvitespage--multi-node'));
    await expect(page.getByTestId('node-invite-group-201')).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId('node-invite-group-203-empty')).toBeVisible();
  });
});
