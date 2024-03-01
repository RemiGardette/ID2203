use self::example_durability::ExampleDurabilityLayer;

use super::*;
use crate::datastore::example_datastore::Tx;
use crate::datastore::{tx_data::TxData, TxOffset};
use omnipaxos::util::LogEntry as Omni_LogEntry;
use omnipaxos::{messages::Message, util::NodeId, OmniPaxos};
use omnipaxos_storage::memory_storage::MemoryStorage;
use omnipaxos::storage::Entry;
use omnipaxos_storage::persistent_storage::PersistentStorage;
use omnipaxos::storage::Snapshot;
use omnipaxos::storage::NoSnapshot;
use omnipaxos::util::LogEntry;

#[derive(Debug, Clone)]
struct Log{
    tx_offset: TxOffset,
    tx_data: TxData,
}

// Implement the Entry trait for LogEntry
impl Entry for Log{
    #[cfg(not(feature = "serde"))]
    /// The snapshot type for this entry type.
    type Snapshot = NoSnapshot;

    #[cfg(feature = "serde")]
    /// The snapshot type for this entry type.
    type Snapshot: Snapshot<Self> + Serialize + for<'a> Deserialize<'a>;

    #[cfg(feature = "unicache")]
    /// The encoded type of some data. If there is a cache hit in UniCache, the data will be replaced and get sent over the network as this type instead. E.g., if `u8` then the cached `Entry` (or field of it) will be sent as `u8` instead.
    type Encoded: Encoded;
    #[cfg(feature = "unicache")]
    /// The type representing the encodable parts of an `Entry`. It can be set to `Self` if the whole `Entry` is cachable. See docs of `pre_process()` for an example of deriving `Encodable` from an `Entry`.
    type Encodable: Encodable;
    #[cfg(feature = "unicache")]
    /// The type representing the **NOT** encodable parts of an `Entry`. Any `NotEncodable` data will be transmitted in its original form, without encoding. It can be set to `()` if the whole `Entry` is cachable. See docs of `pre_process()` for an example.
    type NotEncodable: NotEncodable;

    #[cfg(all(feature = "unicache", not(feature = "serde")))]
    /// The type that represents if there was a cache hit or miss in UniCache.
    type EncodeResult: Clone + Debug;

    #[cfg(all(feature = "unicache", feature = "serde"))]
    /// The type that represents the results of trying to encode i.e., if there was a cache hit or miss in UniCache.
    type EncodeResult: Clone + Debug + Serialize + for<'a> Deserialize<'a>;

    #[cfg(all(feature = "unicache", not(feature = "serde")))]
    /// The type that represents the results of trying to encode i.e., if there was a cache hit or miss in UniCache.
    type UniCache: UniCache<T = Self>;
    #[cfg(all(feature = "unicache", feature = "serde"))]
    /// The unicache type for caching popular/re-occurring fields of an entry.
    type UniCache: UniCache<T = Self> + Serialize + for<'a> Deserialize<'a>;
}

/// OmniPaxosDurability is a OmniPaxos node that should provide the replicated
/// implementation of the DurabilityLayer trait required by the Datastore.
pub struct OmniPaxosDurability {
    omni_paxos: OmniPaxos<Log, MemoryStorage<Log>>
}

impl DurabilityLayer for OmniPaxosDurability {

    fn iter(&self) -> Box<dyn Iterator<Item = (TxOffset, TxData)>> {
        if let Some(entries) = &self.omni_paxos.read_entries(..) {
            let entry_iter = entries.iter().flat_map(|entry| {
                match entry {
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