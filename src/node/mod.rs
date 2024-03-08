use crate::datastore::error::DatastoreError;
use crate::datastore::example_datastore::ExampleDatastore;
use crate::datastore::tx_data::TxResult;
use crate::datastore::*;
use crate::durability::omnipaxos_durability::OmniPaxosDurability;
use crate::durability::{DurabilityLayer, DurabilityLevel};
use std::time::Duration;
use crate::durability::omnipaxos_durability::Log;
use omnipaxos::messages::*;
use omnipaxos::util::NodeId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::{sync::mpsc, time};

use self::example_datastore::MutTx;

const BUFFER_SIZE: usize = 10000;
const ELECTION_TICK_TIMEOUT: u64 = 5;
const TICK_PERIOD: Duration = Duration::from_millis(10);
const OUTGOING_MESSAGE_PERIOD: Duration = Duration::from_millis(1);
const WAIT_LEADER_TIMEOUT: Duration = Duration::from_millis(500);
const UI_TICK_PERIOD: Duration = Duration::from_millis(100);
const BATCH_SIZE: u64 = 100;
const BATCH_PERIOD: Duration = Duration::from_millis(50);
const DATASTORE_CATCHUP_PERIOD: Duration = Duration::from_millis(10000);


pub struct NodeRunner {
    pub node: Arc<Mutex<Node>>,
    //Add the Messaging and running which is used for msging between the servers and for BLE
    pub incoming: mpsc::Receiver<Message<Log>>,
    pub outgoing: HashMap<NodeId, mpsc::Sender<Message<Log>>>,
}

impl NodeRunner {
    async fn send_outgoing_msgs(&mut self) {
        // let messages = self.omni_paxos.lock().unwrap().outgoing_messages();
        let messages = self.node.lock().unwrap().durability.omni_paxos.outgoing_messages();
        for msg in messages {
            let receiver = msg.get_receiver();
            // println!("Sending message to {}", receiver);
            if self.node.lock().unwrap().connected_nodes[receiver as usize-1] == true
            {
            let channel = self
                .outgoing
                .get_mut(&receiver)
                .expect("No channel for receiver");
            let _ = channel.send(msg).await;
            }
        }
    }

    pub async fn run(&mut self) {
        let mut outgoing_interval = time::interval(OUTGOING_MESSAGE_PERIOD);
        let mut tick_interval = time::interval(TICK_PERIOD);
        let mut datastore_update_interval = time::interval(DATASTORE_CATCHUP_PERIOD);
        let mut leader_time_out = time::interval(WAIT_LEADER_TIMEOUT);
        loop {
            tokio::select! {
                biased;
                _ = tick_interval.tick() => { 
                    self.node.lock().unwrap().durability.omni_paxos.tick(); 
                },
                _ = outgoing_interval.tick() => { 
                    if self.node.lock().unwrap().messaging_allowed{
                    self.send_outgoing_msgs().await;}
                 },
                _ = datastore_update_interval.tick() => { 
                    // We update the datastore on every follower node at a regular interval, to avoid having to catch up on too much content at once when the leader changes
                    let current_leader_pid = self.node.lock().unwrap().durability.omni_paxos.get_current_leader();
                    if current_leader_pid != Some(self.node.lock().unwrap().node_id) {
                        self.node.lock().unwrap().apply_replicated_txns();
                    }

                },
                _ = leader_time_out.tick() => {
                    self.node.lock().unwrap().update_leader();
                },
                Some(in_msg) = self.incoming.recv() => { if self.node.lock().unwrap().messaging_allowed{
                    self.node.lock().unwrap().durability.omni_paxos.handle_incoming(in_msg);}
                },
                else => { }
            }
        }
    }
    
}

pub struct Node {
    node_id: NodeId, // Unique identifier for the node
    durability: OmniPaxosDurability,        
    datastore: ExampleDatastore,        // TODO Datastore and OmniPaxosDurability
    messaging_allowed: bool,
    connected_nodes: Vec<bool>
}

