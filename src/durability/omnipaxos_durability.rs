use self::example_durability::ExampleDurabilityLayer;

use super::*;
use crate::datastore::{tx_data::TxData, TxOffset};
use omnipaxos::errors::ConfigError;
use omnipaxos::util::LogEntry;
use omnipaxos::ProposeErr;
use omnipaxos::{messages::Message, util::NodeId, OmniPaxos};
use omnipaxos_storage::memory_storage::MemoryStorage;
use omnipaxos::macros::Entry;
use omnipaxos::ClusterConfig;
use omnipaxos::ServerConfig;




#[derive(Debug, Clone, Entry)]
pub struct Log{
    tx_offset: TxOffset,
    tx_data: TxData,
}

/// OmniPaxosDurability is a OmniPaxos node that should provide the replicated
/// implementation of the DurabilityLayer trait required by the Datastore.
pub struct OmniPaxosDurability {
    pub omni_paxos: OmniPaxos<Log, MemoryStorage<Log>>
}

impl OmniPaxosDurability {
    pub fn new(server_config:ServerConfig,cluster_config:ClusterConfig) -> Result<OmniPaxosDurability, ConfigError>  {
        // Create a new instance of OmniPaxos
        let cluster_config = cluster_config.build_for_server::<Log, MemoryStorage<Log>>(
            server_config,
            MemoryStorage::default(),
        );
        match cluster_config {
            Ok(cluster_config) => {
                // OmniPaxos instance created successfully, use it here
                return Ok(OmniPaxosDurability {
                    omni_paxos: cluster_config
                });
            }
            Err(err) => {
                // Handle the ConfigError here
                println!("Error creating OmniPaxos instance: {:?}", err);
                return Err(err);
            }
        }
    }
}

impl DurabilityLayer for OmniPaxosDurability {

    //We read all the entries from the omnipaxos log and return an iterator over them
    fn iter(&self) -> Box<dyn Iterator<Item = (TxOffset, TxData)>> {
        if let Some(entries) = &self.omni_paxos.read_entries(..) {
            let entry_iter = entries.iter().flat_map(|entry| {
                match entry {
                    //We read both the decided logs and undecided logs, maybe this behaviour will need to be adjusted later
                    LogEntry::Decided(log) | LogEntry::Undecided(log) => {
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
                    LogEntry::Decided(log) | LogEntry::Undecided(log) => {
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
        let log = Log {
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