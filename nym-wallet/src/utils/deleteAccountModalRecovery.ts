export type DeleteAccountModalRecoveryEvent = 'confirm_password_cancel' | 'removal_failed' | 'removal_succeeded';

export type DeleteAccountModalRecoveryOutcome = { action: 'exit_flow' } | { action: 'return_to_warning' };

/** Password-step cancel always exits deletion so the user is never trapped behind a nested sheet. */
export function resolveDeleteModalRecovery(event: DeleteAccountModalRecoveryEvent): DeleteAccountModalRecoveryOutcome {
  if (event === 'removal_failed') {
    return { action: 'return_to_warning' };
  }
  return { action: 'exit_flow' };
}
