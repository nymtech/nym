import type { Meta, StoryObj } from '@storybook/react-webpack5';
import { within, screen, userEvent, waitFor, expect } from 'storybook/test';
import { withFamiliesMock } from 'src/components/Families/withFamiliesMock';
import {
  buildOperatorFlowStore,
  buildOwnerFlowStore,
  MOCK_OPERATOR_ADDRESS,
  MOCK_OPERATOR_FLOW_ACCEPT_NODE,
  MOCK_OPERATOR_FLOW_REJECT_NODE,
  MOCK_OWNER_ADDRESS,
  MOCK_OWNER_FLOW_NODE,
} from 'src/context/mocks/families.fixtures';
import { FamilyPage } from './FamilyPage';

/**
 * End-to-end flow stories driven by play functions against the mock contract.
 * Confirmation dialogs portal to document.body, so confirm buttons are queried via
 * `screen` while in-canvas elements use `within(canvasElement)`.
 * (These are exercised by the Storybook interaction + Playwright runs in §8/§9.)
 */
const meta: Meta<typeof FamilyPage> = {
  title: 'Families/Flows',
  component: FamilyPage,
};
export default meta;

type Story = StoryObj<typeof FamilyPage>;

const NODE = MOCK_OWNER_FLOW_NODE;

/** Owner lifecycle: create → invite → accept → kick → disband (single self-controlled account). */
export const OwnerLifecycle: Story = {
  decorators: [withFamiliesMock({ sender: MOCK_OWNER_ADDRESS, makeStore: buildOwnerFlowStore, latencyMs: 0 })],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    // create
    await userEvent.type(await canvas.findByTestId('create-family-name'), 'Flow Family');
    await userEvent.type(await canvas.findByTestId('create-family-description'), 'A family created in a flow test.');
    await userEvent.click(canvas.getByTestId('create-family-submit'));
    await canvas.findByTestId('owner-management-page');

    // invite the self-controlled node
    await userEvent.type(await canvas.findByTestId('invite-node-id'), String(NODE));
    await userEvent.click(canvas.getByTestId('invite-node-submit'));
    await userEvent.click(await screen.findByTestId('invite-node-confirm'));
    await canvas.findByTestId(`pending-invite-${NODE}`);

    // accept it from the operator tab (same account controls the node)
    await userEvent.click(canvas.getByTestId('family-tab-operator'));
    const group = await canvas.findByTestId(`node-invite-group-${NODE}`);
    await userEvent.click(await within(group).findByTestId('invite-card-1-accept'));
    await userEvent.click(await screen.findByTestId('invite-card-1-accept-confirm'));
    await canvas.findByTestId(`operator-node-${NODE}-family`);

    // kick it from the owner tab
    await userEvent.click(canvas.getByTestId('family-tab-owner'));
    await userEvent.click(await canvas.findByTestId(`member-joined-${NODE}-kick`));
    await userEvent.click(await screen.findByTestId(`member-joined-${NODE}-kick-confirm`));
    await waitFor(() => expect(canvas.queryByTestId(`member-joined-${NODE}`)).toBeNull());

    // disband the now-empty family
    await userEvent.click(await canvas.findByTestId('delete-family-button'));
    await userEvent.click(await screen.findByTestId('delete-family-button-confirm'));
    await canvas.findByTestId('create-family-name');
  },
};

/** Operator lifecycle: receive → accept (then leave) on one node, reject on another. */
export const OperatorLifecycle: Story = {
  decorators: [withFamiliesMock({ sender: MOCK_OPERATOR_ADDRESS, makeStore: buildOperatorFlowStore, latencyMs: 0 })],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByTestId('family-tab-operator'));

    // accept the invite on the accept-node
    const acceptGroup = await canvas.findByTestId(`node-invite-group-${MOCK_OPERATOR_FLOW_ACCEPT_NODE}`);
    await userEvent.click(await within(acceptGroup).findByTestId('invite-card-1-accept'));
    await userEvent.click(await screen.findByTestId('invite-card-1-accept-confirm'));
    await canvas.findByTestId(`operator-node-${MOCK_OPERATOR_FLOW_ACCEPT_NODE}-family`);

    // leave the family
    await userEvent.click(await canvas.findByTestId('leave-family-button'));
    await userEvent.click(await screen.findByTestId('leave-family-button-confirm'));

    // reject the invite on the reject-node
    const rejectGroup = await canvas.findByTestId(`node-invite-group-${MOCK_OPERATOR_FLOW_REJECT_NODE}`);
    await userEvent.click(await within(rejectGroup).findByTestId('invite-card-1-reject'));
    await userEvent.click(await screen.findByTestId('invite-card-1-reject-confirm'));
    await canvas.findByTestId(`node-invite-group-${MOCK_OPERATOR_FLOW_REJECT_NODE}-empty`);
  },
};
