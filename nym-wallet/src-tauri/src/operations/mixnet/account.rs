use crate::config::{Config, CUSTOM_SIMULATED_GAS_MULTIPLIER};
use crate::error::BackendError;
use crate::network_config;
use crate::state::{WalletAccountIds, WalletState};
use crate::wallet_storage::{self, UserPassword, DEFAULT_LOGIN_ID};
use bip39::rand::seq::SliceRandom;
use bip39::{rand, Language, Mnemonic};
use cosmrs::bip32::DerivationPath;
use itertools::Itertools;
use nym_config::defaults::{NymNetworkDetails, COSMOS_DERIVATION_PATH};
use nym_types::account::{Account, AccountEntry, Balance};
use nym_validator_client::nyxd::CosmWasmClient;
use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;
use nym_validator_client::signing::AccountData;
use nym_validator_client::DirectSigningHttpRpcValidatorClient;
use nym_wallet_types::network::Network as WalletNetwork;
use std::collections::HashMap;
use strum::IntoEnumIterator;
use url::Url;

#[tauri::command]
pub async fn connect_with_mnemonic(
    mnemonic: Mnemonic,
    state: tauri::State<'_, WalletState>,
) -> Result<Account, BackendError> {
    _connect_with_mnemonic(mnemonic, state).await
}

#[tauri::command]
pub async fn get_balance(state: tauri::State<'_, WalletState>) -> Result<Balance, BackendError> {
    let guard = state.read().await;
    let client = guard.current_client()?;
    let address = client.nyxd.address();
    let network = guard.current_network();
    let base_mix_denom = network.base_mix_denom();

    match client
        .nyxd
        .get_balance(&address, base_mix_denom.to_string())
        .await?
    {
        Some(coin) => {
            let amount = guard.attempt_convert_to_display_dec_coin(coin)?;
            Ok(Balance::new(amount))
        }
        None => Err(BackendError::NoBalance(address.to_string())),
    }
}

#[tauri::command]
pub fn create_new_mnemonic() -> Mnemonic {
    random_mnemonic()
}

#[tauri::command]
pub fn validate_mnemonic(_mnemonic: Mnemonic) -> bool {
    true
}

#[tauri::command]
pub async fn switch_network(
    state: tauri::State<'_, WalletState>,
    network: WalletNetwork,
) -> Result<Account, BackendError> {
    let account = {
        let r_state = state.read().await;
        let client = r_state.client(network)?;
        let denom = network.mix_denom();

        Account::new(client.nyxd.address().to_string(), denom)
    };

    let mut w_state = state.write().await;
    w_state.set_network(network);

    Ok(account)
}

#[tauri::command]
pub async fn logout(state: tauri::State<'_, WalletState>) -> Result<(), BackendError> {
    state.write().await.logout();
    Ok(())
}

fn random_mnemonic() -> Mnemonic {
    let mut rng = rand::thread_rng();
    Mnemonic::generate_in_with(&mut rng, Language::English, 24).unwrap()
}

async fn _connect_with_mnemonic(
    mnemonic: Mnemonic,
    state: tauri::State<'_, WalletState>,
) -> Result<Account, BackendError> {
    {
        let mut w_state = state.write().await;
        w_state.load_config_files();
    }

    network_config::update_nyxd_urls(state.clone()).await?;

    let config = {
        let state = state.read().await;

        // Take the oppertunity to list all the known validators while we have the state.
        for network in WalletNetwork::iter() {
            // fern really wants us to not evaluate this inside the debug macro argument
            let f = format!(
                "{}",
                state.get_config_validator_entries(network).format(",\n")
            );
            log::debug!("List of validators for {network}: [\n{}\n]", f,);
        }

        state.config().clone()
    };

    // Get all the urls needed for the connection test
    let (untested_nyxd_urls, untested_api_urls) = {
        let state = state.read().await;
        (state.get_all_nyxd_urls(), state.get_all_api_urls())
    };

    let (nyxd_urls, api_urls) = run_connection_test(
        untested_nyxd_urls.clone(),
        untested_api_urls.clone(),
        &config,
    )
    .await;

    let default_nyxd_urls: HashMap<WalletNetwork, Url> = untested_nyxd_urls
        .iter()
        .map(|(network, urls)| (*network, urls.iter().next().unwrap().clone()))
        .collect();
    let default_api_urls: HashMap<WalletNetwork, Url> = untested_api_urls
        .iter()
        .map(|(network, urls)| (*network, urls.iter().next().unwrap().clone()))
        .collect();

    let nyxd_urls = pick_good_nyxd_urls(&default_nyxd_urls, &nyxd_urls).await?;
    let api_urls = pick_good_api_urls(&default_api_urls, &api_urls).await?;

    {
        let mut w_state = state.write().await;
        // Save the checked nyxd URLs
        w_state.set_default_nyxd_urls(&nyxd_urls);
    }

    // Create clients for all networks
    let clients = create_clients(&nyxd_urls, &api_urls, &config, &mnemonic)?;

    // Set the default account
    let default_network = WalletNetwork::MAINNET;
    let client_for_default_network = clients
        .iter()
        .find(|(network, _)| *network == default_network);
    let account_for_default_network = match client_for_default_network {
        Some((_, client)) => Ok(Account::new(
            client.nyxd.address().to_string(),
            default_network.mix_denom(),
        )),
        None => Err(BackendError::NetworkNotSupported),
    };

    // Register all the clients
    {
        let mut w_state = state.write().await;
        w_state.logout();
    }
    for (network, client) in clients {
        let mut w_state = state.write().await;
        w_state.add_client(network, client);
        w_state.register_default_denoms(network);
    }

    account_for_default_network
}

