export type WalletStoragePaths = {
  walletFile: string;
  storageDirectory: string;
  configDirectory: string;
};

export function formatWalletBackupReminder(paths: WalletStoragePaths): string {
  return [
    'Removing a stored account permanently deletes its saved credentials from your wallet file. This cannot be undone.',
    '',
    'Before you continue, copy your wallet file and settings folder to a safe backup location. If removal fails or corrupts the file, you may lose access to your remaining accounts without a backup.',
    '',
    `Wallet file (required backup):`,
    paths.walletFile,
    '',
    `Wallet folder (recommended backup):`,
    paths.storageDirectory,
    '',
    `Wallet settings folder (recommended backup):`,
    paths.configDirectory,
  ].join('\n');
}
