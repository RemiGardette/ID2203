# ID2203

## Project: Distributed Durability for an MMO Game

In this project, we have to build the distributed durability layer for SpacetimeDB, a database that is powering BitCraft, an in-development MMO game. The repo currently has a minimal DataStore and an example implementation of the Durability trait that writes
the transaction log to a file on disk.

### Task
The task is to implement a distributed version of the Durability trait that replicates the transaction log on multiple nodes using OmniPaxos. Running the DataStore in a replicated manner poses a whole new set of challenges: When do follower nodes apply their transactions? How should the DataStore react to failures and leader changes? Furthermore, there is also a need to consider that the application is a game where the latency for committing transactions locally (e.g., player movements) is critical and even
prioritized over consistency.