async fn run_connection_test(
    untested_nyxd_urls: HashMap<WalletNetwork, Vec<Url>>,
    untested_api_urls: HashMap<WalletNetwork, Vec<Url>>,
    config: &Config,
) -> (
    HashMap<NymNetworkDetails, Vec<(Url, bool)>>,
    HashMap<NymNetworkDetails, Vec<(Url, bool)>>,
) {
    let mixnet_contract_address = WalletNetwork::iter()
        .map(|network| (network.into(), config.get_mixnet_contract_address(network)))
        .collect::<HashMap<_, _>>();

    let untested_nyxd_urls = untested_nyxd_urls
        .into_iter()
        .flat_map(|(net, urls)| urls.into_iter().map(move |url| (net.into(), url)));

    let untested_api_urls = untested_api_urls
        .into_iter()
        .flat_map(|(net, urls)| urls.into_iter().map(move |url| (net.into(), url)));

    nym_validator_client::connection_tester::run_validator_connection_test(
        untested_nyxd_urls,
        untested_api_urls,
        mixnet_contract_address,
    )
    .await
}

async fn pick_good_nyxd_urls(
    default_nyxd_urls: &HashMap<WalletNetwork, Url>,
    nyxd_urls: &HashMap<NymNetworkDetails, Vec<(Url, bool)>>,
) -> Result<HashMap<WalletNetwork, Url>, BackendError> {
    let nyxd_urls: HashMap<WalletNetwork, Url> = WalletNetwork::iter()
        .map(|network| {
            let default_nyxd_url = default_nyxd_urls
                .get(&network)
                .expect("Expected at least one nyxd_url");
            let url = select_random_responding_url(nyxd_urls, network).unwrap_or_else(|| {
                log::warn!(
                    "No successful nyxd_urls for {network}: using default: {default_nyxd_url}"
                );
                default_nyxd_url.clone()
            });
            log::info!("Set default nyxd_url for {network}: {url}");
            (network, url)
        })
        .collect();

    Ok(nyxd_urls)
}

async fn pick_good_api_urls(
    default_api_urls: &HashMap<WalletNetwork, Url>,
    api_urls: &HashMap<NymNetworkDetails, Vec<(Url, bool)>>,
) -> Result<HashMap<WalletNetwork, Url>, BackendError> {
    let api_urls: HashMap<WalletNetwork, Url> = WalletNetwork::iter()
        .map(|network| {
            let default_api_url = default_api_urls
                .get(&network)
                .expect("Expected at least one api_url");
            let url = select_first_responding_url(api_urls, network).unwrap_or_else(|| {
                log::warn!("No passing api_urls for {network}: using default: {default_api_url}");
                default_api_url.clone()
            });
            log::info!("Set default api_url for {network}: {url}");
            (network, url)
        })
        .collect();

    Ok(api_urls)
}

