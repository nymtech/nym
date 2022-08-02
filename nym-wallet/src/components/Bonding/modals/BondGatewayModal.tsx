import React, { useEffect, useState } from 'react';
import { Box } from '@mui/material';
import { CurrencyDenom, TNodeType } from '@nymproject/types';
import { ConfirmTx } from 'src/components/ConfirmTX';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { TPoolOption } from 'src/components/TokenPoolSelector';
import { useGetFee } from 'src/hooks/useGetFee';
import { GatewayAmount, GatewayData } from 'src/pages/bonding/types';
import { simulateBondGateway, simulateVestingBondGateway } from 'src/requests';
import { TBondGatewayArgs } from 'src/types';
import { BondGatewayForm } from '../forms/BondGatewayForm';

const defaultMixnodeValues: GatewayData = {
  identityKey: '',
  sphinxKey: '',
  ownerSignature: '',
  location: '',
  host: '',
  version: '',
  mixPort: 1789,
  clientsPort: 1790,
};

const defaultAmountValues = (denom: CurrencyDenom) => ({
  amount: { amount: '100', denom },
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
  const [gatewayData, setGatewayData] = useState<GatewayData>(defaultMixnodeValues);
  const [amountData, setAmountData] = useState<GatewayAmount>(defaultAmountValues(denom));

  const { fee, getFee, resetFeeState, feeError } = useGetFee();

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
    setStep(1);
  };

  const handleUpdateGatwayData = (data: GatewayData) => {
    setGatewayData(data);
    setStep(2);
  };

  const handleUpdateAmountData = async (data: GatewayAmount) => {
    setAmountData(data);
    const payload = {
      pledge: data.amount,
      ownerSignature: gatewayData.ownerSignature,
      gateway: {
        ...gatewayData,
        host: gatewayData.host,
        version: gatewayData.version,
        mix_port: gatewayData.mixPort,
        clients_port: gatewayData.clientsPort,
        sphinx_key: gatewayData.sphinxKey,
        identity_key: gatewayData.identityKey,
        location: gatewayData.location,
      },
    };

    if (data.tokenPool === 'balance') {
      await getFee<TBondGatewayArgs>(simulateBondGateway, payload);
    } else {
      await getFee<TBondGatewayArgs>(simulateVestingBondGateway, payload);
    }
  };

  const handleConfirm = async () => {
    await onBondGateway(
      {
        pledge: amountData.amount,
        ownerSignature: gatewayData.ownerSignature,
        gateway: {
          ...gatewayData,
          host: gatewayData.host,
          version: gatewayData.version,
          mix_port: gatewayData.mixPort,
          clients_port: gatewayData.clientsPort,
          sphinx_key: gatewayData.sphinxKey,
          identity_key: gatewayData.identityKey,
          location: gatewayData.location,
        },
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
      subHeader={`Step ${step}/2`}
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
          onSelectNodeType={onSelectNodeType}
        />
      </Box>
    </SimpleModal>
  );
};
