export function canProceedToAccountRemovalPassword({
  backupConfirmed,
  backupReminder,
  pathsLoadError,
}: {
  backupConfirmed: boolean;
  backupReminder?: string;
  pathsLoadError?: string;
}): boolean {
  if (!backupConfirmed || pathsLoadError) {
    return false;
  }
  return Boolean(backupReminder);
}

export function formatWalletPathsLoadBlockedMessage(pathsLoadError: string): string {
  return `Could not load wallet file locations: ${pathsLoadError}. Account removal is disabled until these paths can be loaded.`;
}