fn create_clients(
    default_nyxd_urls: &HashMap<WalletNetwork, Url>,
    default_api_urls: &HashMap<WalletNetwork, Url>,
    config: &Config,
    mnemonic: &Mnemonic,
) -> Result<Vec<(WalletNetwork, DirectSigningHttpRpcValidatorClient)>, BackendError> {
    let mut clients = Vec::new();
    for network in WalletNetwork::iter() {
        let nyxd_url = if let Some(url) = config.get_selected_validator_nyxd_url(network) {
            log::debug!("Using selected nyxd_url for {network}: {url}");
            url.clone()
        } else {
            let url = default_nyxd_urls
                .get(&network)
                .expect("Expected at least one nyxd_url");
            log::debug!("Using default nyxd_url for {network}: {url}");
            url.to_owned()
        };

        let api_url = if let Some(url) = config.get_selected_nym_api_url(&network) {
            log::debug!("Using selected api_url for {network}: {url}");
            url.clone()
        } else {
            let url = default_api_urls
                .get(&network)
                .expect("Expected at least one api url");
            log::debug!("Using default api_url for {network}: {url}");
            url.to_owned()
        };

        log::info!("Connecting to: nyxd_url: {nyxd_url} for {network}");
        log::info!("Connecting to: api_url: {api_url} for {network}");

        let network_details = NymNetworkDetails::from(network)
            .clone()
            .with_mixnet_contract(Some(config.get_mixnet_contract_address(network).as_ref()))
            .with_vesting_contract(Some(config.get_vesting_contract_address(network).as_ref()));

        let config = nym_validator_client::Config::try_from_nym_network_details(&network_details)?
            .with_urls(nyxd_url, api_url)
            .with_simulated_gas_multiplier(CUSTOM_SIMULATED_GAS_MULTIPLIER);

        let client = nym_validator_client::Client::new_signing(config, mnemonic.clone())?;
        clients.push((network, client));
    }
    Ok(clients)
}

fn select_random_responding_url(
    urls: &HashMap<NymNetworkDetails, Vec<(Url, bool)>>,
    network: WalletNetwork,
) -> Option<Url> {
    urls.get(&network.into()).and_then(|urls| {
        let urls: Vec<_> = urls
            .iter()
            .filter_map(|(url, result)| if *result { Some(url.clone()) } else { None })
            .collect();
        urls.choose(&mut rand::thread_rng()).cloned()
    })
}

fn select_first_responding_url(
    urls: &HashMap<NymNetworkDetails, Vec<(Url, bool)>>,
    network: WalletNetwork,
    //config: &Config,
) -> Option<Url> {
    urls.get(&network.into()).and_then(|urls| {
        urls.iter()
            .find_map(|(url, result)| if *result { Some(url.clone()) } else { None })
    })
}

#[tauri::command]
pub fn does_password_file_exist() -> Result<bool, BackendError> {
    log::info!("Checking wallet file");
    let file = wallet_storage::wallet_login_filepath()?;
    if file.exists() {
        log::info!("Exists: {}", file.to_string_lossy());
        Ok(true)
    } else {
        log::info!("Does not exist: {}", file.to_string_lossy());
        Ok(false)
    }
}

#[tauri::command]
pub fn create_password(mnemonic: Mnemonic, password: UserPassword) -> Result<(), BackendError> {
    if does_password_file_exist()? {
        return Err(BackendError::WalletFileAlreadyExists);
    }
    log::info!("Creating password");

    let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
    // Currently we only support a single, default, login id in the wallet
    let login_id = wallet_storage::LoginId::new(DEFAULT_LOGIN_ID.to_string());
    wallet_storage::store_login_with_multiple_accounts(mnemonic, hd_path, login_id, &password)
}

#[tauri::command]
pub fn update_password(
    current_password: UserPassword,
    new_password: UserPassword,
) -> Result<(), BackendError> {
    log::info!("Updating password");

    wallet_storage::update_encrypted_logins(&current_password, &new_password)
}

#[tauri::command]
pub async fn sign_in_with_password(
    password: UserPassword,
    state: tauri::State<'_, WalletState>,
) -> Result<Account, BackendError> {
    log::info!("Signing in with password");

    // Currently we only support a single, default, id in the wallet
    let login_id = wallet_storage::LoginId::new(DEFAULT_LOGIN_ID.to_string());
    let stored_login = wallet_storage::load_existing_login(&login_id, &password)?;

    let mnemonic = extract_first_mnemonic(&stored_login)?;
    let first_login_id_when_converting = login_id.into();
    set_state_with_all_accounts(stored_login, first_login_id_when_converting, state.clone())
        .await?;

    _connect_with_mnemonic(mnemonic, state).await
}

