import { useState } from 'react';
import { Box, Button, Stack, Tooltip, Typography } from '@mui/material';
import { Delegate, Undelegate } from '../../svg-icons';
import { ActionsMenu, ActionsMenuItem } from '../ActionsMenu';
import { DelegateListItemPending } from './types';

export type DelegationListItemActions = 'delegate' | 'undelegate' | 'redeem' | 'compound';

const BUTTON_SIZE = '32px';
const MIN_WIDTH = '150px';

export const DelegationActions: FCWithChildren<{
  onActionClick?: (action: DelegationListItemActions) => void;
  isPending?: DelegateListItemPending;
  disableRedeemingRewards?: boolean;
}> = ({ disableRedeemingRewards, onActionClick, isPending }) => {
  if (isPending) {
    return (
      <Box py={0.5} fontSize="inherit" minWidth={MIN_WIDTH} minHeight={BUTTON_SIZE}>
        <Tooltip title="There will be a new epoch roughly every hour when your changes will take effect" arrow>
          <Typography fontSize="inherit" color="text.disabled">
            Pending {isPending.actionType === 'delegate' ? 'delegation' : 'undelegation'}...
          </Typography>
        </Tooltip>
      </Box>
    );
  }
  return (
    <Stack spacing={2} direction="row" minWidth={MIN_WIDTH}>
      <Tooltip title="Delegate more" arrow>
        <Button
          onClick={() => (onActionClick ? onActionClick('delegate') : undefined)}
          variant="contained"
          disableElevation
          sx={{ maxWidth: BUTTON_SIZE, minWidth: BUTTON_SIZE, height: BUTTON_SIZE, padding: 0 }}
        >
          <Delegate fontSize="small" />
        </Button>
      </Tooltip>
      <Tooltip title="Undelegate" arrow>
        <Button
          variant="outlined"
          sx={{ maxWidth: BUTTON_SIZE, minWidth: BUTTON_SIZE, height: BUTTON_SIZE, padding: 0 }}
          onClick={() => (onActionClick ? onActionClick('undelegate') : undefined)}
        >
          <Undelegate fontSize="small" />
        </Button>
      </Tooltip>
      <Tooltip title={disableRedeemingRewards ? 'There are no rewards to claim' : 'Claim rewards'} arrow>
        <span>
          <Button
            disabled={disableRedeemingRewards}
            onClick={() => (onActionClick ? onActionClick('redeem') : undefined)}
            variant="outlined"
            color="secondary"
            sx={{ maxWidth: BUTTON_SIZE, minWidth: BUTTON_SIZE, height: BUTTON_SIZE, padding: 0 }}
          >
            C
          </Button>
        </span>
      </Tooltip>
    </Stack>
  );
};

export const DelegationsActionsMenu: FCWithChildren<{
  onActionClick?: (action: DelegationListItemActions) => void;
  disableRedeemingRewards?: boolean;
  disableDelegateMore?: boolean | null;
}> = ({ disableRedeemingRewards, disableDelegateMore, onActionClick }) => {
  const [isOpen, setIsOpen] = useState(false);

  const handleOpenMenu = () => setIsOpen(true);
  const handleOnClose = () => setIsOpen(false);

  const handleActionSelect = (action: DelegationListItemActions) => {
    onActionClick?.(action);
    handleOnClose();
  };

  return (
    <ActionsMenu open={isOpen} onOpen={handleOpenMenu} onClose={handleOnClose}>
      <ActionsMenuItem
        title="Delegate more"
        description={disableDelegateMore ? 'This node is unbonding, action disabled.' : undefined}
        Icon={<Delegate />}
        onClick={() => handleActionSelect('delegate')}
        disabled={Boolean(disableDelegateMore)}
      />
      <ActionsMenuItem title="Undelegate" Icon={<Undelegate />} onClick={() => handleActionSelect('undelegate')} />
      <ActionsMenuItem
        title="Claim rewards"
        description="Transfer your rewards to your balance"
        Icon={<Typography sx={{ pl: 1, fontWeight: 700 }}>C</Typography>}
        onClick={() => handleActionSelect('redeem')}
        disabled={disableRedeemingRewards}
      />
    </ActionsMenu>
  );
};
