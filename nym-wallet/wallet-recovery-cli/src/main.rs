//! Wallet CLI recovery tool
//!
//! Can decrypted wallet files saved by nym-wallet.

// Implementation notes: this utility deliberately doesn't reuse any of the structs in the wallet
// code. Furthermore we don't to strongly typed json parsing, all with the intention of being a
// little more flexible when interpreting the results.

use std::fs::File;

use aes_gcm::{aead::Aead, Aes256Gcm, Key, NewAead, Nonce};
use anyhow::{anyhow, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use clap::Parser;
use serde_json::Value;

// Mostly defaults
const MEMORY_COST: u32 = 16 * 1024; // 4096 is default
const ITERATIONS: u32 = 3; // This appears to be the default
const PARALLELISM: u32 = 1; // 1 thread. Default
const OUTPUT_LENGTH: usize = 32; // Default

#[derive(Debug)]
enum DecryptedAccount {
    Mnemonic((String, String)),
    Multiple(Vec<(String, String, String)>),
}

/// Simple utility to decrypt wallet file used by `nym-wallet` to store encrypted mnemonics.
#[derive(Parser, Debug)]
#[clap(author, about)]
struct Args {
    /// Password used to attempt to decrypt the logins found in the file. The option can be
    /// provided multiple times for files that require multiple passwords.
    #[clap(short, long, min_values(1), required = true)]
    password: Vec<String>,

    /// Path to the wallet file that will be decrypted.
    #[clap(short, long)]
    file: String,
}

fn main() -> Result<()> {
    setup_logging();
    let args = Args::parse();
    let file = File::open(args.file)?;
    decrypt_file(file, &args.password)
}

fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    } else {
        // default to 'Info'
        log_builder.filter(None, log::LevelFilter::Info);
    }

    log_builder.init();
}

fn decrypt_file(file: File, passwords: &[String]) -> Result<()> {
    let json_file: Value = serde_json::from_reader(file)?;

    // The logins are stored under the more generic name "accounts"
    let logins = json_file["accounts"]
        .as_array()
        .ok_or_else(|| anyhow!("No accounts found in file!"))?;

    println!("The file contains the logins:");
    for login in logins {
        let id = &login["id"];
        println!(" - id: {id}");
    }

    println!("We have {} password(s) to try", passwords.len());
    let mut successes = 0;
    for login in logins {
        match decrypt_login(login, passwords) {
            Ok(is_success) if is_success => successes += 1,
            Ok(_) => println!("None of the provided passwords succeeded"),
            Err(err) => println!("Failed: {}", err),
        }
    }

    println!(
        "\nManaged to decrypt {} out of {} found logins, using the {} provided password(s)",
        successes,
        logins.len(),
        passwords.len(),
    );
    if successes != logins.len() {
        return Err(anyhow!("Failed to decrypt all logins"));
    }
    Ok(())
}

fn decrypt_login(login: &Value, passwords: &[String]) -> Result<bool> {
    let id = &login["id"];
    println!("\nAttempting to parse login entry: {id}");

    let (ciphertext, iv, salt) = get_login_entry(login)?;
    let (ciphertext, iv, salt) = base64_decode(ciphertext, iv, salt)?;

    for (i, password) in passwords.iter().enumerate() {
        print!("Trying to decrypt with password {i}:");
        match decrypt_password(password, &ciphertext, &iv, &salt).and_then(|r| parse_results(&r)) {
            Ok(DecryptedAccount::Mnemonic((mnemonic, hd_path))) => {
                println!(" success!");
                println!("  mnemonic: {mnemonic}");
                println!("  hd_path: {hd_path}");
                return Ok(true);
            }
            Ok(DecryptedAccount::Multiple(accounts)) => {
                println!(" success!");
                println!();
                for (id, mnemonic, hd_path) in accounts {
                    println!("  account_id: {id}");
                    println!("  mnemonic: {mnemonic}");
                    println!("  hd_path: {hd_path}");
                    println!();
                }
                return Ok(true);
            }
            Err(err) => println!(" failed\n{}", err),
        }
    }

    Ok(false)
}

