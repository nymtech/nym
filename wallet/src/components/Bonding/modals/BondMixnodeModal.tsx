import React, { useContext, useEffect, useState } from 'react';
import { CurrencyDenom, TNodeType } from '@nymproject/types';
import { ConfirmTx } from '@src/components/ConfirmTX';
import { ModalListItem } from '@src/components/Modals/ModalListItem';
import { SimpleModal } from '@src/components/Modals/SimpleModal';
import { TPoolOption } from '@src/components/TokenPoolSelector';
import { useGetFee } from '@src/hooks/useGetFee';
import { MixnodeAmount, MixnodeData, Signature } from '@src/pages/bonding/types';
import { simulateBondMixnode, simulateVestingBondMixnode } from '@src/requests';
import { TBondMixNodeArgs } from '@src/types';
import { BalanceWarning } from '@src/components/FeeWarning';
import { AppContext } from '@src/context';
import { BondMixnodeForm } from '../forms/BondMixnodeForm';
import { costParamsToTauri, mixnodeToTauri } from '../utils';

const defaultMixnodeValues: MixnodeData = {
  identityKey: '',
  sphinxKey: '',
  ownerSignature: '',
  host: '',
  version: '',
  mixPort: 1789,
  verlocPort: 1790,
  httpApiPort: 8000,
};

const defaultAmountValues = (denom: CurrencyDenom) => ({
  amount: { amount: '100', denom },
  operatorCost: { amount: '40', denom },
  profitMargin: '10',
  tokenPool: 'balance',
});

export const BondMixnodeModal = ({
  denom,
  hasVestingTokens,
  onBondMixnode,
  onSelectNodeType,
  onClose,
  onError,
}: {
  denom: CurrencyDenom;
  hasVestingTokens: boolean;
  onBondMixnode: (data: TBondMixNodeArgs, tokenPool: TPoolOption) => void;
  onSelectNodeType: (type: TNodeType) => void;
  onClose: () => void;
  onError: (e: string) => void;
}) => {
  const [step, setStep] = useState<1 | 2 | 3 | 4>(1);
  const [mixnodeData, setMixnodeData] = useState<MixnodeData>(defaultMixnodeValues);
  const [amountData, setAmountData] = useState<MixnodeAmount>(defaultAmountValues(denom));
  const [signature, setSignature] = useState<string>();

  const { fee, getFee, resetFeeState, feeError } = useGetFee();
  const { userBalance } = useContext(AppContext);

  useEffect(() => {
    if (feeError) {
      onError(feeError);
    }
  }, [feeError]);

  const validateStep = async (s: number) => {
    const event = new CustomEvent('validate_bond_mixnode_step', { detail: { step: s } });
    window.dispatchEvent(event);
  };

  const handleBack = () => {
    if (step === 2) {
      setStep(1);
    } else if (step === 3) {
      setStep(2);
    }
  };

  const handleUpdateMixnodeData = (data: MixnodeData) => {
    setMixnodeData(data);
    setStep(2);
  };

  const handleUpdateAmountData = async (data: MixnodeAmount) => {
    setAmountData({ ...data });
    setStep(3);
  };

  const handleUpdateSignature = async (data: Signature) => {
    setSignature(data.signature);

    const payload = {
      pledge: amountData.amount,
      msgSignature: data.signature,
      mixnode: mixnodeToTauri(mixnodeData),
      costParams: costParamsToTauri(amountData),
    };

    if (amountData.tokenPool === 'balance') {
      await getFee<TBondMixNodeArgs>(simulateBondMixnode, payload);
    } else {
      await getFee<TBondMixNodeArgs>(simulateVestingBondMixnode, payload);
    }
  };

  const handleConfirm = async () => {
    await onBondMixnode(
      {
        pledge: amountData.amount,
        msgSignature: signature as string,
        mixnode: mixnodeToTauri(mixnodeData),
        costParams: costParamsToTauri(amountData),
      },
      amountData.tokenPool as TPoolOption,
    );
  };

  if (fee) {
    return (
      <ConfirmTx
        open
        header="Bond details"
        fee={fee}
        onClose={onClose}
        onPrev={resetFeeState}
        onConfirm={handleConfirm}
      >
        <ModalListItem label="Node identity key" value={mixnodeData.identityKey} divider />
        <ModalListItem
          label="Amount"
          value={`${amountData.amount.amount} ${amountData.amount.denom.toUpperCase()}`}
          divider
        />
        {fee.amount?.amount && userBalance.balance && (
          <BalanceWarning fee={fee.amount?.amount} tx={amountData.amount.amount} />
        )}
      </ConfirmTx>
    );
  }

  return (
    <SimpleModal
      open
      onOk={async () => {
        await validateStep(step);
      }}
      onBack={step === 2 || step === 3 ? handleBack : undefined}
      onClose={onClose}
      header="Bond mixnode"
      subHeader={`Step ${step}/3`}
      okLabel="Next"
    >
      <BondMixnodeForm
        step={step}
        denom={denom}
        mixnodeData={mixnodeData}
        amountData={amountData}
        hasVestingTokens={hasVestingTokens}
        onValidateMixnodeData={handleUpdateMixnodeData}
        onValidateAmountData={handleUpdateAmountData}
        onValidateSignature={handleUpdateSignature}
        onSelectNodeType={onSelectNodeType}
      />
    </SimpleModal>
  );
};
