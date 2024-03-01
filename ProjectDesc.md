## 2.1 Distributed Durability for an MMO Game

In this project, you will build the distributed durability layer for SpacetimeDB, a
database that is powering BitCraft, an in-development MMO game. You will be given a
minimal DataStore and an example implementation of the Durability trait that writes
the transaction log to a file on disk.
## 

Your task is to implement a distributed version of the Durability trait that replicates
the transaction log on multiple nodes using OmniPaxos. Running the DataStore in a
replicated manner poses a whole new set of challenges: When do follower nodes apply
their transactions? How should the DataStore react to failures and leader changes?
Furthermore, you will also need to consider that the application is a game where the
latency for committing transactions locally (e.g., player movements) is critical and even
prioritized over consistency.
### 2.1.1 Requirements
Basic Requirements (30p):
#### • Implement the OmniPaxosDurability struct provided in the skeleton code.
#### • Implement the Node struct using the provided Datastore and your OmniPaxosDurability.
#### • Use an async runtime such as Tokio to spawn multiple instances of Node that together form the Datastore RSM running on OmniPaxos. Your code must run different nodes on multiple threads (or VM instances) and perform messaging passing via local channels or over the network.
#### • Implement and show that your system can pass the described test cases in the repository.
### Bonus Tasks (10p): Pick one of the following bonus tasks.
#### • Sharding and Atomic Commit: Currently, the whole datastore is replicated on OmniPaxos. This is not a scalable solution. To make the system scale and increase parallelism, run multiple instances of OmniPaxos that are each responsible for a subset of keys. For full points, either (a) make the sharding scheme automatic or (b) use a static sharding scheme and add support for atomic commit across shards.
#### • Introducing MemoryDecided: A decided entry in OmniPaxos has the dura-bility guarantees according to the storage implementation. However, in practice,2 we might want more fine-grained control. In this task, modify the OmniPaxos library so that we can differentiate an entry that has only been decided in memory yet and an entry that is decided on disk. This should also be reflected when we read, i.e., a LogEntry can be either MemoryDecided or (completely) Decided. Use your newly developed feature and modify the DataStore to also support reading DurabilityLevel::MemoryReplicated.
#### • Learning Data with UniCache: The latest feature of OmniPaxos is UniCache,a novel data-driven approach to consensus. UniCache enables OmniPaxos to learnfrom the history in the log and cache popular data objects so that they can becompressed in future entries. Find a dataset with skewed distribution. Use theunicache feature in OmniPaxos and show in a benchmark how it can improve theperformance of the system.
Category: Distributed databases
Language and libraries: Rust, OmniPaxos, Tokio
Repository: https://gits-15.sys.kth.se/hng/OmniSpacetimeDB