impl Node {
    pub fn new(node_id: NodeId, omni_durability: OmniPaxosDurability, number_of_nodes: u64) -> Self {
        let node = Node {
            node_id,
            durability: omni_durability,
            datastore: ExampleDatastore::new(),
            messaging_allowed: true,
            connected_nodes: vec![true; number_of_nodes as usize],
        };
        return node
    }

    /// update who is the current leader. If a follower becomes the leader,
    /// it needs to apply any unapplied txns to its datastore.
    /// If a node loses leadership, it needs to rollback the txns committed in
    /// memory that have not been replicated yet.
    pub fn update_leader(&mut self) {
        let leader_id = self.durability.omni_paxos.get_current_leader();
        if leader_id == Some(self.node_id) {
            self.apply_replicated_txns();
        } else {
            self.rollback_unreplicated_txns();
        }
    }


    /// Apply the transactions that have been decided in OmniPaxos to the Datastore.
    /// We need to be careful with which nodes should do this according to desired
    /// behavior in the Datastore as defined by the application.
    fn apply_replicated_txns(&mut self) {
        self.advance_replicated_durability_offset().unwrap();
        let currentTxOffset = self.durability.get_durable_tx_offset();
        let mut txns = self.durability.iter_starting_from_offset(currentTxOffset);
        while let Some((offset, tx_data)) = txns.next() {
            self.datastore.replay_transaction(&tx_data);
        }
    }


    fn rollback_unreplicated_txns(&mut self) {
        self.advance_replicated_durability_offset().unwrap();
        if let Err(err) = self.datastore.rollback_to_replicated_durability_offset() {
            println!("Error: {}", err);
        }
    }

    pub fn begin_tx(
        &self,
        durability_level: DurabilityLevel,
    ) -> <ExampleDatastore as Datastore<String, String>>::Tx {
        self.datastore.begin_tx(durability_level)
    }

    pub fn release_tx(&self, tx: <ExampleDatastore as Datastore<String, String>>::Tx) {
        self.datastore.release_tx(tx);
    }

    /// Begins a mutable transaction. Only the leader is allowed to do so.
    pub fn begin_mut_tx(
        &self,
    ) -> Result<<ExampleDatastore as Datastore<String, String>>::MutTx, DatastoreError> {
        let leader_id = self.durability.omni_paxos.get_current_leader();
        if leader_id == Some(self.node_id) {
            Ok(self.datastore.begin_mut_tx())
        } else {
            Err(DatastoreError::NotLeader)
        }   
    }

    /// Commits a mutable transaction. Only the leader is allowed to do so.
    pub fn commit_mut_tx(
        &mut self,
        tx: <ExampleDatastore as Datastore<String, String>>::MutTx,
    ) -> Result<TxResult, DatastoreError> {
        let leader_id = self.durability.omni_paxos.get_current_leader();
        if leader_id == Some(self.node_id) {
            self.datastore.commit_mut_tx(tx).map_err(|err| err.into())
        } else {
            Err(DatastoreError::NotLeader)
        }   
    }

    fn advance_replicated_durability_offset(
        &self,
    ) -> Result<(), crate::datastore::error::DatastoreError> {
        let tx_offset = self.durability.get_durable_tx_offset();
        self.datastore.advance_replicated_durability_offset(tx_offset)
    }
}

/// Your test cases should spawn up multiple nodes in tokio and cover the following:
/// 1. Find the leader and commit a transaction. Show that the transaction is really *chosen* (according to our definition in Paxos) among the nodes.
/// 2. Find the leader and commit a transaction. Kill the leader and show that another node will be elected and that the replicated state is still correct.
/// 3. Find the leader and commit a transaction. Disconnect the leader from the other nodes and continue to commit transactions before the OmniPaxos election timeout.
/// Verify that the transaction was first committed in memory but later rolled back.
/// 4. Simulate the 3 partial connectivity scenarios from the OmniPaxos liveness lecture. Does the system recover? *NOTE* for this test you may need to modify the messaging logic.
///
/// A few helper functions to help structure your tests have been defined that you are welcome to use.
#[cfg(test)]
mod tests {
    use crate::{durability, node::*};
    use omnipaxos::messages::Message;
    use omnipaxos::util::{NodeId};
    use omnipaxos::{ClusterConfig, OmniPaxosConfig, ServerConfig};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use omnipaxos_storage::memory_storage::MemoryStorage;
    use tokio::runtime::{Builder, Runtime};
    use tokio::sync::mpsc;
    use tokio::task::JoinHandle;
    use self::durability::omnipaxos_durability::Log;

