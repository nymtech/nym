import React, { useContext, useEffect } from 'react';
import { ConfirmTx } from 'src/components/ConfirmTX';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { useGetFee } from 'src/hooks/useGetFee';
import { BalanceWarning } from 'src/components/FeeWarning';
import { AppContext } from 'src/context';
import { TBondNymNodeArgs } from 'src/types';
import FormContextProvider, { useFormContext } from '../forms/nym-node/FormContext';
import NymNodeData from '../forms/nym-node/NymNodeData';
import NymNodeAmount from '../forms/nym-node/NymNodeAmount';
import NymNodeSignature from '../forms/nym-node/NymNodeSignature';

export const BondNymNodeModal = ({
  onClose,
  onBond,
}: {
  onClose: () => void;
  onBond: (data: TBondNymNodeArgs) => Promise<void>;
}) => {
  const { fee, resetFeeState, feeError } = useGetFee();
  const { userBalance } = useContext(AppContext);
  const { setStep, step, onError, signature, amountData, costParams, nymNodeData } = useFormContext();

  useEffect(() => {
    if (feeError) {
      onError(feeError);
    }
  }, [feeError]);

  const handleUpdateNymnodeData = async () => {
    setStep(2);
  };

  const handleBond = async () => {
    onBond({
      nymnode: nymNodeData,
      pledge: amountData,
      costParams,
      msgSignature: signature,
    });
  };

  const handleConfirm = async () => {};

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
        <ModalListItem label="Node identity key" value={nymNodeData.identity_key} divider />
        <ModalListItem label="Amount" value={`${amountData.amount} ${amountData.denom.toUpperCase()}`} divider />
        {fee.amount?.amount && userBalance.balance && (
          <BalanceWarning fee={fee.amount?.amount} tx={amountData.amount} />
        )}
      </ConfirmTx>
    );
  }

  if (step === 1) {
    return <NymNodeData onClose={onClose} onBack={onClose} onNext={handleUpdateNymnodeData} step={step} />;
  }

  if (step === 2) {
    return <NymNodeAmount onClose={onClose} onBack={() => setStep(1)} onNext={async () => setStep(3)} step={step} />;
  }

  if (step === 3) {
    return (
      <NymNodeSignature
        nymnode={nymNodeData}
        pledge={amountData}
        costParams={costParams}
        onNext={handleBond}
        onClose={onClose}
        onBack={() => setStep(2)}
        step={step}
      />
    );
  }

  return null;
};

export const BondNymNode = ({
  open,
  onClose,
  onBond,
}: {
  open: boolean;
  onClose: () => void;
  onBond: (data: TBondNymNodeArgs) => Promise<void>;
}) => {
  if (!open) {
    return null;
  }

  return (
    <FormContextProvider>
      <BondNymNodeModal onClose={onClose} onBond={onBond} />
    </FormContextProvider>
  );
};
