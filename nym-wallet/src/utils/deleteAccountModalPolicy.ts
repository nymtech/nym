export function canProceedToAccountRemovalPassword({
  backupConfirmed,
  backupReminder,
  pathsLoadError,
}: {
  backupConfirmed: boolean;
  backupReminder?: string;
  pathsLoadError?: string;
}): boolean {
  if (!backupConfirmed) {
    return false;
  }
  return Boolean(backupReminder) || Boolean(pathsLoadError);
}
