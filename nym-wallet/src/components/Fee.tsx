import React, { useState, useEffect, useContext } from 'react';
import { Typography } from '@mui/material';
import { Operation } from '../types';
import { getGasFee } from '../requests';
import { AppContext } from '../context/main';

export const Fee = ({ feeType }: { feeType: Operation }) => {
  const [fee, setFee] = useState<string>();
  const { currency } = useContext(AppContext);

  const getFee = async () => {
    const res = await getGasFee(feeType);
    setFee(res.amount);
  };

  useEffect(() => {
    getFee();
  }, []);

  if (fee) {
    return (
      <Typography sx={{ color: 'nym.fee', fontWeight: 600 }}>
        Estimated fee for this transaction: {`${fee} ${currency?.major}`}{' '}
      </Typography>
    );
  }

  return null;
};
