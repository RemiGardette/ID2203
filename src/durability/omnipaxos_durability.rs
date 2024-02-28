use omnipaxos::util::NodeId;
use std::sync::{Mutex, Arc};
use super::*;

/// OmniPaxosDurability is a OmniPaxos node that should provide the replicated
/// implementation of the DurabilityLayer trait required by the Datastore.
pub struct OmniPaxosDurability {
    // TODO
    id: NodeId,
    leader: NodeId,
    peers: Vec<NodeId>,
    txoffset: TxOffset,
    durable_txoffset: TxOffset,
    data: RwLock<Vec<(TxOffset, TxData)>>,
}

impl DurabilityLayer for OmniPaxosDurability {
    fn iter(&self) -> Box<dyn Iterator<Item = (TxOffset, TxData)>> {
        todo!()
    }

    fn iter_starting_from_offset(
        &self,
        offset: TxOffset,
    ) -> Box<dyn Iterator<Item = (TxOffset, TxData)>> {
        let data = self.data.read().unwrap();
        let filtered_data = data.iter().filter(move |&(tx_offset, _)| *tx_offset >= offset);
        Box::new(filtered_data.map(|&(tx_offset, ref tx_data)| (tx_offset, tx_data.clone())))
}

    }

    fn append_tx(&mut self, tx_offset: TxOffset, tx_data: TxData) {
        todo!()
    }

    fn get_durable_tx_offset(&self) -> TxOffset {
        todo!()
    }
}