fn extract_first_mnemonic(
    stored_login: &wallet_storage::StoredLogin,
) -> Result<Mnemonic, BackendError> {
    let mnemonic = match stored_login {
        wallet_storage::StoredLogin::Mnemonic(ref account) => account.mnemonic().clone(),
        wallet_storage::StoredLogin::Multiple(ref accounts) => {
            // Login using the first account in the list
            accounts
                .get_accounts()
                .next()
                .ok_or(BackendError::WalletNoSuchAccountIdInWalletLogin)?
                .mnemonic()
                .clone()
        }
    };

    Ok(mnemonic)
}

#[tauri::command]
pub async fn sign_in_with_password_and_account_id(
    account_id: &str,
    password: UserPassword,
    state: tauri::State<'_, WalletState>,
) -> Result<Account, BackendError> {
    log::info!("Signing in with password");

    // Currently we only support a single, default, id in the wallet
    let login_id = wallet_storage::LoginId::new(DEFAULT_LOGIN_ID.to_string());
    let account_id = wallet_storage::AccountId::new(account_id.to_string());
    let stored_login = wallet_storage::load_existing_login(&login_id, &password)?;

    let mnemonic = extract_mnemonic(&stored_login, &account_id)?;
    let first_login_id_when_converting = login_id.into();
    set_state_with_all_accounts(stored_login, first_login_id_when_converting, state.clone())
        .await?;

    _connect_with_mnemonic(mnemonic, state).await
}

fn extract_mnemonic(
    stored_login: &wallet_storage::StoredLogin,
    account_id: &wallet_storage::AccountId,
) -> Result<Mnemonic, BackendError> {
    let mnemonic = match stored_login {
        wallet_storage::StoredLogin::Mnemonic(_) => {
            return Err(BackendError::WalletNoSuchAccountIdInWalletLogin);
        }
        wallet_storage::StoredLogin::Multiple(ref accounts) => accounts
            .get_account(account_id)
            .ok_or(BackendError::WalletNoSuchAccountIdInWalletLogin)?
            .mnemonic()
            .clone(),
    };
    Ok(mnemonic)
}

#[tauri::command]
pub fn remove_password() -> Result<(), BackendError> {
    log::info!("Removing password");
    let login_id = wallet_storage::LoginId::new(DEFAULT_LOGIN_ID.to_string());
    wallet_storage::remove_login(&login_id)
}

#[tauri::command]
pub fn archive_wallet_file() -> Result<(), BackendError> {
    wallet_storage::archive_wallet_file()
}

