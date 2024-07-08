import React from 'react';
import { Box } from '@mui/material';
import { NodeTypeSelector } from '@src/components';
import { CurrencyDenom, TNodeType } from '@nymproject/types';
import { GatewayAmount, GatewayData, Signature } from '@src/pages/bonding/types';
import GatewayInitForm from './GatewayInitForm';
import GatewayAmountForm from './GatewayAmountForm';
import GatewaySignatureForm from './GatewaySignatureForm';

export const BondGatewayForm = ({
  step,
  denom,
  gatewayData,
  amountData,
  hasVestingTokens,
  onSelectNodeType,
  onValidateGatewayData,
  onValidateAmountData,
  onValidateSignature,
}: {
  step: 1 | 2 | 3 | 4;
  gatewayData: GatewayData;
  amountData: GatewayAmount;
  denom: CurrencyDenom;
  hasVestingTokens: boolean;
  onSelectNodeType: (nodeType: TNodeType) => void;
  onValidateGatewayData: (data: GatewayData) => void;
  onValidateAmountData: (data: GatewayAmount) => Promise<void>;
  onValidateSignature: (signature: Signature) => void;
}) => (
  <>
    {step === 1 && (
      <>
        <Box sx={{ mb: 2 }}>
          <NodeTypeSelector disabled={false} setNodeType={onSelectNodeType} nodeType="gateway" />
        </Box>
        <GatewayInitForm onNext={onValidateGatewayData} gatewayData={gatewayData} />
      </>
    )}
    {step === 2 && (
      <GatewayAmountForm
        denom={denom}
        amountData={amountData}
        hasVestingTokens={hasVestingTokens}
        onNext={onValidateAmountData}
      />
    )}
    {step === 3 && <GatewaySignatureForm gateway={gatewayData} amount={amountData} onNext={onValidateSignature} />}
  </>
);
