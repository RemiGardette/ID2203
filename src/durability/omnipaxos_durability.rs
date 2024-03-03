use self::example_durability::ExampleDurabilityLayer;

use super::*;
use crate::datastore::example_datastore::Tx;
use crate::datastore::{tx_data::TxData, TxOffset};
use omnipaxos::util::LogEntry as OmniLogEntry;
use omnipaxos::{messages::Message, util::NodeId, OmniPaxos};
use omnipaxos_storage::memory_storage::MemoryStorage;
use omnipaxos::macros::Entry;
use omnipaxos::storage::Entry;
use omnipaxos_storage::persistent_storage::PersistentStorage;
use omnipaxos::storage::Snapshot;
use omnipaxos::storage::NoSnapshot;

#[derive(Debug, Clone, Entry)]
struct LogEntry{
    tx_offset: TxOffset,
    tx_data: TxData,
}

/// OmniPaxosDurability is a OmniPaxos node that should provide the replicated
/// implementation of the DurabilityLayer trait required by the Datastore.
pub struct OmniPaxosDurability {
    omni_paxos: OmniPaxos<LogEntry, MemoryStorage<LogEntry>>
}

impl DurabilityLayer for OmniPaxosDurability {

    //We read all the entries from the omnipaxos log and return an iterator over them
    fn iter(&self) -> Box<dyn Iterator<Item = (TxOffset, TxData)>> {
        if let Some(entries) = &self.omni_paxos.read_entries(..) {
            let entry_iter = entries.iter().flat_map(|entry| {
                match entry {
                    //We read both the decided logs and undecided logs, maybe this behaviour will need to be adjusted later
                    OmniLogEntry::Decided(log) | OmniLogEntry::Undecided(log) => {
                        Some((log.tx_offset.clone(), log.tx_data.clone()))
                    }
                    _ => None,
                }
            });
            Box::new(entry_iter.collect::<Vec<_>>().into_iter())
        } else {
            Box::new(std::iter::empty())
        }
    }

    //Same logic as iter, but we filter out the entries with an offset stricly lower than the one provided
    fn iter_starting_from_offset(
        &self,
        offset: TxOffset,
    ) -> Box<dyn Iterator<Item = (TxOffset, TxData)>> {
        if let Some(entries) = &self.omni_paxos.read_entries(..) {
            let entry_iter = entries.iter().filter_map(|entry| {
                match entry {
                    //We read both the decided logs and undecided logs, maybe this behaviour will need to be adjusted later
                    OmniLogEntry::Decided(log) | OmniLogEntry::Undecided(log) => {
                        if log.tx_offset >= offset {
                            Some((log.tx_offset.clone(), log.tx_data.clone()))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            });
            Box::new(entry_iter.collect::<Vec<_>>().into_iter())
        } else {
            Box::new(std::iter::empty())
        }
    }

    fn append_tx(&mut self, tx_offset: TxOffset, tx_data: TxData) {
        let log = LogEntry {
            tx_offset,
            tx_data,
        };
        let _ = self.omni_paxos.append(log);
    }
    
    // This is a function to decide the transaction durable offset
    // We ask the omnipaxos to return the decided index
    fn get_durable_tx_offset(&self) -> TxOffset {
        let decided_index : u64 = self.omni_paxos.get_decided_idx();
        TxOffset(decided_index)
    }
}