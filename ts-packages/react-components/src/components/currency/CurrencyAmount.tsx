/* eslint-disable react/no-array-index-key */
import * as React from 'react';
import type { DecCoin } from '@nymproject/types';
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

export const CurrencyAmountString: React.FC<{
  majorAmount?: string;
  showSeparators?: boolean;
  hideFractions?: boolean;
  sx?: SxProps;
}> = ({ majorAmount, sx, showSeparators = true, hideFractions = false }) => {
  if (!majorAmount) {
    return (
      <Stack direction="row" sx={sx} fontSize="inherit">
        <span>-</span>
      </Stack>
    );
  }
  if (!showSeparators) {
    return (
      <Stack direction="row" sx={sx} fontSize="inherit">
        <span>{majorAmount}</span>
      </Stack>
    );
  }

  if (majorAmount.trim() === '0') {
    return (
      <Stack direction="row" sx={sx}>
        <span>0</span>
      </Stack>
    );
  }

  const parts = majorAmount.split('.');
  if (parts.length !== 1 && parts.length !== 2) {
    return (
      <Typography sx={sx} fontSize="inherit">
        Error
      </Typography>
    );
  }

  const wholePartFormatted = new Intl.NumberFormat('en-US', { style: 'decimal' })
    .format(Number.parseFloat(parts[0]))
    .replaceAll(',', ' ');

  if (parts.length === 1 || hideFractions) {
    return (
      <Stack direction="row" sx={sx}>
        <span>{wholePartFormatted}</span>
      </Stack>
    );
  }

  return (
    <Stack direction="row" sx={sx}>
      <span>{wholePartFormatted}</span>
      <span>.</span>
      <span>{parts[1]}</span>
    </Stack>
  );
};

export const CurrencyAmount: React.FC<{
  majorAmount?: DecCoin;
  showSeparators?: boolean;
  hideFractions?: boolean;
  sx?: SxProps;
}> = ({ majorAmount, ...props }) => <CurrencyAmountString majorAmount={majorAmount?.amount} {...props} />;
