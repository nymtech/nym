import React, { useContext, useEffect, useState } from 'react';
import { Box } from '@mui/material';
import { CurrencyDenom, TNodeType } from '@nymproject/types';
import { ConfirmTx } from 'src/components/ConfirmTX';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { TPoolOption } from 'src/components/TokenPoolSelector';
import { useGetFee } from 'src/hooks/useGetFee';
import { GatewayAmount, GatewayData, Signature } from 'src/pages/bonding/types';
import { simulateBondGateway, simulateVestingBondGateway } from 'src/requests';
import { TBondGatewayArgs } from 'src/types';
import { BalanceWarning } from 'src/components/FeeWarning';
import { AppContext } from 'src/context';
import { gatewayToTauri } from '../utils';
import { BondGatewayForm } from '../forms/legacyForms/BondGatewayForm';

const defaultGatewayValues: GatewayData = {
  identityKey: '',
  sphinxKey: '',
  ownerSignature: '',
  location: '',
  host: '',
  version: '',
  mixPort: 1789,
  clientsPort: 9000,
};

const defaultAmountValues = (denom: CurrencyDenom) => ({
  amount: { amount: '100', denom },
  operatorCost: { amount: '40', denom },
  tokenPool: 'balance',
});

export const BondGatewayModal = ({
  denom,
  hasVestingTokens,
  onBondGateway,
  onSelectNodeType,
  onClose,
  onError,
}: {
  denom: CurrencyDenom;
  hasVestingTokens: boolean;
  onBondGateway: (data: TBondGatewayArgs, tokenPool: TPoolOption) => void;
  onSelectNodeType: (type: TNodeType) => void;
  onClose: () => void;
  onError: (e: string) => void;
}) => {
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [gatewayData, setGatewayData] = useState<GatewayData>(defaultGatewayValues);
  const [amountData, setAmountData] = useState<GatewayAmount>(defaultAmountValues(denom));
  const [signature, setSignature] = useState<string>();

  const { fee, getFee, resetFeeState, feeError } = useGetFee();
  const { userBalance } = useContext(AppContext);

  useEffect(() => {
    if (feeError) {
      onError(feeError);
    }
  }, [feeError]);

  const validateStep = async (s: number) => {
    const event = new CustomEvent('validate_bond_gateway_step', { detail: { step: s } });
    window.dispatchEvent(event);
  };

  const handleBack = () => {
    if (step === 2) {
      setStep(1);
    } else if (step === 3) {
      setStep(2);
    }
  };

  const handleUpdateGatwayData = (data: GatewayData) => {
    setGatewayData(data);
    setStep(2);
  };

  const handleUpdateAmountData = async (data: GatewayAmount) => {
    setAmountData(data);
    setStep(3);
  };

  const handleUpdateSignature = async (data: Signature) => {
    setSignature(data.signature);

    const payload = {
      pledge: amountData.amount,
      msgSignature: data.signature,
      gateway: gatewayToTauri(gatewayData),
    };

    if (amountData.tokenPool === 'balance') {
      await getFee<TBondGatewayArgs>(simulateBondGateway, payload);
    } else {
      await getFee<TBondGatewayArgs>(simulateVestingBondGateway, payload);
    }
  };

  const handleConfirm = async () => {
    await onBondGateway(
      {
        pledge: amountData.amount,
        msgSignature: signature as string,
        gateway: gatewayToTauri(gatewayData),
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
        <ModalListItem label="Node identity key" value={gatewayData.identityKey} divider />
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
      onBack={step === 2 ? handleBack : undefined}
      onClose={onClose}
      header="Bond gateway"
      subHeader={`Step ${step}/3`}
      okLabel="Next"
    >
      <Box sx={{ mb: 2 }}>
        <BondGatewayForm
          step={step}
          denom={denom}
          gatewayData={gatewayData}
          amountData={amountData}
          hasVestingTokens={hasVestingTokens}
          onValidateGatewayData={handleUpdateGatwayData}
          onValidateAmountData={handleUpdateAmountData}
          onValidateSignature={handleUpdateSignature}
          onSelectNodeType={onSelectNodeType}
        />
      </Box>
    </SimpleModal>
  );
};
