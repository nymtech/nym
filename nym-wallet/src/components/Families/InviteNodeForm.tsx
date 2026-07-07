/* eslint-disable @typescript-eslint/naming-convention */
import React, { useState } from 'react';
import { Alert, Box, Button, Stack, TextField, Typography } from '@mui/material';
import { NymCard } from '../NymCard';
import { ConfirmationModal } from '../Modals/ConfirmationModal';
import { INVITE_WARNING_MESSAGES, InviteWarning, formatDurationSecs } from './helpers';

export interface InviteNodeFormProps {
  isSubmitting?: boolean;
  isBlocked?: boolean;
  /** Chain-configured invitation TTL, passed through on submit and shown in the confirm dialog. */
  invitationValiditySecs?: number;
  embedded?: boolean;
  /** Set after a failed attempt to surface one of the three warning states. */
  warning?: InviteWarning;
  errorMessage?: string;
  onSubmit: (nodeId: number) => void;
}

const isValidNodeId = (raw: string): boolean => /^\d+$/.test(raw.trim()) && Number(raw.trim()) > 0;

export const InviteNodeForm = ({
  isSubmitting,
  isBlocked,
  invitationValiditySecs,
  embedded,
  warning,
  errorMessage,
  onSubmit,
}: InviteNodeFormProps) => {
  const [nodeId, setNodeId] = useState('');
  const [confirmOpen, setConfirmOpen] = useState(false);

  const trimmed = nodeId.trim();
  const malformed = trimmed.length > 0 && !isValidNodeId(trimmed);
  const canSubmit = isValidNodeId(trimmed) && !isSubmitting && !isBlocked;

  const handleConfirm = () => {
    setConfirmOpen(false);
    if (canSubmit) onSubmit(Number(trimmed));
  };

  const body = (
    <Stack spacing={3}>
      <Typography variant="body2" color="text.secondary">
        Only invite nodes you control and that are already bonded and operational.
      </Typography>

      <TextField
        label="Node ID"
        value={nodeId}
        onChange={(e) => setNodeId(e.target.value)}
        error={malformed}
        helperText={malformed ? 'Enter a valid numeric node ID' : ' '}
        fullWidth
        inputProps={{ 'data-testid': 'invite-node-id', inputMode: 'numeric' }}
      />

      {warning && (
        <Alert severity="warning" data-testid="invite-node-warning">
          {INVITE_WARNING_MESSAGES[warning]}
        </Alert>
      )}

      {errorMessage && (
        <Alert severity="error" data-testid="invite-node-error">
          {errorMessage}
        </Alert>
      )}

      <Box>
        <Button
          variant="contained"
          disabled={!canSubmit}
          onClick={() => setConfirmOpen(true)}
          data-testid="invite-node-submit"
        >
          {isSubmitting ? 'Sending…' : 'Send invite'}
        </Button>
      </Box>
    </Stack>
  );

  const confirmModal = (
    <ConfirmationModal
      open={confirmOpen}
      title="Confirm invite"
      subTitle={
        <Stack spacing={0.75} sx={{ mt: 1.5, textAlign: 'center' }}>
          <Typography variant="body1" color="text.secondary">
            Send a family invitation to node {trimmed}?
          </Typography>
          {invitationValiditySecs !== undefined && (
            <Typography variant="body2" color="text.secondary">
              Invitations expire after {formatDurationSecs(invitationValiditySecs)}.
            </Typography>
          )}
        </Stack>
      }
      confirmButton={
        <Stack direction="row" spacing={2} width="100%">
          <Button fullWidth variant="outlined" onClick={() => setConfirmOpen(false)}>
            Cancel
          </Button>
          <Button fullWidth variant="contained" onClick={handleConfirm} data-testid="invite-node-confirm">
            Confirm & send invite
          </Button>
        </Stack>
      }
      onConfirm={handleConfirm}
      onClose={() => setConfirmOpen(false)}
    />
  );

  if (embedded) {
    return (
      <>
        <Stack spacing={2} data-testid="invite-node-form">
          <Typography variant="subtitle1" fontWeight={600}>
            Invite a node
          </Typography>
          {body}
        </Stack>
        {confirmModal}
      </>
    );
  }

  return (
    <>
      <NymCard title="Invite a node" data-testid="invite-node-form">
        {body}
      </NymCard>
      {confirmModal}
    </>
  );
};
