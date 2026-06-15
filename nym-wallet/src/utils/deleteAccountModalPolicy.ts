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
