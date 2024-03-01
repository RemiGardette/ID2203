use super::log_entry::LogEntry;
use super::*;
use crate::datastore::example_datastore::Tx;
use crate::datastore::{tx_data::TxData, TxOffset};
use omnipaxos::util::LogEntry as Omni_LogEntry;
use omnipaxos::{messages::Message, util::NodeId, OmniPaxos};
use omnipaxos_storage::memory_storage::MemoryStorage;
/// OmniPaxosDurability is a OmniPaxos node that should provide the replicated
/// implementation of the DurabilityLayer trait required by the Datastore.
pub struct OmniPaxosDurability {
    pub omni_paxos: OmniPaxos<LogEntry, MemoryStorage<LogEntry>>
}

impl DurabilityLayer for OmniPaxosDurability {
    fn iter(&self) -> Box<dyn Iterator<Item = (TxOffset, TxData)>> {
        todo!()
    }

    fn iter_starting_from_offset(
        &self,
        offset: TxOffset,
    ) -> Box<dyn Iterator<Item = (TxOffset, TxData)>> {
        todo!()
    }

    fn append_tx(&mut self, tx_offset: TxOffset, tx_data: TxData) {
        todo!()
    }
    
    // This is a function to decide the transaction durable offset
    // We ask the omnipaxos to return the decided index
    fn get_durable_tx_offset(&self) -> TxOffset {
        let decided_index : u64 = self.omni_paxos.get_decided_idx();
        TxOffset(decided_index)
    }
}