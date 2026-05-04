use std::collections::{HashMap, HashSet};
use tetra_core::TimeslotAllocator;

#[derive(Debug, Clone, Default)]
pub struct RxGainDecodeCounters {
    pub decode_attempted: u64,
    pub decode_success: u64,
    pub crc_ok: u64,
    pub false_positive: u64,
    pub slot_attempted: [u64; 4],
    pub slot_crc_ok: [u64; 4],
}

impl RxGainDecodeCounters {
    pub fn record(&mut self, timeslot: u8, decode_attempted: bool, decode_success: bool, crc_ok: bool) {
        if !decode_attempted {
            return;
        }

        self.decode_attempted = self.decode_attempted.saturating_add(1);
        if decode_success {
            self.decode_success = self.decode_success.saturating_add(1);
        } else {
            self.false_positive = self.false_positive.saturating_add(1);
        }

        let idx = timeslot.saturating_sub(1).min(3) as usize;
        self.slot_attempted[idx] = self.slot_attempted[idx].saturating_add(1);

        if crc_ok {
            self.crc_ok = self.crc_ok.saturating_add(1);
            self.slot_crc_ok[idx] = self.slot_crc_ok[idx].saturating_add(1);
        }
    }

    pub fn diff_from(&self, baseline: &Self) -> Self {
        let mut slot_attempted = [0u64; 4];
        let mut slot_crc_ok = [0u64; 4];
        for i in 0..4 {
            slot_attempted[i] = self.slot_attempted[i].saturating_sub(baseline.slot_attempted[i]);
            slot_crc_ok[i] = self.slot_crc_ok[i].saturating_sub(baseline.slot_crc_ok[i]);
        }

        Self {
            decode_attempted: self.decode_attempted.saturating_sub(baseline.decode_attempted),
            decode_success: self.decode_success.saturating_sub(baseline.decode_success),
            crc_ok: self.crc_ok.saturating_sub(baseline.crc_ok),
            false_positive: self.false_positive.saturating_sub(baseline.false_positive),
            slot_attempted,
            slot_crc_ok,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Subscriber {
    pub issi: u32,
    // Set of attached GSSIs
    pub attached_groups: HashSet<u32>,
}

/// Centralized subscriber registry tracking locally registered ISSIs and their group affiliations.
#[derive(Debug, Clone)]
pub struct SubscriberRegistry {
    /// Registered ISSIs → Subscriber information
    subscribers: HashMap<u32, Subscriber>,
    /// Set of all GSSIs with at least one local affiliate
    all_attached_groups: HashSet<u32>,
}

impl SubscriberRegistry {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
            all_attached_groups: HashSet::new(),
        }
    }

    pub fn is_registered(&self, issi: u32) -> bool {
        self.subscribers.contains_key(&issi)
    }

    /// Tolerant registration; if ISSI already registered, we overwrite it with a fresh Subscriber struct
    pub fn register(&mut self, issi: u32) {
        self.deregister(issi); // Clean up any existing registration to prevent stale affiliations
        self.subscribers.insert(
            issi,
            Subscriber {
                issi,
                attached_groups: HashSet::new(),
            },
        );
    }

    /// Gets mutable ref to subscriber. If not registered, a default Subscriber is inserted.
    pub fn get_subscriber_mut(&mut self, issi: u32) -> &mut Subscriber {
        self.subscribers.entry(issi).or_insert_with(|| Subscriber {
            issi,
            attached_groups: HashSet::new(),
        })
    }

    /// Deregister an ISSI, removing it from the registry and cleaning up any group affiliations
    pub fn deregister(&mut self, issi: u32) {
        if let Some(subscriber) = self.subscribers.remove(&issi) {
            // Clean up global group affiliations for this subscriber
            for gssi in &subscriber.attached_groups {
                // Check if any other subscriber is still affiliated with this group
                let still_has_members = self.subscribers.values().any(|s| s.attached_groups.contains(gssi));
                if !still_has_members {
                    self.all_attached_groups.remove(gssi);
                }
            }
        }
    }

    /// Add GSSI to subscriber's attached groups and global set
    pub fn affiliate(&mut self, issi: u32, gssi: u32) {
        let subscriber = self.get_subscriber_mut(issi);
        subscriber.attached_groups.insert(gssi);
        self.all_attached_groups.insert(gssi);
    }

    /// Remove GSSI from subscriber's attached groups. Update global set if no more subscribers are affiliated with this GSSI.
    pub fn deaffiliate(&mut self, issi: u32, gssi: u32) {
        let subscriber = self.get_subscriber_mut(issi);
        if subscriber.attached_groups.remove(&gssi) {
            // Check if any other subscriber is still affiliated with this group
            let still_has_members = self.subscribers.values().any(|s| s.attached_groups.contains(&gssi));
            if !still_has_members {
                self.all_attached_groups.remove(&gssi);
            }
        }
    }

    /// Check if any subscriber is affiliated with the given GSSI
    pub fn has_group_members(&self, gssi: u32) -> bool {
        self.all_attached_groups.contains(&gssi)
    }
}

/// Mutable, stack-editable state (mutex-protected).
#[derive(Debug, Clone)]
pub struct StackState {
    pub timeslot_alloc: TimeslotAllocator,
    /// Backhaul/network connection to SwMI (e.g., Brew/TetraPack). False -> fallback mode.
    pub network_connected: bool,
    /// Explicit CLI-enabled autonomous RX gain test mode.
    pub rx_gain_test_mode: bool,
    /// Cross-entity decode/CRC counters used by autonomous RX gain sweep ranking.
    pub rx_gain_decode_counters: RxGainDecodeCounters,
    /// Centralized subscriber registry for local-first routing decisions.
    pub subscribers: SubscriberRegistry,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_deregister() {
        let mut reg = SubscriberRegistry::new();
        assert!(!reg.is_registered(1001));
        reg.register(1001);
        assert!(reg.is_registered(1001));
        reg.deregister(1001);
        assert!(!reg.is_registered(1001));
    }

