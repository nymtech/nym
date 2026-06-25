/* eslint-disable @typescript-eslint/naming-convention */
import React, { useState } from 'react';
import { Alert, Box, Button, Stack, TextField, Typography } from '@mui/material';
import { NymCard } from '../NymCard';
import { byteLength, sanitizeInput } from './helpers';

/** Keeps single-line and multiline family fields visually aligned. */
const familyFieldSx = {
  '& .MuiInputBase-input': {
    py: 1.5,
    boxSizing: 'border-box',
  },
};

export interface EditFamilyFormProps {
  initialName: string;
  initialDescription: string;
  nameLimit: number;
  descriptionLimit: number;
  isSubmitting?: boolean;
  isBlocked?: boolean;
  embedded?: boolean;
  errorMessage?: string;
  /** Sends only changed fields: `string` to set, `null` to leave unchanged. No-op when nothing changed. */
  onSubmit: (updatedName: string | null, updatedDescription: string | null) => void;
}

export const EditFamilyForm = ({
  initialName,
  initialDescription,
  nameLimit,
  descriptionLimit,
  isSubmitting,
  isBlocked,
  embedded,
  errorMessage,
  onSubmit,
}: EditFamilyFormProps) => {
  const [name, setName] = useState(initialName);
  const [description, setDescription] = useState(initialDescription);

  const nameBytes = byteLength(name);
  const descBytes = byteLength(description);
  const nameTooLong = nameBytes > nameLimit;
  const descTooLong = descBytes > descriptionLimit;

  const nameChanged = name !== initialName;
  const descChanged = description !== initialDescription;
  const nothingChanged = !nameChanged && !descChanged;
  const canSubmit =
    !nothingChanged && !nameTooLong && !descTooLong && name.trim().length > 0 && !isSubmitting && !isBlocked;

  const handleSubmit = () => {
    if (!canSubmit) return;
    onSubmit(nameChanged ? sanitizeInput(name) : null, descChanged ? sanitizeInput(description) : null);
  };

  const body = (
    <Stack spacing={3}>
      <TextField
        label="Family name"
        value={name}
        onChange={(e) => setName(e.target.value)}
        error={nameTooLong}
        helperText={
          nameTooLong ? `Name is ${nameBytes}/${nameLimit} bytes — too long` : `${nameBytes}/${nameLimit} bytes`
        }
        fullWidth
        sx={familyFieldSx}
        inputProps={{ 'data-testid': 'edit-family-name' }}
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
        minRows={1}
        maxRows={6}
        sx={familyFieldSx}
        inputProps={{ 'data-testid': 'edit-family-description' }}
      />

      {errorMessage && (
        <Alert severity="error" data-testid="edit-family-error">
          {errorMessage}
        </Alert>
      )}

      <Box>
        <Button variant="contained" disabled={!canSubmit} onClick={handleSubmit} data-testid="edit-family-submit">
          {isSubmitting ? 'Saving…' : 'Save changes'}
        </Button>
      </Box>
    </Stack>
  );

  if (embedded) {
    return (
      <Stack spacing={2} data-testid="edit-family-form">
        <Typography variant="subtitle1" fontWeight={600}>
          Edit family
        </Typography>
        {body}
      </Stack>
    );
  }

  return (
    <NymCard title="Edit family" data-testid="edit-family-form">
      {body}
    </NymCard>
  );
};
