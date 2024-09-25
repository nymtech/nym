import React, { createContext, useContext, useMemo, useState } from 'react';
import { CurrencyDenom } from '@nymproject/types';
import { TBondNymNodeArgs, TBondMixNodeArgs } from 'src/types';

const defaultNymNodeValues: TBondNymNodeArgs['nymNode'] = {
  identity_key: 'H6rXWgsW89QsVyaNSS3qBe9zZFLhBS6Gn3YRkGFSoFW9',
  custom_http_port: 1,
  host: '1.1.1.1',
};

const defaultCostParams = (denom: CurrencyDenom): TBondNymNodeArgs['costParams'] => ({
  interval_operating_cost: { amount: '40', denom },
  profit_margin_percent: '10',
});

const defaultAmount = (denom: CurrencyDenom): TBondMixNodeArgs['pledge'] => ({
  amount: '100',
  denom,
});

interface FormContextType {
  step: 1 | 2 | 3 | 4;
  setStep: React.Dispatch<React.SetStateAction<1 | 2 | 3 | 4>>;
  nymNodeData: TBondNymNodeArgs['nymNode'];
  setNymNodeData: React.Dispatch<React.SetStateAction<TBondNymNodeArgs['nymNode']>>;
  costParams: TBondNymNodeArgs['costParams'];
  setCostParams: React.Dispatch<React.SetStateAction<TBondNymNodeArgs['costParams']>>;
  amountData: TBondMixNodeArgs['pledge'];
  setAmountData: React.Dispatch<React.SetStateAction<TBondMixNodeArgs['pledge']>>;
  signature?: string;
  setSignature: React.Dispatch<React.SetStateAction<string | undefined>>;
  onError: (e: string) => void;
}

const FormContext = createContext<FormContextType>({
  step: 1,
  setStep: () => {},
  nymNodeData: defaultNymNodeValues,
  setNymNodeData: () => {},
  costParams: defaultCostParams('nym'),
  setCostParams: () => {},
  amountData: defaultAmount('nym'),
  setAmountData: () => {},
  signature: undefined,
  setSignature: () => {},

  onError: () => {},
});

const FormContextProvider = ({ children }: { children: React.ReactNode }) => {
  // TODO - Make denom dynamic
  const denom = 'nym';

  const [step, setStep] = useState<1 | 2 | 3 | 4>(1);
  const [nymNodeData, setNymNodeData] = useState<TBondNymNodeArgs['nymNode']>(defaultNymNodeValues);
  const [costParams, setCostParams] = useState<TBondNymNodeArgs['costParams']>(defaultCostParams(denom));
  const [amountData, setAmountData] = useState<TBondNymNodeArgs['pledge']>(defaultAmount(denom));
  const [signature, setSignature] = useState<string>();

  const onError = (e: string) => {
    console.error(e);
  };

  const value = useMemo(
    () => ({
      step,
      setStep,
      nymNodeData,
      setNymNodeData,
      costParams,
      setCostParams,
      amountData,
      setAmountData,
      signature,
      setSignature,
      onError,
    }),
    [
      step,
      setStep,
      nymNodeData,
      setNymNodeData,
      costParams,
      setCostParams,
      amountData,
      setAmountData,
      signature,
      setSignature,
      onError,
    ],
  );

  return <FormContext.Provider value={value}>{children}</FormContext.Provider>;
};

export const useFormContext = () => useContext(FormContext);

export default FormContextProvider;
