export function mapAccountRemovalError(error: unknown): string {
  let message: string;
  if (error instanceof Error) {
    message = error.message;
  } else if (typeof error === 'string') {
    message = error;
  } else {
    message = String(error);
  }

  if (message.includes('Unexpected mnemonic account') || message.includes('WalletUnexpectedMnemonicAccount')) {
    return 'This wallet uses a legacy single-account format. Add another account first, then try again.';
  }
  if (message.includes('Cannot remove the only stored account') || message.includes('WalletCannotRemoveLastAccount')) {
    return 'You cannot remove your only stored account here. To reset the wallet entirely, sign out and remove your saved wallet file after backing it up.';
  }
  if (
    message.includes('Switch to another account before removing the active account') ||
    message.includes('WalletCannotRemoveActiveAccount')
  ) {
    return 'Switch to another account before removing this one.';
  }

  return message;
}
