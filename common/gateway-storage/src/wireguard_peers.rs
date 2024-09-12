// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::models::WireguardPeer;

#[derive(Clone)]
pub(crate) struct WgPeerManager {
    connection_pool: sqlx::SqlitePool,
}

impl WgPeerManager {
    /// Creates new instance of the `WgPeersManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub(crate) fn new(connection_pool: sqlx::SqlitePool) -> Self {
        WgPeerManager { connection_pool }
    }

    /// Creates a new wireguard peer entry for its particular public key or
    /// overwrittes the peer entry data if it already existed.
    ///
    /// # Arguments
    ///
    /// * `peer`: peer information needed by wireguard interface.
    pub(crate) async fn insert_peer(&self, peer: &WireguardPeer) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT OR IGNORE INTO wireguard_peer(public_key, preshared_key, protocol_version, endpoint, last_handshake, tx_bytes, rx_bytes, persistent_keepalive_interval, allowed_ips, suspended)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?);

                UPDATE wireguard_peer 
                SET preshared_key = ?, protocol_version = ?, endpoint = ?, last_handshake = ?, tx_bytes = ?, rx_bytes = ?, persistent_keepalive_interval = ?, allowed_ips = ?, suspended = ?
                WHERE public_key = ?
            "#,
            peer.public_key, peer.preshared_key, peer.protocol_version, peer.endpoint, peer.last_handshake, peer.tx_bytes, peer.rx_bytes, peer.persistent_keepalive_interval, peer.allowed_ips, peer.suspended,

            peer.preshared_key, peer.protocol_version, peer.endpoint, peer.last_handshake, peer.tx_bytes, peer.rx_bytes, peer.persistent_keepalive_interval, peer.allowed_ips, peer.suspended,peer.public_key,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Retrieve the wireguard peer with the provided public key from the storage.
    ///
    /// # Arguments
    ///
    /// * `public_key`: the unique public key of the wireguard peer.
    pub(crate) async fn retrieve_peer(
        &self,
        public_key: &str,
    ) -> Result<Option<WireguardPeer>, sqlx::Error> {
        sqlx::query_as!(
            WireguardPeer,
            r#"
                SELECT * FROM wireguard_peer
                WHERE public_key = ?
                LIMIT 1
            "#,
            public_key,
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    /// Retrieve all wireguard peers.
    pub(crate) async fn retrieve_all_peers(&self) -> Result<Vec<WireguardPeer>, sqlx::Error> {
        sqlx::query_as!(
            WireguardPeer,
            r#"
                    SELECT *
                    FROM wireguard_peer;
                "#,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Remove the wireguard peer with the provided public key from the storage.
    ///
    /// # Arguments
    ///
    /// * `public_key`: the unique public key of the wireguard peer.
    pub(crate) async fn remove_peer(&self, public_key: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                DELETE FROM wireguard_peer
                WHERE public_key = ?
            "#,
            public_key,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}
