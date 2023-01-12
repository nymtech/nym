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

export const CurrencyAmountString: FCWithChildren<{
  majorAmount?: string;
  showSeparators?: boolean;
  sx?: SxProps;
}> = ({ majorAmount, sx, showSeparators = true }) => {
  if (!majorAmount) {
    return (
      <Stack direction="row" sx={sx}>
        <span>-</span>
      </Stack>
    );
  }
  if (!showSeparators) {
    return (
      <Stack direction="row" sx={sx}>
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
    return <Typography sx={sx}>Error</Typography>;
  }

  const wholePart = toReverseChunks(parts[0]);
  const fractionPart = parts[1] ? toChunks(parts[1]) : [];

  return (
    <Stack direction="row" sx={sx}>
      <Stack direction="row" spacing={CURRENCY_AMOUNT_SPACING}>
        {wholePart.map((chunk, index) => (
          <span key={`${chunk}-${index}`}>{chunk}</span>
        ))}
      </Stack>
      {parts[1] && (
        <>
          <span>.</span>
          <Stack direction="row" spacing={CURRENCY_AMOUNT_SPACING}>
            {fractionPart.map((chunk, index) => (
              <span key={`${chunk}-${index}`}>{chunk}</span>
            ))}
          </Stack>
        </>
      )}
    </Stack>
  );
};

export const CurrencyAmount: FCWithChildren<{
  majorAmount?: DecCoin;
  showSeparators?: boolean;
  sx?: SxProps;
}> = ({ majorAmount, ...props }) => <CurrencyAmountString majorAmount={majorAmount?.amount} {...props} />;