    const SERVERS: [NodeId; 5] = [1, 2, 3, 4, 5];

    #[allow(clippy::type_complexity)]
    fn initialise_channels() -> (
        HashMap<NodeId, mpsc::Sender<Message<Log>>>,
        HashMap<NodeId, mpsc::Receiver<Message<Log>>>,
    ) {
        let mut sender_channels = HashMap::new();
        let mut receiver_channels = HashMap::new();
        for pid in SERVERS {
            let (sender, receiver) = mpsc::channel(100);
            sender_channels.insert(pid, sender);
            receiver_channels.insert(pid, receiver);
        }
        (sender_channels, receiver_channels)
    }

    fn create_runtime() -> Runtime {
        Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap()
    }

    fn spawn_nodes(runtime: &mut Runtime) -> HashMap<NodeId, (Arc<Mutex<Node>>, JoinHandle<()>)> {
        let mut nodes = HashMap::new();
        let (sender_channels, mut receiver_channels) = initialise_channels();
        for pid in SERVERS {
            // todo!("spawn the nodes")
            let server_config = ServerConfig{
                pid,
                election_tick_timeout: ELECTION_TICK_TIMEOUT,
                ..Default::default()
            };
            let cluster_config = ClusterConfig{
                configuration_id: 1,
                nodes: SERVERS.into(),
                ..Default::default()
            };
            let durability = OmniPaxosDurability::new(server_config.clone(), cluster_config.clone()).unwrap();
            let node: Arc<Mutex<Node>> = Arc::new(Mutex::new(Node::new(pid, durability, SERVERS.len() as u64)));
            let mut node_runner = NodeRunner {
                node: node.clone(),
                incoming: receiver_channels.remove(&pid).unwrap(),
                outgoing: sender_channels.clone(),
            };
            let join_handle = runtime.spawn({
                async move {
                    node_runner.run().await;
                }
            });
            nodes.insert(pid, ( node, join_handle));
        }
        nodes
    }

    //#[test]
    fn basic_test_cluster_size() {
        let mut runtime = create_runtime();
        let nodes = spawn_nodes(&mut runtime);
        std::thread::sleep(WAIT_LEADER_TIMEOUT);
        assert_eq!(SERVERS.len(), nodes.len());
    }

