import React from 'react';
import { CurrencyDenom, TNodeType } from '@nymproject/types';
import { MixnodeAmount, MixnodeData, Signature } from 'src/pages/bonding/types';
import MixnodeInitForm from './MixnodeInitForm';
import MixnodeSignatureForm from './MixnodeSignatureForm';
import NymnodeAmountForm from './NymnodeAmountForm';
import NymnodeSignatureForm from './NymnodeSignatureForm';

export const NymnodeForm = ({
  step,
  denom,
  nymnodeData,
  amountData,
  onValidateMixnodeData,
  onValidateAmountData,
  onValidateSignature,
}: {
  step: 1 | 2 | 3 | 4;
  nymnodeData: MixnodeData;
  amountData: MixnodeAmount;
  denom: CurrencyDenom;
  onValidateMixnodeData: (data: MixnodeData) => void;
  onValidateAmountData: (data: MixnodeAmount) => Promise<void>;
  onValidateSignature: (signature: Signature) => void;
}) => (
  <>
    {step === 1 && <MixnodeInitForm onNext={onValidateMixnodeData} mixnodeData={nymnodeData} />}
    {step === 2 && <NymnodeAmountForm denom={denom} amountData={amountData} onNext={onValidateAmountData} />}
    {step === 3 && <NymnodeSignatureForm nymnode={nymnodeData} amount={amountData} onNext={onValidateSignature} />}
  </>
);
