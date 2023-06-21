use std::fmt::Display;
use std::future::Future;
use std::io::Write;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::{future, FutureExt};
use log::error;
use nym_ephemera_common::types::JsonPeerInfo;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::crypto::PublicKey;
use crate::network::{Address, Peer};
use crate::peer::PeerId;

/// Information about an Ephemera peer.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PeerInfo {
    /// The cosmos address of the peer, used in interacting with the chain.
    pub cosmos_address: String,
    /// The address of the peer.
    /// Expected formats:
    /// 1. `<IP>:<PORT>`
    /// 2. `/ip4/<IP>/tcp/<PORT>` - this is the format used by libp2p multiaddr
    pub address: String,
    /// The public key of the peer. It uniquely identifies the peer.
    /// Public key is used to derive the peer id.
    pub pub_key: PublicKey,
}

impl Display for PeerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cosmos address {}, address {}, public key {}",
            self.cosmos_address, self.address, self.pub_key
        )
    }
}

impl TryFrom<PeerInfo> for Peer {
    type Error = anyhow::Error;

    fn try_from(value: PeerInfo) -> std::result::Result<Self, Self::Error> {
        let address: Address = value.address.parse()?;
        let public_key = value.pub_key;
        Ok(Self {
            cosmos_address: value.cosmos_address,
            address,
            public_key: public_key.clone(),
            peer_id: PeerId::from_public_key(&public_key),
        })
    }
}

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("ResourceUnavailable: {0}")]
    ResourceUnavailable(String),
    #[error("MembersProvider: {0}")]
    MembersProvider(#[from] anyhow::Error),
    #[error("Could not get peers - {0}")]
    GetPeers(String),
}

pub type Result<T> = std::result::Result<T, ProviderError>;

/// A membership provider that does nothing.
/// Might be useful for testing.
pub struct DummyMembersProvider;

#[allow(clippy::missing_errors_doc, clippy::unused_async)]
impl DummyMembersProvider {
    pub async fn empty_peers_list() -> Result<Vec<PeerInfo>> {
        Ok(vec![])
    }
}

#[derive(Error, Debug)]
pub enum ConfigMembersProviderError {
    #[error("ConfigDoesNotExist: '{0}'")]
    NotExist(String),
    #[error("ParsingFailed: {0}")]
    ParsingFailed(#[from] config::ConfigError),
    #[error("TomlError: {0}")]
    TomlError(#[from] toml::ser::Error),
    #[error("IoError: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PeerSetting {
    /// The cosmos address of the peer, used in interacting with the chain.
    pub cosmos_address: String,
    /// The address of the peer.
    /// Expected formats:
    /// 1. `<IP>:<PORT>`
    /// 2. `/ip4/<IP>/tcp/<PORT>` - this is the format used by libp2p multiaddr
    pub address: String,
    ///Serialized public key.
    ///
    /// # Converting to string and back example
    ///```
    /// use ephemera::crypto::{EphemeraKeypair, EphemeraPublicKey, Keypair, PublicKey};
    ///
    /// let public_key = Keypair::generate(None).public_key();
    ///
    /// let public_key_str = public_key.to_string();
    ///
    /// let public_key_parsed = public_key_str.parse::<PublicKey>().unwrap();
    ///
    /// assert_eq!(public_key, public_key_parsed);
    /// ```
    pub public_key: String,
}

impl TryFrom<PeerSetting> for PeerInfo {
    type Error = anyhow::Error;

    fn try_from(setting: PeerSetting) -> std::result::Result<Self, Self::Error> {
        let pub_key = setting.public_key.parse::<PublicKey>()?;
        Ok(PeerInfo {
            cosmos_address: setting.cosmos_address,
            address: setting.address,
            pub_key,
        })
    }
}

///[`ProviderFut`] that reads the peers from a toml config file.
///
/// # Configuration example
/// ```toml
/// [[peers]]
/// name = "node1"
/// address = "/ip4/127.0.0.1/tcp/3000"
/// pub_key = "4XTTMEghav9LZThm6opUaHrdGEEYUkrfkakVg4VAetetBZDWJ"
///
/// [[peers]]
/// name = "node2"
/// address = "/ip4/127.0.0.1/tcp/3001"
/// pub_key = "4XTTMFQt2tgNRmwRgEAaGQe2NXygsK6Vr3pkuBfYezhDfoVty"
/// ```
pub struct ConfigMembersProvider {
    config_location: PathBuf,
}

impl ConfigMembersProvider {
    /// Creates a new [`ConfigMembersProvider`] instance.
    ///
    /// # Arguments
    /// * `path` - Path to the peers toml config file.
    ///
    /// # Errors
    /// Returns [`ConfigMembersProviderError::NotExist`] if the file does not exist.
    /// Returns [`ConfigMembersProviderError::ParsingFailed`] if the file is not a valid members file.
    pub fn init<I: Into<PathBuf>>(
        path: I,
    ) -> std::result::Result<Self, ConfigMembersProviderError> {
        let path_buf = path.into();
        if !path_buf.exists() {
            return Err(ConfigMembersProviderError::NotExist(
                path_buf.to_string_lossy().to_string(),
            ));
        }

        let provider = Self {
            config_location: path_buf,
        };

        if provider.read_config().is_err() {
            return Err(ConfigMembersProviderError::ParsingFailed(
                config::ConfigError::Message("Failed to parse config".to_string()),
            ));
        }

        Ok(provider)
    }

    pub(crate) fn read_config(&self) -> Result<Vec<PeerInfo>> {
        let config_peers = ConfigPeers::try_load(self.config_location.clone())
            .map_err(|err| anyhow::anyhow!(err))?;

        let peers = config_peers
            .peers
            .iter()
            .map(|peer| PeerInfo::try_from(peer.clone()))
            .collect::<anyhow::Result<Vec<PeerInfo>>>()?;
        Ok(peers)
    }
}

impl Future for ConfigMembersProvider {
    type Output = Result<Vec<PeerInfo>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        future::ready(self.read_config()).poll_unpin(cx)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConfigPeers {
    peers: Vec<PeerSetting>,
}

impl ConfigPeers {
    pub(crate) fn new(peers: Vec<PeerSetting>) -> Self {
        Self { peers }
    }

    pub(crate) fn try_load<I: Into<PathBuf>>(
        path: I,
    ) -> std::result::Result<ConfigPeers, ConfigMembersProviderError> {
        let path = path.into();
        let config = config::Config::builder()
            .add_source(config::File::from(path))
            .build()?;

        config.try_deserialize().map_err(Into::into)
    }

    pub(crate) fn try_write<I: Into<PathBuf>>(
        &self,
        path: I,
    ) -> std::result::Result<(), ConfigMembersProviderError> {
        let config = toml::to_string(&self)?;

        let config = format!(
            "#This file is generated by cli and automatically overwritten every time when cli is §\n{config}",
        );

        let mut file = std::fs::File::create(path.into())?;
        file.write_all(config.as_bytes())?;

        Ok(())
    }
}

impl TryFrom<JsonPeerInfo> for PeerInfo {
    type Error = anyhow::Error;

    fn try_from(json_peer_info: JsonPeerInfo) -> std::result::Result<Self, Self::Error> {
        let pub_key = json_peer_info.public_key.parse::<PublicKey>()?;
        Ok(PeerInfo {
            cosmos_address: json_peer_info.cosmos_address.to_string(),
            address: json_peer_info.ip_address,
            pub_key,
        })
    }
}