    //#[test]
    //TestCase #1 Find the leader and commit a transaction. Show that the transaction is really *chosen* (according to our definition in Paxos) among the nodes
    /*fn test_case_1() {
        let mut runtime = create_runtime();
        let nodes = spawn_nodes(&mut runtime);
        std::thread::sleep(WAIT_LEADER_TIMEOUT);
        let (first_server, _) = nodes.get(&1).unwrap();
        let leader_pid = first_server
             .lock()
             .unwrap()
             .durability.omni_paxos
             .get_current_leader()
             .expect("Failed to get leader");

        let leader = nodes.get(&leader_pid).unwrap();
        let mut tx = leader.0.lock().unwrap().begin_mut_tx().unwrap();
        leader.0.lock().unwrap().datastore.set_mut_tx(&mut tx, "key1".to_string(), "value1".to_string());
        let transaction = leader.0.lock().unwrap().commit_mut_tx(tx).unwrap();
        leader.0.lock().unwrap().durability.append_tx(transaction.tx_offset, transaction.tx_data);
        // After committing the transaction, check the leader status
        std::thread::sleep(TICK_PERIOD);
        let leader_iter = leader.0.lock().unwrap().durability.iter();
        let leader_offset = leader.0.lock().unwrap().durability.get_durable_tx_offset();
        let leader_collected: Vec<_> = leader_iter.collect();
        //check that follower nodes are in sync with the leader
        for pid in SERVERS {
            let (server, _) = nodes.get(&pid).unwrap();
            if server.lock().unwrap().node_id != leader_pid {
                let iter = server.lock().unwrap().durability.iter();
                let collected: Vec<_> = iter.collect();
                assert_eq!(collected.len(), leader_collected.len());
                assert_eq!(leader_offset, server.lock().unwrap().durability.get_durable_tx_offset());
            }
        
        //Here we will assert that a node which is not a leader cannot commit

        }
    }
    

    //#[test]
    /// 2. Find the leader and commit a transaction. Kill the leader and show that another node will be elected and that the replicated state is still correct.
    fn test_case_2() {
        let mut runtime = create_runtime();
        let nodes = spawn_nodes(&mut runtime);
        std::thread::sleep(WAIT_LEADER_TIMEOUT);
        let (first_server, _) = nodes.get(&1).unwrap();
        let leader_pid = first_server
             .lock()
             .unwrap()
             .durability.omni_paxos
             .get_current_leader()
             .expect("Failed to get leader");
        
         println!("Elected leader: {}", leader_pid);
         println!("Current Offset: {}", first_server.lock().unwrap().durability.get_durable_tx_offset().0);

        let leader = nodes.get(&leader_pid).unwrap();
        let mut tx = leader.0.lock().unwrap().begin_mut_tx().unwrap();
        leader.0.lock().unwrap().datastore.set_mut_tx(&mut tx, "key1".to_string(), "value1".to_string());
        let transaction = leader.0.lock().unwrap().commit_mut_tx(tx).unwrap();
        leader.0.lock().unwrap().durability.append_tx(transaction.tx_offset, transaction.tx_data);
        // After committing the transaction, check the leader status
        std::thread::sleep(TICK_PERIOD);
        let leader_iter = leader.0.lock().unwrap().durability.iter();
        let leader_offset = leader.0.lock().unwrap().durability.get_durable_tx_offset();
        let leader_collected: Vec<_> = leader_iter.collect();
        leader.1.abort();
        std::thread::sleep(WAIT_LEADER_TIMEOUT);
        let (second, _) = nodes.get(&2).unwrap();
        let new_leader = second
             .lock()
             .unwrap()
             .durability.omni_paxos
             .get_current_leader()
             .expect("Failed to get leader");
        assert_ne!(leader_pid, new_leader);
        println!("New leader: {}", new_leader);
        let new_leader_offset = second.lock().unwrap().durability.get_durable_tx_offset();
        let new_leader_iter = second.lock().unwrap().durability.iter();
        let new_leader_collected: Vec<_> = new_leader_iter.collect();
        // print the collected 
        println!("Leader Collected: {:?}", leader_collected);
        println!("Leader Collected: {:?}", new_leader_collected);
        assert_eq!(leader_collected.len(), new_leader_collected.len());

    }*/

