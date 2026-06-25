/* eslint-disable @typescript-eslint/naming-convention */
import React, { useState } from 'react';
import { Alert, Box, Button, Stack, TextField, Typography } from '@mui/material';
import { DecCoin } from '@nymproject/types';
import { NymCard } from '../NymCard';
import { byteLength, formatCoin, isInsufficientBalance, sanitizeInput } from './helpers';

export interface CreateFamilyFormProps {
  /** Creation fee read from chain config (never hardcoded). */
  fee: DecCoin;
  nameLimit: number;
  descriptionLimit: number;
  /** Connected account balance, used to pre-check insufficient funds. */
  balance?: DecCoin;
  isSubmitting?: boolean;
  /** Disable submit while another family action is in flight. */
  isBlocked?: boolean;
  /** Shown before submit when creation should be prevented. */
  blockedMessage?: string;
  /** When true, render inline without an outer NymCard (for use inside FamilyContentPanel). */
  embedded?: boolean;
  /** Contract/fee error surfaced after a failed submit. */
  errorMessage?: string;
  onSubmit: (name: string, description: string) => void;
}

export const CreateFamilyForm = ({
  fee,
  nameLimit,
  descriptionLimit,
  balance,
  isSubmitting,
  isBlocked,
  blockedMessage,
  errorMessage,
  embedded,
  onSubmit,
}: CreateFamilyFormProps) => {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');

  const nameBytes = byteLength(name);
  const descBytes = byteLength(description);
  const nameTooLong = nameBytes > nameLimit;
  const descTooLong = descBytes > descriptionLimit;
  const insufficient = isInsufficientBalance(balance, fee);
  const canSubmit =
    name.trim().length > 0 && !nameTooLong && !descTooLong && !insufficient && !isSubmitting && !isBlocked;

  const handleSubmit = () => {
    if (!canSubmit) return;
    onSubmit(sanitizeInput(name), sanitizeInput(description));
  };

  const body = (
    <Stack spacing={3}>
      <Typography variant="body2" color="text.secondary">
        Group your node with others under a family wallet. Creating a family requires a refundable fee of{' '}
        <strong>{formatCoin(fee)}</strong>, returned in full when the family is dissolved.
      </Typography>

      <TextField
        label="Family name"
        value={name}
        onChange={(e) => setName(e.target.value)}
        error={nameTooLong}
        helperText={
          nameTooLong ? `Name is ${nameBytes}/${nameLimit} bytes — too long` : `${nameBytes}/${nameLimit} bytes`
        }
        fullWidth
        inputProps={{ 'data-testid': 'create-family-name' }}
      />

      <TextField
        label="Description"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        error={descTooLong}
        helperText={
          descTooLong
            ? `Description is ${descBytes}/${descriptionLimit} bytes — too long`
            : `${descBytes}/${descriptionLimit} bytes`
        }
        fullWidth
        multiline
        minRows={2}
        inputProps={{ 'data-testid': 'create-family-description' }}
      />

      {blockedMessage && (
        <Alert severity="warning" data-testid="create-family-blocked">
          {blockedMessage}
        </Alert>
      )}

      {insufficient && (
        <Alert severity="error" data-testid="create-family-insufficient">
          Insufficient balance — you need at least {formatCoin(fee)} plus gas to create a family.
        </Alert>
      )}

      {errorMessage && (
        <Alert severity="error" data-testid="create-family-error">
          {errorMessage}
        </Alert>
      )}

      <Box>
        <Button variant="contained" disabled={!canSubmit} onClick={handleSubmit} data-testid="create-family-submit">
          {isSubmitting ? 'Creating…' : `Create family · ${formatCoin(fee)}`}
        </Button>
      </Box>
    </Stack>
  );

  if (embedded) {
    return (
      <Stack spacing={2} data-testid="create-family-form">
        <Typography variant="subtitle1" fontWeight={600}>
          Create a family
        </Typography>
        {body}
      </Stack>
    );
  }

  return (
    <NymCard title="Create a family" data-testid="create-family-form">
      {body}
    </NymCard>
  );
};
