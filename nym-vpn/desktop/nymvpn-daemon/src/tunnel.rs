use talpid_core::tunnel_state_machine::TunnelParametersGenerator;
use talpid_types::net::{
    all_of_the_internet,
    wireguard::{ConnectionConfig, PeerConfig, PublicKey, TunnelConfig, TunnelOptions},
    GenericTunnelOptions, TunnelParameters,
};
use nymvpn_entity::device::Entity as Device;
use nymvpn_entity::vpn_session::Entity as VpnSession;
use nymvpn_migration::sea_orm::{DatabaseConnection, EntityTrait};
use nymvpn_types::device::DeviceDetails;

use crate::db::Db;

pub struct ParameterGenerator {
    db: Db,
}

impl ParameterGenerator {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

async fn generate_parameter(
    db: DatabaseConnection,
) -> Result<talpid_types::net::TunnelParameters, String> {
    let vpn_session = VpnSession::find()
        .one(&db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no vpn session found during parameter generation")?;

    let device = Device::find()
        .one(&db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no device found during parameter generation")?;

    let device_details = DeviceDetails::try_from(device)?;

    Ok(TunnelParameters::Wireguard(
        talpid_types::net::wireguard::TunnelParameters {
            connection: ConnectionConfig {
                tunnel: TunnelConfig {
                    private_key: device_details.wireguard_meta.private_key,
                    addresses: vec![std::net::IpAddr::V4(
                        device_details
                            .wireguard_meta
                            .device_addresses
                            .ok_or("no device addresses found in wireguard metadata")?
                            .ipv4_address,
                    )],
                },
                peer: PeerConfig {
                    public_key: PublicKey::from_base64(
                        &vpn_session
                            .server_public_key
                            .ok_or("no server public key found in vpn session model")?,
                    )
                    .map_err(|e| format!("failed to convert public key from base64: {e:?}"))?,
                    allowed_ips: all_of_the_internet(),
                    endpoint: vpn_session
                        .server_ipv4_endpoint
                        .ok_or("no server ipv4 endpoint found in vpn session model")?
                        .parse()
                        .map_err(|e| {
                            format!(
                                "failed to parse server ipv4 endpoint in vpn session model: {e:?}"
                            )
                        })?,
                    psk: None,
                },
                exit_peer: None,
                ipv4_gateway: vpn_session
                    .server_private_ipv4
                    .ok_or("no server private ipv4 found in vpn session model")?
                    .parse()
                    .map_err(|e| {
                        format!("failed to parse server ipv4 in vpn session model: {e:?}")
                    })?,
                ipv6_gateway: None,
                #[cfg(target_os = "linux")]
                fwmark: Some(nymvpn_types::TUNNEL_FWMARK),
            },
            options: TunnelOptions {
                mtu: None,
                quantum_resistant: false,
                #[cfg(windows)]
                use_wireguard_nt: true,
            },
            generic_options: GenericTunnelOptions { enable_ipv6: true },
            obfuscation: None,
        },
    ))
}

impl TunnelParametersGenerator for ParameterGenerator {
    fn generate(
        &mut self,
        _retry_attempt: u32,
    ) -> std::pin::Pin<
        Box<
            dyn futures::Future<
                Output = Result<
                    talpid_types::net::TunnelParameters,
                    talpid_types::tunnel::ParameterGenerationError,
                >,
            >,
        >,
    > {
        let db = self.db.connection();
        Box::pin(async move {
            let parameters = generate_parameter(db).await;

            match parameters {
                Ok(parameters) => Ok(parameters),
                Err(e) => {
                    tracing::error!("TunnelParameterGenerator: {e}");
                    // return placeholder error
                    Err(talpid_types::tunnel::ParameterGenerationError::NoMatchingRelay)
                }
            }
        })
    }
}