fn get_login_entry(login: &Value) -> Result<(&str, &str, &str)> {
    let account = &login["account"]
        .as_object()
        .ok_or_else(|| anyhow!("No account entry in json"))?;
    let ciphertext = account["ciphertext"]
        .as_str()
        .ok_or_else(|| anyhow!("No ciphertext entry"))?;
    let iv = account["iv"]
        .as_str()
        .ok_or_else(|| anyhow!("No IV entry"))?;
    let salt = account["salt"]
        .as_str()
        .ok_or_else(|| anyhow!("No salt entry"))?;
    Ok((ciphertext, iv, salt))
}

fn base64_decode(ciphertext: &str, iv: &str, salt: &str) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let ciphertext = base64::decode(&ciphertext)
        .map_err(|err| anyhow!("Unable to base64 decode ciphertext: {}", err))?;
    let iv = base64::decode(iv).map_err(|err| anyhow!("Unable to base64 decode iv: {}", err))?;
    let salt =
        base64::decode(salt).map_err(|err| anyhow!("Unable to base64 decode salt: {}", err))?;
    Ok((ciphertext, iv, salt))
}

fn decrypt_password(password: &str, ciphertext: &[u8], iv: &[u8], salt: &[u8]) -> Result<Value> {
    let params = Params::new(MEMORY_COST, ITERATIONS, PARALLELISM, Some(OUTPUT_LENGTH)).unwrap();

    // Argon2id is default, V0x13 is default
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Key::default();
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|err| anyhow!("Unable to hash password: {}", err))?;

    // Create the Cipher
    let cipher = Aes256Gcm::new(&key);

    // Decrypt using the nonce, which we get from the IV
    let nonce = Nonce::from_slice(iv);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| anyhow!("Unable to decrypt"))?;
    let plaintext = String::from_utf8(plaintext)?;

    let json_data: Value = serde_json::from_str(&plaintext)?;
    Ok(json_data)
}

fn parse_results(json_data: &Value) -> Result<DecryptedAccount> {
    try_parse_mnemonic_account(json_data).or_else(|_| try_parse_multiple_account(json_data))
}

fn try_parse_mnemonic_account(json_data: &Value) -> Result<DecryptedAccount> {
    let mnemonic = json_data["mnemonic"]
        .as_str()
        .ok_or_else(|| anyhow!("No mnemonic entry after decrypting"))?;
    let hd_path = json_data["hd_path"]
        .as_str()
        .ok_or_else(|| anyhow!("No hd_path entry after decrypting"))?;
    Ok(DecryptedAccount::Mnemonic((
        mnemonic.to_string(),
        hd_path.to_string(),
    )))
}

fn try_parse_multiple_account(json_data: &Value) -> Result<DecryptedAccount> {
    let accounts = json_data["accounts"]
        .as_array()
        .ok_or_else(|| anyhow!("No accounts decrypting"))?;

    let mut found_accounts = Vec::new();

    for account in accounts {
        let id = account["id"].to_string();
        let account = &account["account"];
        match try_parse_mnemonic_account(account) {
            Ok(DecryptedAccount::Mnemonic((mnemonic, hd_path))) => {
                found_accounts.push((id, mnemonic, hd_path));
            }
            Ok(DecryptedAccount::Multiple(_)) => {
                println!("Error: double nested accounts not supported")
            }
            Err(err) => {
                println!("Error: {err}");
            }
        };
    }

    Ok(DecryptedAccount::Multiple(found_accounts))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    fn try_decrypt(file: &str, passwords: Vec<&str>) -> bool {
        let wallet_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file);
        let file = File::open(wallet_file).unwrap();
        let passwords: Vec<_> = passwords.into_iter().map(ToString::to_string).collect();
        decrypt_file(file, &passwords).is_ok()
    }

    #[test]
    fn decrypt_saved_file() {
        assert!(try_decrypt(
            "../src-tauri/src/wallet_storage/test-data/saved-wallet.json",
            vec!["password"],
        ));
    }

    #[test]
    fn decrypt_saved_file_1_0_4() {
        assert!(try_decrypt(
            "../src-tauri/src/wallet_storage/test-data/saved-wallet-1.0.4.json",
            vec!["password11!"],
        ));
    }

    #[test]
    fn decrypt_saved_file_1_0_5() {
        assert!(try_decrypt(
            "../src-tauri/src/wallet_storage/test-data/saved-wallet-1.0.5.json",
            vec!["password11!"],
        ));
    }
}