    #[test]
    fn test_affiliate_deaffiliate() {
        let mut reg = SubscriberRegistry::new();
        reg.register(1001);
        reg.affiliate(1001, 91);
        assert!(reg.has_group_members(91));
        reg.deaffiliate(1001, 91);
        assert!(!reg.has_group_members(91));
    }

    #[test]
    fn test_has_group_members() {
        let mut reg = SubscriberRegistry::new();
        reg.register(1001);
        reg.register(1002);
        reg.register(1003);
        reg.affiliate(1001, 100);
        reg.affiliate(1002, 100);
        reg.affiliate(1003, 100);
        assert!(reg.has_group_members(100));

        // Deaffiliate one, should still have members
        reg.deaffiliate(1001, 100);
        assert!(reg.has_group_members(100));

        // Deregister a user, should still have members
        reg.deregister(1002);
        assert!(reg.has_group_members(100));

        // Deregister last user, should have no members
        reg.deregister(1003);
        assert!(!reg.has_group_members(100));
    }

    #[test]
    fn test_has_group_members_empty() {
        let reg = SubscriberRegistry::new();
        assert!(!reg.has_group_members(999));
    }

    #[test]
    fn test_register_overwrites_existing_subscriber() {
        let mut reg = SubscriberRegistry::new();
        reg.register(1001);
        reg.affiliate(1001, 91);
        assert!(reg.has_group_members(91));

        reg.register(1001);

        assert!(reg.is_registered(1001));
        reg.deaffiliate(1001, 91);
        assert!(!reg.has_group_members(91));
    }
}

impl Default for StackState {
    fn default() -> Self {
        Self {
            timeslot_alloc: TimeslotAllocator::default(),
            network_connected: false,
            rx_gain_test_mode: false,
            rx_gain_decode_counters: RxGainDecodeCounters::default(),
            subscribers: SubscriberRegistry::new(),
        }
    }
}
