import * as React from 'react';
import { useState } from 'react';
import { MajorCurrencyAmount } from '@nymproject/types';
import ProfitMarginModal from './ProfitMarginModal';
import { BondedMixnode } from '../../../../context';

interface Props {
  mixnode: BondedMixnode;
  open: boolean;
}

const MOCK_ESTIMATED_OP_REWARD: MajorCurrencyAmount = { amount: '42', denom: 'NYM' };

const NodeSettings = ({ open, mixnode }: Props) => {
  const [profitMargin, setProfitMargin] = useState<number>();
  const [pmModalOpen, setPmModalOpen] = useState<boolean>(true);

  if (!open) return null;

  return (
    <ProfitMarginModal
      open={pmModalOpen}
      onClose={() => {
        setPmModalOpen(false);
      }}
      onSubmit={async (pm) => {
        setProfitMargin(pm);
        setPmModalOpen(false);
      }}
      estimatedOpReward={MOCK_ESTIMATED_OP_REWARD}
      currentPm={mixnode.profitMargin}
    />
  );
};

export default NodeSettings;
