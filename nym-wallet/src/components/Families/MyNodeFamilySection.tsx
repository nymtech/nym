/* eslint-disable @typescript-eslint/naming-convention */
import React from 'react';
import { Stack, Typography } from '@mui/material';
import { NodeFamilyId } from 'src/types/families';
import { useFamiliesContext, useFamilyById, useFamilyMembership } from 'src/context/families';
import { FamilyContentPanel } from './FamilyContentPanel';
import { LeaveFamilyButton } from './LeaveFamilyButton';

export interface MyNodeFamilySectionProps {
  nodeId: number;
  /** When set, hide this block if the node is already in that family (the owner page covers it). */
  managedFamilyId?: NodeFamilyId;
  /** Name of the family this account manages, used when the node joined a different one. */
  managedFamilyName?: string;
  onLeave: () => void | Promise<void>;
}

const FamilyName = ({ name }: { name: string }) => (
  <Typography component="span" variant="body2" color="primary.main" fontWeight={600}>
    {name}
  </Typography>
);

/** Shows the bonded node's current family membership. Belongs on the My family tab. */
export const MyNodeFamilySection = ({
  nodeId,
  managedFamilyId,
  managedFamilyName,
  onLeave,
}: MyNodeFamilySectionProps) => {
  const ctx = useFamiliesContext();
  const membership = useFamilyMembership(nodeId);
  const familyId = membership.data?.family_id ?? undefined;
  const family = useFamilyById(familyId);

  if (familyId === undefined || !family.data) return null;

  const isManagedHere = managedFamilyId !== undefined && familyId === managedFamilyId;
  if (isManagedHere) return null;

  const isOwnWallet =
    ctx.ownerAddress !== undefined && family.data.owner.toLowerCase() === ctx.ownerAddress.toLowerCase();
  // Own-family membership is shown on the management page, never as a standalone card.
  if (isOwnWallet) return null;

  const isBusy = ctx.executingAction === 'leave';

  return (
    <FamilyContentPanel>
      <Stack spacing={2} data-testid={`my-node-family-${nodeId}`} data-membership="external">
        <Typography variant="body2" color="text.secondary">
          Your bonded node {nodeId} belongs to <FamilyName name={family.data.name} />.
        </Typography>
        {managedFamilyId !== undefined && managedFamilyName ? (
          <Typography variant="body2" color="text.secondary">
            This is separate from <FamilyName name={managedFamilyName} />, the family you manage on this account.
          </Typography>
        ) : (
          <Typography variant="body2" color="text.secondary">
            Leave this family if you want to create one with your own wallet.
          </Typography>
        )}
        <LeaveFamilyButton familyName={family.data.name} isBusy={isBusy} onLeave={onLeave} />
      </Stack>
    </FamilyContentPanel>
  );
};