    /// 3. Find the leader and commit a transaction. Disconnect the leader from the other nodes and continue to commit transactions before the OmniPaxos election timeout.
    /// Verify that the transaction was first committed in memory but later rolled back.
    #[test]
/// 3. Find the leader and commit a transaction. Disconnect the leader from the other nodes and continue to commit transactions before the OmniPaxos election timeout.
/// Verify that the transaction was first committed in memory but later rolled back.
// fn test_case_3() {
//     let mut runtime = create_runtime();
//     let nodes = spawn_nodes(&mut runtime);
//     std::thread::sleep(WAIT_LEADER_TIMEOUT);
//     let (first_server, _) = nodes.get(&1).unwrap();
//     let leader_pid = first_server
//         .lock()
//         .unwrap()
//         .durability
//         .omni_paxos
//         .get_current_leader()
//         .expect("Failed to get leader");

//     println!("Elected leader: {}", leader_pid);
//     println!(
//         "Current Offset: {}",
//         first_server.lock().unwrap().durability.get_durable_tx_offset().0
//     );

//     // Assuming you have already obtained the leader_pid and nodes HashMap

// let leader = nodes.get(&leader_pid).unwrap();

// for _ in 0..5 {
//     let mut tx = leader.0.lock().unwrap().begin_mut_tx().unwrap();
//     leader
//         .0
//         .lock()
//         .unwrap()
//         .datastore
//         .set_mut_tx(&mut tx, "key1".to_string(), "value1".to_string());
//     let transaction = leader
//         .0
//         .lock()
//         .unwrap()
//         .commit_mut_tx(tx)
//         .unwrap();
//     leader.0.lock().unwrap().durability.append_tx(transaction.tx_offset, transaction.tx_data);
// }
// println!(
//     "Current Offset before cutting connection, should be 0+5=5: {}",
//     leader.0.lock().unwrap().datastore.get_cur_offset().unwrap().0
// );

//     std::thread::sleep(WAIT_LEADER_TIMEOUT);
//     println!("Replicated logs: {:?}", leader.0.lock().unwrap().durability.iter().collect::<Vec<_>>());
//     println!(
//         "test, should be 0+5=5: {}",
//         leader.0.lock().unwrap().datastore.get_cur_offset().unwrap().0
//     );
//     println!(
//         "test, should be 0+5=5: {}",
//         leader.0.lock().unwrap().datastore.get_cur_offset().unwrap().0
//     );
//     println!(
//         "Current durability offset, should be 0+5=5: {}",
//         leader.0.lock().unwrap().durability.get_durable_tx_offset().0
//     );
//     println!(
//         "Current durability datastore offset, should be 0+5=5: {}",
//         leader.0.lock().unwrap().datastore.get_replicated_offset().unwrap().0
//     );
//     // Cutting the connection
//     leader.0.lock().unwrap().messaging_allowed = false;
//     // Adding some commits
//     for _ in 0..5 {
//         let mut tx = leader.0.lock().unwrap().begin_mut_tx().unwrap();
//         leader
//             .0
//             .lock()
//             .unwrap()
//             .datastore
//             .set_mut_tx(&mut tx, "key1".to_string(), "value1".to_string());
//         let transaction = leader
//             .0
//             .lock()
//             .unwrap()
//             .commit_mut_tx(tx)
//             .unwrap();
//     }
//     let value2 = leader.0.lock().unwrap().datastore.get_cur_offset().unwrap().0.to_le();
//     println!(
//         "Current Offset after cutting connection, should be 5+5=10: {}",
//         value2
//     );
//     std::thread::sleep(WAIT_LEADER_TIMEOUT);
//     println!(
//         "test, should be 5+6=11: {}",
//         leader.0.lock().unwrap().datastore.get_cur_offset().unwrap().0
//     );


//     // Simulate waiting for some time (more than WAIT_LEADER_TIMEOUT)
//     std::thread::sleep(WAIT_LEADER_TIMEOUT * 2);

//     // Verify that the transaction was rolled back after the timeout
//     // Verify that the transaction was rolled back after the timeout
//     let leader_first_check = leader
//     .0
//     .lock()
//     .unwrap()
//     .datastore
//     .get_cur_offset()
//     .unwrap()
//     .0
//     .to_le();
//     let (s1, _) = nodes.get(&1).unwrap();
//     let (s2, _) = nodes.get(&2).unwrap();
//     let (s3, _) = nodes.get(&3).unwrap();
//     println!("Current length of the iter is {:?} after timeout", leader_first_check);
//     println!("New leader, AFTER TIMEOUT, for 1: {}", s1.lock().unwrap().durability.omni_paxos.get_current_leader().unwrap());
//     println!("New leader, AFTER TIMEOUT, for old leader: {}", s3.lock().unwrap().durability.omni_paxos.get_current_leader().unwrap());
//     let length_after_timeout = leader_first_check;
//     println!("Current length of the iter is {:?} after timeout", length_after_timeout);
//     leader.0.lock().unwrap().messaging_allowed = true;
    
//     //s1.lock().unwrap().durability.omni_paxos.reconnected(3);
//     //s2.lock().unwrap().durability.omni_paxos.reconnected(3);
//     std::thread::sleep(WAIT_LEADER_TIMEOUT * 2);
//     println!("New leader, AFTER REJOIN, for 1: {}", s1.lock().unwrap().durability.omni_paxos.get_current_leader().unwrap());
//     println!("New leader, AFTER REJOIN, for old leader: {}", s3.lock().unwrap().durability.omni_paxos.get_current_leader().unwrap());
//     let length_after_rejoin = s3.lock().unwrap().datastore.get_cur_offset().unwrap().0.to_le();
//     println!("Current length of the iter is {:?} after rejoin", length_after_rejoin);
//     //s3.lock().unwrap().datastore.rollback_to_replicated_durability_offset().unwrap();
//     //s3.lock().unwrap().update_leader();
//     let length_after_rejoin = s3.lock().unwrap().datastore.get_cur_offset().unwrap().0.to_le();
//     println!("Current length of the iter is {:?} after forced rollback", length_after_rejoin);
//     // Assert that the second commit length is equal to the first check length
//     //assert_eq!(leader_second_commit_len, leader_first_check);

// }


