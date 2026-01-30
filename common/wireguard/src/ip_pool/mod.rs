// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use defguard_wireguard_rs::host::Peer;
use ipnetwork::IpNetwork;
use rand::seq::IteratorRandom;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;
use tracing::{trace, warn};

mod compat;

#[cfg(test)]
use mock_instant::thread_local::Instant;
#[cfg(not(test))]
use std::time::Instant;

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

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Hash)]
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

    /// The IP has been pre-allocated for a peer, but the corresponding registration has not yet been finalised
    PreAllocated { allocated_at: Instant },

    /// IP is allocated and in use, with timestamp
    Allocated { allocated_at: Instant },
}

impl AllocationState {
    pub fn is_free(&self) -> bool {
        matches!(self, AllocationState::Free)
    }

    pub fn new_pre_allocated() -> Self {
        AllocationState::PreAllocated {
            allocated_at: Instant::now(),
        }
    }

    pub fn new_allocated() -> Self {
        AllocationState::Allocated {
            allocated_at: Instant::now(),
        }
    }
}

/// Thread-safe IP address pool manager
///
/// Manages allocation of IPv4/IPv6 address pairs from configured CIDR ranges.
/// Ensures collision-free allocation and supports stale cleanup.
pub struct IpPool {
    allocations: HashMap<IpPair, AllocationState>,
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
                    if v4 != ipv4_network { Some(v4) } else { None }
                } else {
                    None
                }
            })
            .collect();

        let ipv6_addrs: Vec<Ipv6Addr> = ipv6_net
            .iter()
            .filter_map(|ip| {
                if let IpAddr::V6(v6) = ip {
                    if v6 != ipv6_network { Some(v6) } else { None }
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

        Ok(IpPool { allocations })
    }

    /// Preallocate a free IP pair from the pool
    ///
    /// # Errors
    /// Returns `IpPoolError::NoFreeIp` if no IPs are available
    pub fn pre_allocate(&mut self) -> Result<IpPair, IpPoolError> {
        // Find a free IP and allocate it
        let assignment_start = Instant::now();
        let free_ip = self
            .allocations
            .iter_mut()
            .filter(|(_, state)| matches!(state, AllocationState::Free))
            .choose(&mut rand::thread_rng())
            .ok_or(IpPoolError::NoFreeIp)?;
        let taken = assignment_start.elapsed();
        trace!("assigning free ip pair took {taken:?}");
        if taken > Duration::from_millis(500) {
            warn!("assigning free ip pair took {taken:?}");
        }

        let ip_pair = *free_ip.0;
        *free_ip.1 = AllocationState::new_pre_allocated();

        tracing::debug!("Allocated IP pair: {ip_pair}");
        Ok(ip_pair)
    }

    pub fn confirm_allocation(&mut self, ip_pair: IpPair) -> Result<(), IpPoolError> {
        let Some(allocation) = self.allocations.get_mut(&ip_pair) else {
            return Err(IpPoolError::UnknownIpPair { ip_pair });
        };
        match allocation {
            AllocationState::Free => {
                // seems the IpPair has been released before the confirmation, but it has not yet been re-allocated
                warn!(
                    "{ip_pair} seems to have already been released, but has not been allocated to a new peer yet"
                );
                *allocation = AllocationState::Allocated {
                    allocated_at: Instant::now(),
                };
                Ok(())
            }
            AllocationState::PreAllocated { allocated_at } => {
                *allocation = AllocationState::Allocated {
                    allocated_at: *allocated_at,
                };
                Ok(())
            }
            AllocationState::Allocated { .. } => Err(IpPoolError::AlreadyUsed { ip_pair }),
        }
    }

    /// Release an IP pair back to the pool
    ///
    /// Marks the IP as free for future allocations.
    pub fn release(&mut self, ip_pair: IpPair) {
        if let Some(state) = self.allocations.get_mut(&ip_pair) {
            *state = AllocationState::Free;
            tracing::debug!("Released IP pair: {ip_pair}");
        }
    }

    /// Mark an IP pair as allocated (used during initialization from database)
    ///
    /// This is used when restoring state from the database on gateway startup.
    pub fn mark_used(&mut self, ip_pair: IpPair) -> Result<(), IpPoolError> {
        let Some(state) = self.allocations.get_mut(&ip_pair) else {
            return Err(IpPoolError::UnknownIpPair { ip_pair });
        };

        if !state.is_free() {
            return Err(IpPoolError::AlreadyUsed { ip_pair });
        }
        tracing::debug!("Marked IP pair as used: {ip_pair}");
        *state = AllocationState::new_allocated();
        Ok(())
    }

    /// Get the number of free IPs in the pool
    pub fn free_count(&self) -> usize {
        self.allocations
            .iter()
            .filter(|(_, state)| matches!(state, AllocationState::Free))
            .count()
    }

    /// Get the number of allocated IPs in the pool
    pub fn allocated_count(&self) -> usize {
        self.allocations
            .iter()
            .filter(|(_, state)| matches!(state, AllocationState::Allocated { .. }))
            .count()
    }

    /// Get the total pool size
    pub fn total_count(&self) -> usize {
        self.allocations.len()
    }

    /// Clean up stale allocations older than the specified duration
    ///
    /// Returns the number of IPs that were freed
    pub fn cleanup_stale(&mut self, max_age: Duration) -> usize {
        let now = Instant::now();
        let mut freed = 0;

        for state in self.allocations.values_mut() {
            println!("entry: {state:?}");
            if let AllocationState::PreAllocated { allocated_at, .. } = state {
                let age = now.duration_since(*allocated_at);
                if age > max_age {
                    *state = AllocationState::Free;
                    freed += 1;
                }
            }
        }

        if freed > 0 {
            tracing::info!("Cleaned up {freed} stale IP allocations");
        }

        freed
    }
}

/// Errors that can occur during IP pool operations
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IpPoolError {
    #[error("No free IP addresses available in pool")]
    NoFreeIp,

    #[error("Attempted to mark an IpPair that is already in used: {ip_pair}")]
    AlreadyUsed { ip_pair: IpPair },

    #[error("Attempted to mark an unknown ip pair: {ip_pair}")]
    UnknownIpPair { ip_pair: IpPair },

    #[error("Invalid IP network configuration: {0}")]
    InvalidNetwork(#[from] ipnetwork::IpNetworkError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::bail;
    use mock_instant::thread_local::MockClock;

    // 3 addresses in each pool
    fn small_ip_pool() -> IpPool {
        IpPool::new(
            Ipv4Addr::new(10, 0, 0, 0),
            30,
            Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 0),
            126,
        )
        .unwrap()
    }

    #[test]
    fn ip_pool_initial_allocation() -> anyhow::Result<()> {
        let base_ipv4_network = Ipv4Addr::new(10, 0, 0, 0);
        let base_ipv4_prefix = 24; // 255 addresses
        let base_ipv6_network = Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 0);
        let base_ipv6_prefix = 112; // 65535 addresses

        // ipv4 pool size < ipv6 pool size
        let base_ip_pool = IpPool::new(
            base_ipv4_network,
            base_ipv4_prefix,
            base_ipv6_network,
            base_ipv6_prefix,
        )?;
        let inner = base_ip_pool.allocations;
        // minimum of ipv4 and ipv6 allocations
        assert_eq!(inner.len(), 255);

        // no ipv4 addresses
        let base_ip_pool = IpPool::new(base_ipv4_network, 32, base_ipv6_network, base_ipv6_prefix)?;
        let inner = base_ip_pool.allocations;
        assert_eq!(inner.len(), 0);

        // no ipv6 addresses
        let base_ip_pool =
            IpPool::new(base_ipv4_network, base_ipv4_prefix, base_ipv6_network, 128)?;
        let inner = base_ip_pool.allocations;
        assert_eq!(inner.len(), 0);

        // ipv4 pool size == ipv6 pool size
        let base_ip_pool = IpPool::new(base_ipv4_network, 16, base_ipv6_network, base_ipv6_prefix)?;
        let inner = base_ip_pool.allocations;
        assert_eq!(inner.len(), 65535);

        // ipv4 pool size > ipv6 pool size
        let base_ip_pool = IpPool::new(base_ipv4_network, 12, base_ipv6_network, base_ipv6_prefix)?;
        let inner = base_ip_pool.allocations;
        assert_eq!(inner.len(), 65535);

        Ok(())
    }

    fn ensure_different_allocation(left: IpPair, right: IpPair) -> anyhow::Result<()> {
        if left.ipv4 == right.ipv4 || left.ipv6 == right.ipv6 {
            bail!("ip allocation overlap")
        }

        Ok(())
    }

    #[test]
    fn ip_pool_allocation() -> anyhow::Result<()> {
        let mut pool = small_ip_pool();
        assert_eq!(pool.allocations.len(), 3);

        let gateway_pair = IpPair {
            ipv4: Ipv4Addr::new(10, 0, 0, 0),
            ipv6: Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 0),
        };

        assert_eq!(pool.free_count(), 3);
        assert_eq!(pool.allocated_count(), 0);

        let allocation1 = pool.pre_allocate()?;
        assert_eq!(pool.free_count(), 2);
        assert_eq!(pool.allocated_count(), 1);

        let allocation2 = pool.pre_allocate()?;
        assert_eq!(pool.free_count(), 1);
        assert_eq!(pool.allocated_count(), 2);

        let allocation3 = pool.pre_allocate()?;
        assert_eq!(pool.free_count(), 0);
        assert_eq!(pool.allocated_count(), 3);

        // make sure each was unique and different from the gateway
        ensure_different_allocation(allocation1, allocation2)?;
        ensure_different_allocation(allocation1, allocation3)?;
        ensure_different_allocation(allocation2, allocation3)?;

        ensure_different_allocation(allocation1, gateway_pair)?;
        ensure_different_allocation(allocation2, gateway_pair)?;
        ensure_different_allocation(allocation3, gateway_pair)?;

        // allocation 4 will fail as we have run out of addresses
        assert_eq!(pool.pre_allocate().unwrap_err(), IpPoolError::NoFreeIp);

        // if pair gets released, it's eligible for allocation again
        pool.release(allocation2);
        assert_eq!(pool.free_count(), 1);
        assert_eq!(pool.allocated_count(), 2);

        let reallocation = pool.pre_allocate()?;
        assert_eq!(reallocation, allocation2);

        assert_eq!(pool.free_count(), 0);
        assert_eq!(pool.allocated_count(), 3);

        Ok(())
    }

    #[test]
    fn ip_pool_mark_used() -> anyhow::Result<()> {
        let mut pool = small_ip_pool();

        let pair1 = IpPair::new(
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1),
        );
        let pair2 = IpPair::new(
            Ipv4Addr::new(10, 0, 0, 2),
            Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 2),
        );
        let pair3 = IpPair::new(
            Ipv4Addr::new(10, 0, 0, 3),
            Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 3),
        );

        let bad_pair1 = IpPair::new(
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 4),
        );
        let bad_pair2 = IpPair::new(
            Ipv4Addr::new(10, 0, 0, 4),
            Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1),
        );

        assert!(pool.mark_used(pair1,).is_ok());
        assert_eq!(
            pool.mark_used(pair1).unwrap_err(),
            IpPoolError::AlreadyUsed { ip_pair: pair1 }
        );

        assert!(pool.mark_used(pair2).is_ok());
        assert!(pool.mark_used(pair3).is_ok());

        assert_eq!(
            pool.mark_used(bad_pair1).unwrap_err(),
            IpPoolError::UnknownIpPair { ip_pair: bad_pair1 }
        );

        assert_eq!(
            pool.mark_used(bad_pair2,).unwrap_err(),
            IpPoolError::UnknownIpPair { ip_pair: bad_pair2 }
        );

        Ok(())
    }

    #[test]
    fn ip_pool_cleanup() -> anyhow::Result<()> {
        MockClock::set_time(Duration::ZERO);

        let mut pool = small_ip_pool();

        let age_threshold = Duration::from_secs(1);

        // nothing to cleanup
        assert_eq!(pool.cleanup_stale(age_threshold), 0);

        // just allocated
        let pair1 = pool.pre_allocate()?;
        let pair2 = pool.pre_allocate()?;
        assert_eq!(pool.cleanup_stale(age_threshold), 0);

        // advance time to go beyond the allocation threshold
        MockClock::advance(Duration::from_millis(1001));
        assert_eq!(pool.cleanup_stale(age_threshold), 2);

        // ensure those pairs are now marked as free
        assert!(pool.allocations.get(&pair1).unwrap().is_free());
        assert!(pool.allocations.get(&pair2).unwrap().is_free());

        pool.pre_allocate()?;
        MockClock::advance(Duration::from_millis(500));
        pool.pre_allocate()?;

        assert_eq!(pool.cleanup_stale(age_threshold), 0);
        MockClock::advance(Duration::from_millis(501));
        assert_eq!(pool.cleanup_stale(age_threshold), 1);

        MockClock::advance(Duration::from_millis(500));
        assert_eq!(pool.cleanup_stale(age_threshold), 1);

        let mut new_pool = small_ip_pool();
        let pair1 = new_pool.pre_allocate()?;
        let pair2 = new_pool.pre_allocate()?;

        // complete allocation for pair2
        new_pool.confirm_allocation(pair2)?;
        MockClock::advance(Duration::from_millis(2000));

        // only pair1 should have got cleaned up
        assert_eq!(new_pool.cleanup_stale(age_threshold), 1);
        assert!(new_pool.allocations.get(&pair1).unwrap().is_free());
        assert!(!new_pool.allocations.get(&pair2).unwrap().is_free());

        Ok(())
    }
}
