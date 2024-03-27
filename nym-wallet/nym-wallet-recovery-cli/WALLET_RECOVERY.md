# Wallet CLI Recovery Tool Guide

This guide provides instructions on how to use the Wallet CLI recovery tool to recover your mnemonic phrase using your password, especially useful if you're unable to access your wallet in the usual way.

## Step 1: Install the CLI Tool

1. Change directory `cd /nym-wallet/nym-wallet-recovery-cli` from root Nym repository
2. Have rust installed `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
3. Once the installation is complete run `cargo build --release`
4. The binary should live here: `nym/nym-wallet/nym-wallet-recovery-cli)`

## Step 2: Prepare Your Command

The tool requires specific command-line arguments to specify the password(s) and the file path to your wallet file. The basic structure of the command is as follows:

```plaintext
nym-recovery-cli --password <YOUR_PASSWORD> --file <PATH_TO_YOUR_WALLET_FILE> [OPTIONS]
```

- Replace `<YOUR_PASSWORD>` with your wallet password.
- Replace `<PATH_TO_YOUR_WALLET_FILE>` with the path to your wallet file, where your encrypted mnemonic is stored.

## Step 3: Running the Tool

1. Open your terminal or command prompt.
2. Navigate to the directory where the `nym-recovery-cli` tool is located.
3. Execute the command prepared in Step 2.

Example:

```bash
./nym-recovery-cli --password "mySecurePassword123" --file "/path/to/mywallet.json"
```

To try multiple passwords:

```bash
./nym-recovery-cli --password "myFirstPassword" --password "mySecondPassword" --file "/path/to/mywallet.json"
```

## Step 4: Understanding the Output

The tool will attempt to decrypt the wallet file. If successful, it will print the decrypted content, including your mnemonic phrase.

## Step 5: If You Encounter Issues

- Verify the accuracy of the passwords and file path.
- Ensure there are no typos in the command.
- Make sure the wallet file's path is correctly specified.

## Additional Options

- `--raw`: Skips trying to parse the decrypted content, showing raw output instead.

This guide should help you safely recover your mnemonic phrase. Remember to keep it secure once retrieved.