    #[test]
    /// 4. Simulate the 3 partial connectivity scenarios from the OmniPaxos liveness lecture. Does the system recover? *NOTE* for this test you may need to modify the messaging logic.
    /// First Scenario Quorum-Loss Scenario
    fn test_case_4_loss() {
        let mut runtime = create_runtime();
        let nodes = spawn_nodes(&mut runtime);
        std::thread::sleep(WAIT_LEADER_TIMEOUT);
        let (first_server, _) = nodes.get(&1).unwrap();
        let leader = first_server
            .lock()
            .unwrap()
            .durability.omni_paxos
            .get_current_leader()
            .expect("Failed to get leader");
        println!("Elected leader: {}", leader);
        let leader_to_be = 1 as u64;

        for pid in SERVERS {
            if pid != leader_to_be {
                let (server_to_get, _) = nodes.get(&pid).unwrap();
                for pid1 in SERVERS{
                    if pid1 != leader_to_be{
                        server_to_get.lock().unwrap().connected_nodes[pid1 as usize-1] = false;
                    }
                }
            }    
        }

        std::thread::sleep(WAIT_LEADER_TIMEOUT * 2);
        let new_leader = nodes.get(&leader_to_be).unwrap().0.lock().unwrap().durability.omni_paxos.get_current_leader().expect("Failed to get leader");
        println!("Newly elected leader {:?}", new_leader)







    }

    #[test]
    /// 4. Simulate the 3 partial connectivity scenarios from the OmniPaxos liveness lecture. Does the system recover? *NOTE* for this test you may need to modify the messaging logic.
    /// Constrained-Election Scenario
    fn test_case_4_constrained() {
        let mut runtime = create_runtime();
        let nodes = spawn_nodes(&mut runtime);
        std::thread::sleep(WAIT_LEADER_TIMEOUT);
        let (first_server, _) = nodes.get(&1).unwrap();
        // let leader = first_server
        //     .lock()
        //     .unwrap()
        //     .get_current_leader()
        //     .expect("Failed to get leader");
        // println!("Elected leader: {}", leader);
    }
    #[test]
    /// 4. Simulate the 3 partial connectivity scenarios from the OmniPaxos liveness lecture. Does the system recover? *NOTE* for this test you may need to modify the messaging logic.
    /// Chained Scenario 
    fn test_case_4_chained() {
        let mut runtime = create_runtime();
        let nodes = spawn_nodes(&mut runtime);
        std::thread::sleep(WAIT_LEADER_TIMEOUT);
        let (first_server, _) = nodes.get(&1).unwrap();
        // let leader = first_server
        //     .lock()
        //     .unwrap()
        //     .get_current_leader()
        //     .expect("Failed to get leader");
        // println!("Elected leader: {}", leader);
    }




}