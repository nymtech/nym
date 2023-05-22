use std::fmt::Display;
use std::future::Future;
use std::io::Write;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::Poll::Pending;
use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use futures_util::{future, FutureExt};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::crypto::PublicKey;
use crate::network::{Address, Peer};
use crate::peer::PeerId;

/// Information about an Ephemera peer.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PeerInfo {
    /// The name of the peer. Can be arbitrary.
    pub name: String,
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
            "name {}, address {}, public key {}",
            self.name, self.address, self.pub_key
        )
    }
}

impl TryFrom<PeerInfo> for Peer {
    type Error = anyhow::Error;

    fn try_from(value: PeerInfo) -> std::result::Result<Self, Self::Error> {
        let address: Address = value.address.parse()?;
        let public_key = value.pub_key;
        Ok(Self {
            name: value.name,
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
}

/// Future type which allows user to implement their own peers membership source mechanism.
pub type ProviderFut = BoxFuture<'static, Result<Vec<PeerInfo>>>;

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
    /// The name of the peer. Can be arbitrary.
    pub name: String,
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
            name: setting.name,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct JsonPeerInfo {
    /// The name of the peer. Can be arbitrary.
    pub name: String,
    /// The address of the peer. See [PeerInfo] for more details.
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

impl JsonPeerInfo {
    #[must_use]
    pub fn new(name: String, address: String, pub_key: String) -> Self {
        Self {
            name,
            address,
            public_key: pub_key,
        }
    }
}

impl TryFrom<JsonPeerInfo> for PeerInfo {
    type Error = anyhow::Error;

    fn try_from(json_peer_info: JsonPeerInfo) -> std::result::Result<Self, Self::Error> {
        let pub_key = json_peer_info.public_key.parse::<PublicKey>()?;
        Ok(PeerInfo {
            name: json_peer_info.name,
            address: json_peer_info.address,
            pub_key,
        })
    }
}

///[`ProviderFut`] that reads peers from a http endpoint.
///
/// The endpoint must return a json array of [`JsonPeerInfo`].
/// # Configuration example
/// ```json
/// [
///  {
///     "name": "node1",
///     "address": "/ip4/",
///     "public_key": "4XTTMEghav9LZThm6opUaHrdGEEYUkrfkakVg4VAetetBZDWJ"
///   },
///  {
///     "name": "node2",
///     "address": "/ip4/",
///     "public_key": "4XTTMFQt2tgNRmwRgEAaGQe2NXygsK6Vr3pkuBfYezhDfoVty"
///   }
/// ]
/// ```
pub struct HttpMembersProvider {
    /// The url of the http endpoint.
    members_url: String,
    fut: Option<ProviderFut>,
}

impl HttpMembersProvider {
    #[must_use]
    pub fn new(members_url: String) -> Self {
        Self {
            members_url,
            fut: None,
        }
    }

    async fn request_peers(members_url: String) -> Result<Vec<PeerInfo>> {
        debug!("Requesting peers from: {:?}", members_url);
        let json_peers: Vec<JsonPeerInfo> = reqwest::get(members_url)
            .await
            .map_err(|err| anyhow::anyhow!("Failed to get peers: {err}"))?
            .json()
            .await
            .map_err(|err| anyhow::anyhow!("Failed to parse peers: {err}"))?;

        let peers = json_peers
            .into_iter()
            .map(TryInto::try_into)
            .collect::<anyhow::Result<Vec<PeerInfo>>>()?;

        Ok(peers)
    }
}

impl Future for HttpMembersProvider {
    type Output = Result<Vec<PeerInfo>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.fut.take() {
            None => {
                self.fut = Some(Box::pin(HttpMembersProvider::request_peers(
                    self.members_url.clone(),
                )));
            }
            Some(mut fut) => {
                let peers = match fut.poll_unpin(cx) {
                    Poll::Ready(Ok(peers)) => peers,
                    Poll::Ready(Err(err)) => {
                        error!("Failed to get peers: {err}");
                        return Poll::Ready(Err(err));
                    }
                    Pending => {
                        self.fut = Some(fut);
                        return Pending;
                    }
                };

                return Poll::Ready(Ok(peers));
            }
        }
        Pending
    }
}
