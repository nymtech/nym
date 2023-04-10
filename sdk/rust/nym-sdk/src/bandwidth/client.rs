use crate::error::{Error, Result};
use nym_bandwidth_controller::acquire::state::State;
use nym_credential_storage::ephemeral_storage::EphemeralStorage;
use nym_credentials::coconut::bandwidth::BandwidthVoucher;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::{Coin, SigningNyxdClient};
use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;
use nym_validator_client::{Client, Config};

/// The serialized version of the yet untransformed bandwidth voucher. It can be used to complete
/// the acquirement process of a bandwidth credential.
/// Its serialized nature makes it easy to store and load it to e.g. disk.
pub type VoucherBlob = Vec<u8>;

/// Represents a client that can be used to acquire bandwidth. You typically create one when you
/// want to connect to the mixnet using paid coconut bandwidth credentials.
/// The way to create this client is by calling
/// [`crate::mixnet::DisconnectedMixnetClient::create_bandwidth_client`] on the associated mixnet
/// client.
pub struct BandwidthAcquireClient {
    network_details: NymNetworkDetails,
    client: Client<SigningNyxdClient<DirectSecp256k1HdWallet>>,
    storage: EphemeralStorage,
}

impl BandwidthAcquireClient {
    pub(crate) fn new(
        network_details: NymNetworkDetails,
        mnemonic: String,
        storage: EphemeralStorage,
    ) -> Result<Self> {
        let config = Config::try_from_nym_network_details(&network_details)?;
        let client = nym_validator_client::Client::new_signing(config, mnemonic.parse()?)?;
        Ok(Self {
            network_details,
            client,
            storage,
        })
    }

    /// Buy a credential worth amount utokens. If [`Error::UnconvertedDeposit`] is returned, it
    /// means the tokens have been deposited, but the proper bandwidth credential hasn't yet been
    /// created. A [`VoucherBlob`] is returned that can be used for a later recovery of the
    /// associated bandwidth credential, using [`Self::recover`].
    pub async fn acquire(&self, amount: u128) -> Result<()> {
        let amount = Coin::new(amount, &self.network_details.chain_details.mix_denom.base);
        let state = nym_bandwidth_controller::acquire::deposit(&self.client.nyxd, amount).await?;
        nym_bandwidth_controller::acquire::get_credential(&state, &self.client, &self.storage)
            .await
            .map_err(|_| Error::UnconvertedDeposit {
                voucher_blob: state.voucher.to_bytes(),
            })
    }

    /// In case of an error in the mid of the acquire process, this function should be used for
    /// later retries to recover the bandwidth credential, either immediately or after some time.
    pub async fn recover(&self, voucher_blob: &VoucherBlob) -> Result<()> {
        let voucher = BandwidthVoucher::try_from_bytes(voucher_blob)
            .map_err(|_| Error::InvalidVoucherBlob)?;
        let state = State::new(voucher);
        nym_bandwidth_controller::acquire::get_credential(&state, &self.client, &self.storage)
            .await?;

        Ok(())
    }
}
