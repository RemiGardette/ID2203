use self::example_durability::ExampleDurabilityLayer;

use super::*;
use crate::datastore::{tx_data::TxData, TxOffset};
use omnipaxos::util::LogEntry as OmniLogEntry;
use omnipaxos::ProposeErr;
use omnipaxos::{messages::Message, util::NodeId, OmniPaxos};
use omnipaxos_storage::memory_storage::MemoryStorage;
use omnipaxos::macros::Entry;
use crate::sequence_paxos::SequencePaxos; // Add the missing import statement
use omnipaxos::ballot_leader_election::BallotLeaderElection; // Add the missing import statement

#[derive(Debug, Clone, Entry)]
pub struct LogEntry{
    tx_offset: TxOffset,
    tx_data: TxData,
}

/// OmniPaxosDurability is a OmniPaxos node that should provide the replicated
/// implementation of the DurabilityLayer trait required by the Datastore.
pub struct OmniPaxosDurability {
    pub omni_paxos: OmniPaxos<LogEntry, MemoryStorage<LogEntry>>
}

<<<<<<< HEAD
impl OmniPaxosDurability {
    pub fn new(omnipaxos: OmniPaxos<LogEntry, MemoryStorage<LogEntry>>) -> Self {
        OmniPaxosDurability { omni_paxos: omnipaxos}
=======
fn new() -> OmniPaxosDurability  {
    // Create a new instance of OmniPaxos
    let omnipaxos = OmniPaxos {
        seq_paxos: SequencePaxos<T, B>,
        ble: BallotLeaderElection,
        election_clock: LogicalClock,
        resend_message_clock: LogicalClock::new(OUTGOING_MESSAGE_PERIOD),
    };
    return OmniPaxosDurability {
        omni_paxos: omnipaxos
>>>>>>> 375aa99d5eb7037a1983076f55c3d9b7ee392fe8
    }
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
        match self.omni_paxos.append(log) {
            Ok(v) => println!("Entry appended to omnipaxos {v:?}"),
            Err(e) => println!("Error appending entry to omnipaxos: {e:?}"),
        }

    }
    
    // This is a function to decide the transaction durable offset
    // We ask the omnipaxos to return the decided index
    fn get_durable_tx_offset(&self) -> TxOffset {
        let decided_index : u64 = self.omni_paxos.get_decided_idx();
        TxOffset(decided_index)
    }
}