#[tauri::command]
pub async fn add_account_for_password(
    mnemonic: Mnemonic,
    password: UserPassword,
    account_id: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<AccountEntry, BackendError> {
    log::info!("Adding account for the current password: {account_id}");
    let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
    // Currently we only support a single, default, login id in the wallet
    let login_id = wallet_storage::LoginId::new(DEFAULT_LOGIN_ID.to_string());
    let account_id = wallet_storage::AccountId::new(account_id.to_string());

    wallet_storage::append_account_to_login(
        mnemonic.clone(),
        hd_path,
        login_id.clone(),
        account_id.clone(),
        &password,
    )?;

    let address = {
        let state = state.read().await;
        let network: NymNetworkDetails = state.current_network().into();
        derive_address(mnemonic, &network.chain_details.bech32_account_prefix)?.to_string()
    };

    // Re-read all the acccounts from the  wallet to reset the state, rather than updating it
    // incrementally
    let stored_login = wallet_storage::load_existing_login(&login_id, &password)?;
    // NOTE: since we are appending, this id shouldn't be needed, but setting the state is supposed
    // to be a general function
    let first_id_when_converting = login_id.into();
    set_state_with_all_accounts(stored_login, first_id_when_converting, state).await?;

    Ok(AccountEntry {
        id: account_id.to_string(),
        address,
    })
}

// Set the tauri state with all the accounts in the wallet.
// NOTE: the first `AccoundId` when converting is the `LoginId` for the entry that was loaded.
async fn set_state_with_all_accounts(
    stored_login: wallet_storage::StoredLogin,
    first_id_when_converting: wallet_storage::AccountId,
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    log::trace!("Set state with accounts:");
    let stored = stored_login.unwrap_into_multiple_accounts(first_id_when_converting);
    let all_accounts = stored.inner();

    for account in all_accounts {
        log::trace!("account: {:?}", account.id());
    }

    let all_account_ids: Vec<WalletAccountIds> = all_accounts
        .iter()
        .map(|account| {
            let mnemonic = account.mnemonic();
            let addresses: HashMap<WalletNetwork, cosmrs::AccountId> = WalletNetwork::iter()
                .map(|network| {
                    let config_network: NymNetworkDetails = network.into();
                    (
                        network,
                        derive_address(
                            mnemonic.clone(),
                            &config_network.chain_details.bech32_account_prefix,
                        )
                        .unwrap(),
                    )
                })
                .collect();
            WalletAccountIds {
                id: account.id().clone(),
                addresses,
            }
        })
        .collect();

    let mut w_state = state.write().await;
    w_state.set_all_accounts(all_account_ids);
    Ok(())
}

#[tauri::command]
pub async fn remove_account_for_password(
    password: UserPassword,
    account_id: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    log::info!("Removing account: {account_id}");
    // Currently we only support a single, default, id in the wallet
    let login_id = wallet_storage::LoginId::new(DEFAULT_LOGIN_ID.to_string());
    let account_id = wallet_storage::AccountId::new(account_id.to_string());
    wallet_storage::remove_account_from_login(&login_id, &account_id, &password)?;

    // Load to reset the internal state
    let stored_login = wallet_storage::load_existing_login(&login_id, &password)?;
    // NOTE: Since we removed from a multi-account login, this id shouldn't be needed, but setting
    // the state is supposed to be a general function
    let first_account_id_when_converting = login_id.into();
    set_state_with_all_accounts(stored_login, first_account_id_when_converting, state).await
}

#[tauri::command]
pub async fn rename_account_for_password(
    password: UserPassword,
    account_id: &str,
    new_account_id: &str,
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    log::info!("Renaming account: {account_id} to {new_account_id}");
    // Currently we only support a single, default, id in the wallet
    let login_id = wallet_storage::LoginId::new(DEFAULT_LOGIN_ID.to_string());
    let account_id = wallet_storage::AccountId::new(account_id.to_string());
    let new_account_id = wallet_storage::AccountId::new(new_account_id.to_string());
    wallet_storage::rename_account_in_login(&login_id, &account_id, &new_account_id, &password)?;

    // Load from storage to reset the internal tuari state
    let stored_login = wallet_storage::load_existing_login(&login_id, &password)?;
    // NOTE: Since we removed from a multi-account login, this id shouldn't be needed, but setting
    // the state is supposed to be a general function
    let first_account_id_when_converting = login_id.into();
    set_state_with_all_accounts(stored_login, first_account_id_when_converting, state).await
}

fn derive_address(
    mnemonic: bip39::Mnemonic,
    prefix: &str,
) -> Result<cosmrs::AccountId, BackendError> {
    // note: the ephemeral wallet will zeroize the mnemonic on drop
    DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic)
        .try_derive_accounts()?
        .first()
        .map(AccountData::address)
        .cloned()
        .ok_or(BackendError::FailedToDeriveAddress)
}

#[tauri::command]
pub async fn list_accounts(
    state: tauri::State<'_, WalletState>,
) -> Result<Vec<AccountEntry>, BackendError> {
    log::trace!("Listing accounts");
    let state = state.read().await;
    let network = state.current_network();

    let all_accounts = state
        .get_all_accounts()
        .map(|account| AccountEntry {
            id: account.id.to_string(),
            address: account.addresses[&network].to_string(),
        })
        .map(|account| {
            log::trace!("{:?}", account);
            account
        })
        .collect();

    Ok(all_accounts)
}

#[tauri::command]
pub fn show_mnemonic_for_account_in_password(
    account_id: String,
    password: UserPassword,
) -> Result<Mnemonic, BackendError> {
    log::info!("Getting mnemonic for: {account_id}");
    let login_id = wallet_storage::LoginId::new(DEFAULT_LOGIN_ID.to_string());
    let account_id = wallet_storage::AccountId::new(account_id);
    let mnemonic = _show_mnemonic_for_account_in_password(&login_id, &account_id, &password)?;
    Ok(mnemonic)
}

