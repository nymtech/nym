import { useContext, useEffect } from 'react';
import { ConfirmTx } from 'src/components/ConfirmTX';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { useGetFee } from 'src/hooks/useGetFee';
import { MixnodeAmount, Signature } from 'src/pages/bonding/types';
import { BalanceWarning } from 'src/components/FeeWarning';
import { AppContext } from 'src/context';
import FormContextProvider, { useFormContext } from '../forms/nym-node/FormContext';
import NymNodeData from '../forms/nym-node/NymNodeData';
import NymNodeAmount from '../forms/nym-node/NymNodeAmount';
import NymNodeSignature from '../forms/nym-node/NymNodeSignature';

export const BondNymNodeModal = ({ onClose }: { onClose: () => void }) => {
  const { fee, getFee, resetFeeState, feeError } = useGetFee();
  const { userBalance } = useContext(AppContext);
  const { setStep, step, onError, setSignature, amountData, costParams, nymNodeData } = useFormContext();

  useEffect(() => {
    if (feeError) {
      onError(feeError);
    }
  }, [feeError]);

  const handleBack = () => {
    setStep(step);
  };

  const handleUpdateMixnodeData = async () => {
    setStep(2);
  };

  const handleUpdateAmountData = async (data: MixnodeAmount) => {
    setStep(3);
  };

  const handleUpdateSignature = async (data: Signature) => {
    setSignature(data.signature);
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
    return <NymNodeData onClose={onClose} onBack={onClose} onNext={handleUpdateMixnodeData} step={step} />;
  }

  if (step === 2) {
    return <NymNodeAmount onClose={onClose} onBack={() => setStep(1)} onNext={async () => setStep(3)} step={step} />;
  }

  if (step === 3) {
    return (
      <NymNodeSignature
        nymNode={nymNodeData}
        pledge={amountData}
        costParams={costParams}
        onNext={handleUpdateSignature}
        onClose={onClose}
        onBack={() => setStep(2)}
        step={step}
      />
    );
  }

  return null;
};

export const BondNymNodeModalWithState = ({ open, onClose }: { open: boolean; onClose: () => void }) => {
  if (!open) {
    return null;
  }

  return (
    <FormContextProvider>
      <BondNymNodeModal onClose={onClose} />
    </FormContextProvider>
  );
};
