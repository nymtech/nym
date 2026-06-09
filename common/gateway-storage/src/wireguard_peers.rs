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
        let psk = peer.psk.as_deref();
        sqlx::query!(
            r#"
                INSERT OR IGNORE INTO wireguard_peer(public_key, allowed_ips, client_id, psk)
                VALUES (?, ?, ?, ?);

                UPDATE wireguard_peer 
                SET allowed_ips = ?, client_id = ?, psk = ?
                WHERE public_key = ?
            "#,
            peer.public_key,
            peer.allowed_ips,
            peer.client_id,
            psk,
            peer.allowed_ips,
            peer.client_id,
            psk,
            peer.public_key,
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

    /// Update the stored PSK of the wireguard peer.
    ///
    /// # Arguments
    ///
    /// * `public_key`: the unique public key of the wireguard peer.
    /// * `psk`: the PSK of the wireguard peer.
    pub(crate) async fn update_peer_psk(
        &self,
        public_key: &str,
        psk: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE wireguard_peer
                SET psk = ?
                WHERE public_key = ?
            "#,
            psk,
            public_key,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::WireguardPeer;
    use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask};
    use std::net::Ipv4Addr;

    fn test_peer(public_key: Key, psk: Key) -> Peer {
        let mut peer = Peer::new(public_key);
        peer.allowed_ips = vec![IpAddrMask::new(Ipv4Addr::new(10, 0, 0, 2).into(), 32)];
        peer.preshared_key = Some(psk);
        peer
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn insert_peer_persists_psk_on_insert_and_update(
        pool: sqlx::SqlitePool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query("INSERT INTO clients (id, client_type) VALUES (?, ?)")
            .bind(1_i64)
            .bind("entry_wireguard")
            .execute(&pool)
            .await?;

        let manager = WgPeerManager::new(pool);
        let public_key = Key::new([1; 32]);

        let first_psk = Key::new([2; 32]);
        let first_psk_hex = first_psk.to_lower_hex();
        let first_peer = test_peer(public_key.clone(), first_psk.clone());
        manager
            .insert_peer(&WireguardPeer::from_defguard_peer(first_peer.clone(), 1)?)
            .await?;

        let retrieved = manager
            .retrieve_peer(&first_peer.public_key.to_string())
            .await?
            .expect("peer should be present after insert");
        assert_eq!(retrieved.psk.as_deref(), Some(first_psk_hex.as_str()));

        let second_psk = Key::new([3; 32]);
        let second_psk_hex = second_psk.to_lower_hex();
        let second_peer = test_peer(public_key, second_psk.clone());
        manager
            .insert_peer(&WireguardPeer::from_defguard_peer(second_peer.clone(), 1)?)
            .await?;

        let retrieved = manager
            .retrieve_peer(&second_peer.public_key.to_string())
            .await?
            .expect("peer should be present after update");
        assert_eq!(retrieved.psk.as_deref(), Some(second_psk_hex.as_str()));

        Ok(())
    }
}