fn _show_mnemonic_for_account_in_password(
    login_id: &wallet_storage::LoginId,
    account_id: &wallet_storage::AccountId,
    password: &wallet_storage::UserPassword,
) -> Result<Mnemonic, BackendError> {
    let stored_account = wallet_storage::load_existing_login(login_id, password)?;
    let mnemonic = match stored_account {
        wallet_storage::StoredLogin::Mnemonic(ref account) => account.mnemonic().clone(),
        wallet_storage::StoredLogin::Multiple(ref accounts) => accounts
            .get_account(account_id)
            .ok_or(BackendError::WalletNoSuchAccountIdInWalletLogin)?
            .mnemonic()
            .clone(),
    };
    Ok(mnemonic)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::str::FromStr;

    use crate::wallet_storage::account_data::{MnemonicAccount, WalletAccount};

    use super::*;

    // This decrypts a stored wallet file using the same procedure as when signing in. Most tests
    // related to the encrypted wallet storage is in `wallet_storage`.
    #[test]
    fn decrypt_stored_wallet_for_sign_in() {
        const SAVED_WALLET: &str = "src/wallet_storage/test-data/saved-wallet.json";
        let wallet_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SAVED_WALLET);
        let login_id = wallet_storage::LoginId::new("first".to_string());
        let account_id = wallet_storage::AccountId::new("first".to_string());
        let password = wallet_storage::UserPassword::new("password".to_string());
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();

        let stored_login =
            wallet_storage::load_existing_login_at_file(&wallet_file, &login_id, &password)
                .unwrap();
        let mnemonic = extract_first_mnemonic(&stored_login).unwrap();

        let expected_mnemonic = bip39::Mnemonic::from_str("country mean universe text phone begin deputy reject result good cram illness common cluster proud swamp digital patrol spread bar face december base kick").unwrap();
        assert_eq!(mnemonic, expected_mnemonic);

        let all_accounts: Vec<_> = stored_login
            .unwrap_into_multiple_accounts(account_id.clone())
            .inner()
            .to_vec();

        assert_eq!(
            all_accounts,
            vec![WalletAccount::new(
                account_id,
                MnemonicAccount::new(expected_mnemonic, hd_path),
            )]
        );
    }

    // This decryptes a stored wallet file using the same procedure as when signing in. Most tests
    // related to the encryped wallet storage is in `wallet_storage`.
    #[test]
    fn decrypt_stored_wallet_multiple_for_sign_in() {
        const SAVED_WALLET: &str = "src/wallet_storage/test-data/saved-wallet-1.0.5.json";
        let wallet_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SAVED_WALLET);
        let login_id = wallet_storage::LoginId::new(DEFAULT_LOGIN_ID.to_string());
        let account_id = wallet_storage::AccountId::new("default".to_string());
        let password = wallet_storage::UserPassword::new("password11!".to_string());
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();

        let stored_login =
            wallet_storage::load_existing_login_at_file(&wallet_file, &login_id, &password)
                .unwrap();
        let mnemonic = extract_first_mnemonic(&stored_login).unwrap();

        let expected_mnemonic = bip39::Mnemonic::from_str("arrow capable abstract industry elevator nominee december piece hotel feed lounge web faint sword veteran bundle hour page actual laptop horror gold test warrior").unwrap();
        assert_eq!(mnemonic, expected_mnemonic);

        let all_accounts: Vec<_> = stored_login
            .unwrap_into_multiple_accounts(account_id)
            .inner()
            .to_vec();

        let expected_mn2 = bip39::Mnemonic::from_str("border hurt skull lunar goddess second danger game dismiss exhaust oven thumb dog drama onion false orchard spice tent next predict invite cherry green").unwrap();
        let expected_mn3 = bip39::Mnemonic::from_str("gentle crowd rule snap girl urge flat jump winner cluster night sand museum stock grunt quick tree acquire traffic major awake tag rack peasant").unwrap();
        let expected_mn4 = bip39::Mnemonic::from_str("debris blue skin annual inhale text border rigid spatial lesson coconut yard horn crystal control survey version vote hawk neck frame arrive oblige width").unwrap();

        assert_eq!(
            all_accounts,
            vec![
                WalletAccount::new(
                    "default".into(),
                    MnemonicAccount::new(expected_mnemonic, hd_path.clone())
                ),
                WalletAccount::new(
                    "account2".into(),
                    MnemonicAccount::new(expected_mn2, hd_path.clone()),
                ),
                WalletAccount::new(
                    "foobar".into(),
                    MnemonicAccount::new(expected_mn3, hd_path.clone()),
                ),
                WalletAccount::new("42".into(), MnemonicAccount::new(expected_mn4, hd_path)),
            ]
        );
    }
}
