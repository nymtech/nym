/* eslint-disable @typescript-eslint/naming-convention */
import React, { useState } from 'react';
import { Alert, Box, Button, Stack, TextField } from '@mui/material';
import { NymCard } from '../NymCard';
import { byteLength, sanitizeInput } from './helpers';

export interface EditFamilyFormProps {
  initialName: string;
  initialDescription: string;
  nameLimit: number;
  descriptionLimit: number;
  isSubmitting?: boolean;
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
  const canSubmit = !nothingChanged && !nameTooLong && !descTooLong && name.trim().length > 0 && !isSubmitting;

  const handleSubmit = () => {
    if (!canSubmit) return;
    onSubmit(nameChanged ? sanitizeInput(name) : null, descChanged ? sanitizeInput(description) : null);
  };

  return (
    <NymCard title="Edit family" data-testid="edit-family-form">
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
          minRows={2}
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
    </NymCard>
  );
};
