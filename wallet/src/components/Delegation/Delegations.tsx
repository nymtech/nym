import React from 'react';
import { Box, Typography } from '@mui/material';
import { DelegationWithEverything } from '@nymproject/types';
import { Link } from '@nymproject/react';
import { DelegationList } from './DelegationList';
import { DelegationListItemActions } from './DelegationActions';

export const Delegations: FCWithChildren<{
  isLoading?: boolean;
  items: DelegationWithEverything[];
  explorerUrl: string;
  onDelegationItemActionClick?: (item: DelegationWithEverything, action: DelegationListItemActions) => void;
}> = ({ isLoading, items, explorerUrl, onDelegationItemActionClick }) => (
  <>
    <DelegationList
      isLoading={isLoading}
      items={items}
      explorerUrl={explorerUrl}
      onItemActionClick={onDelegationItemActionClick}
    />
    <Box sx={{ mt: 3 }}>
      <Link href={`${explorerUrl}/network-components/mixnodes/`} target="_blank" rel="noreferrer">
        Check the{' '}
        <Typography color="primary.main" component="span">
          list of mixnodes
        </Typography>{' '}
        for uptime and performance to make delegation decisions
      </Link>
    </Box>
  </>
);
