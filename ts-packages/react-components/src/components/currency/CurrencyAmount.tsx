import * as React from 'react';
import type { MajorCurrencyAmount } from '@nymproject/types';
import { Stack, SxProps, Typography } from '@mui/material';

export const CURRENCY_AMOUNT_SPACING = 0.35;

const toReverseChunks = (value: String, size: number = 3): Array<string> => {
  const reversed = value.split('').reverse();
  const chunks: Array<Array<String>> = [];
  let chunksIndex = 0;
  reversed.forEach((char, index) => {
    if (index > 0 && index % size === 0) {
      chunksIndex += 1;
    }
    if (!chunks[chunksIndex]) {
      chunks.push([]);
    }
    chunks[chunksIndex].push(char);
  });
  return chunks.map((chars) => chars.reverse().join('')).reverse();
};

const toChunks = (value: String, size: number = 3): Array<string> => {
  const chunks: Array<Array<String>> = [];
  let chunksIndex = 0;
  value.split('').forEach((char, index) => {
    if (index > 0 && index % size === 0) {
      chunksIndex += 1;
    }
    if (!chunks[chunksIndex]) {
      chunks.push([]);
    }
    chunks[chunksIndex].push(char);
  });
  return chunks.map((chars) => chars.join(''));
};

export const CurrencyAmount: React.FC<{
  majorAmount?: MajorCurrencyAmount;
  showSeparators?: boolean;
  sx?: SxProps;
}> = ({ majorAmount, sx, showSeparators = true }) => {
  if (!majorAmount || !majorAmount.amount) {
    return (
      <Stack direction="row" sx={sx}>
        <span>-</span>
      </Stack>
    );
  }
  if (!showSeparators) {
    return (
      <Stack direction="row" sx={sx}>
        <span>{majorAmount.amount}</span>
      </Stack>
    );
  }

  if (majorAmount.amount.trim() === '0') {
    return (
      <Stack direction="row" sx={sx}>
        <span>0</span>
      </Stack>
    );
  }

  const parts = majorAmount.amount.split('.');
  if (parts.length !== 2) {
    return <Typography sx={sx}>Error</Typography>;
  }

  const wholePart = toReverseChunks(parts[0]);
  const fractionPart = toChunks(parts[1]);

  return (
    <Stack direction="row" sx={sx}>
      <Stack direction="row" spacing={CURRENCY_AMOUNT_SPACING}>
        {wholePart.map((chunk) => (
          <span key={chunk}>{chunk}</span>
        ))}
      </Stack>
      <span>.</span>
      <Stack direction="row" spacing={CURRENCY_AMOUNT_SPACING}>
        {fractionPart.map((chunk) => (
          <span key={chunk}>{chunk}</span>
        ))}
      </Stack>
    </Stack>
  );
};
