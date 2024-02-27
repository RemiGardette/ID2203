# ID2203

## Project: Distributed Durability for an MMO Game

In this project, we have to build the distributed durability layer for SpacetimeDB, a database that is powering BitCraft, an in-development MMO game. 

### Aim

Run the DataStore in a replicated Manner

### Points to keep in mind while designing

- When do follower nodes apply their transactions?
- How should DataStore react to failures and leader changes
- Application is a game where ** the latency for committing transactions locally is critical and prioritized over consistency **

### Task
- [] Implement OmniPaxosDurability struct provided in the skeleton code
- [] Implement the Node struct using the provided Data Store and OmniPaxosDurability
- [] Use an async runtime such as Tokio to spawn multiple instances of Node that together form the Datastore RSM running on OmniPaxos
- [] Show that the system can pass the described test cases in the repository