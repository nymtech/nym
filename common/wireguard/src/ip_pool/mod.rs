// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod compat;

use defguard_wireguard_rs::host::Peer;
use ipnetwork::IpNetwork;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

// helper to convert peer's allocation into an `IpPair`
pub fn allocated_ip_pair(peer: &Peer) -> Option<IpPair> {
    for allowed_ip in &peer.allowed_ips {
        // Extract IPv4 and IPv6 from peer's allowed_ips
        if let IpAddr::V4(ipv4) = allowed_ip.address {
            // Find corresponding IPv6
            if let Some(ipv6_mask) = peer
                .allowed_ips
                .iter()
                .find(|ip| matches!(ip.address, IpAddr::V6(_)))
                && let IpAddr::V6(ipv6) = ipv6_mask.address
            {
                return Some(IpPair::new(ipv4, ipv6));
            }
        }
    }
    None
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IpPair {
    pub ipv4: Ipv4Addr,
    pub ipv6: Ipv6Addr,
}

impl IpPair {
    pub fn new(ipv4: Ipv4Addr, ipv6: Ipv6Addr) -> Self {
        IpPair { ipv4, ipv6 }
    }
}

impl Display for IpPair {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "IPv4: {}, IPv6: {}", self.ipv4, self.ipv6)
    }
}

/// Represents the state of an IP allocation
#[derive(Debug, Clone, Copy)]
pub enum AllocationState {
    /// IP is available for allocation
    Free,
    /// IP is allocated and in use, with timestamp of allocation
    Allocated(SystemTime),
}

/// Thread-safe IP address pool manager
///
/// Manages allocation of IPv4/IPv6 address pairs from configured CIDR ranges.
/// Ensures collision-free allocation and supports stale cleanup.
#[derive(Clone)]
pub struct IpPool {
    allocations: Arc<RwLock<HashMap<IpPair, AllocationState>>>,
}

impl IpPool {
    /// Create a new IP pool from IPv4 and IPv6 CIDR ranges
    ///
    /// # Arguments
    /// * `ipv4_network` - Base IPv4 address for the pool
    /// * `ipv4_prefix` - CIDR prefix length for IPv4 (e.g., 16 for /16)
    /// * `ipv6_network` - Base IPv6 address for the pool
    /// * `ipv6_prefix` - CIDR prefix length for IPv6 (e.g., 112 for /112)
    ///
    /// # Errors
    /// Returns error if CIDR ranges are invalid
    pub fn new(
        ipv4_network: Ipv4Addr,
        ipv4_prefix: u8,
        ipv6_network: Ipv6Addr,
        ipv6_prefix: u8,
    ) -> Result<Self, IpPoolError> {
        let ipv4_net = IpNetwork::new(ipv4_network.into(), ipv4_prefix)?;
        let ipv6_net = IpNetwork::new(ipv6_network.into(), ipv6_prefix)?;

        // Build initial pool with all IPs marked as free
        let mut allocations = HashMap::new();

        // Collect IPv4 and IPv6 addresses into vectors for pairing
        let ipv4_addrs: Vec<Ipv4Addr> = ipv4_net
            .iter()
            .filter_map(|ip| {
                if let IpAddr::V4(v4) = ip {
                    Some(v4)
                } else {
                    None
                }
            })
            .collect();

        let ipv6_addrs: Vec<Ipv6Addr> = ipv6_net
            .iter()
            .filter_map(|ip| {
                if let IpAddr::V6(v6) = ip {
                    Some(v6)
                } else {
                    None
                }
            })
            .collect();

        // Create IpPairs by matching IPv4 and IPv6 addresses
        // Use the minimum length to avoid index out of bounds
        let pair_count = ipv4_addrs.len().min(ipv6_addrs.len());
        for i in 0..pair_count {
            let pair = IpPair::new(ipv4_addrs[i], ipv6_addrs[i]);
            allocations.insert(pair, AllocationState::Free);
        }

        tracing::info!(
            "Initialized IP pool with {} address pairs from {ipv4_network}/{ipv4_prefix} and {ipv6_network}/{ipv6_prefix}",
            allocations.len(),
        );

        Ok(IpPool {
            allocations: Arc::new(RwLock::new(allocations)),
        })
    }

    /// Allocate a free IP pair from the pool
    ///
    /// # Errors
    /// Returns `IpPoolError::NoFreeIp` if no IPs are available
    pub async fn allocate(&self) -> Result<IpPair, IpPoolError> {
        let mut pool = self.allocations.write().await;

        // Find a free IP and allocate it
        let free_ip = pool
            .iter_mut()
            .filter(|(_, state)| matches!(state, AllocationState::Free))
            .next()
            .ok_or(IpPoolError::NoFreeIp)?;

        let ip_pair = *free_ip.0;
        *free_ip.1 = AllocationState::Allocated(SystemTime::now());

        tracing::debug!("Allocated IP pair: {ip_pair}");
        Ok(ip_pair)
    }

    /// Release an IP pair back to the pool
    ///
    /// Marks the IP as free for future allocations.
    pub async fn release(&self, ip_pair: IpPair) {
        let mut pool = self.allocations.write().await;
        if let Some(state) = pool.get_mut(&ip_pair) {
            *state = AllocationState::Free;
            tracing::debug!("Released IP pair: {ip_pair}");
        }
    }

    /// Mark an IP pair as allocated (used during initialization from database)
    ///
    /// This is used when restoring state from the database on gateway startup.
    pub async fn mark_used(&self, ip_pair: IpPair) {
        let mut pool = self.allocations.write().await;
        if let Some(state) = pool.get_mut(&ip_pair) {
            *state = AllocationState::Allocated(SystemTime::now());
            tracing::debug!("Marked IP pair as used: {ip_pair}");
        } else {
            tracing::warn!("Attempted to mark unknown IP pair as used: {ip_pair}");
        }
    }

    /// Get the number of free IPs in the pool
    pub async fn free_count(&self) -> usize {
        let pool = self.allocations.read().await;
        pool.iter()
            .filter(|(_, state)| matches!(state, AllocationState::Free))
            .count()
    }

    /// Get the number of allocated IPs in the pool
    pub async fn allocated_count(&self) -> usize {
        let pool = self.allocations.read().await;
        pool.iter()
            .filter(|(_, state)| matches!(state, AllocationState::Allocated(_)))
            .count()
    }

    /// Get the total pool size
    pub async fn total_count(&self) -> usize {
        let pool = self.allocations.read().await;
        pool.len()
    }

    /// Clean up stale allocations older than the specified duration
    ///
    /// Returns the number of IPs that were freed
    pub async fn cleanup_stale(&self, max_age: std::time::Duration) -> usize {
        let mut pool = self.allocations.write().await;
        let now = SystemTime::now();
        let mut freed = 0;

        for (_ip, state) in pool.iter_mut() {
            if let AllocationState::Allocated(allocated_at) = state
                && let Ok(age) = now.duration_since(*allocated_at)
                && age > max_age
            {
                *state = AllocationState::Free;
                freed += 1;
            }
        }

        if freed > 0 {
            tracing::info!("Cleaned up {} stale IP allocations", freed);
        }

        freed
    }
}

/// Errors that can occur during IP pool operations
#[derive(Debug, thiserror::Error)]
pub enum IpPoolError {
    #[error("No free IP addresses available in pool")]
    NoFreeIp,

    #[error("Invalid IP network configuration: {0}")]
    InvalidNetwork(#[from] ipnetwork::IpNetworkError),
}
