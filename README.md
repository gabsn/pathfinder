# StarkNet

A StarkNet full node written in Rust.

This project is a work-in-progress and is not yet usable.

A first release will be made with the completion of [Milestone I](#milestone-i).

## Roadmap

The end goal is to have a node which

- holds the full StarkNet state
- synchronises StarkNet state from both L1 and L2 (p2p)
- verifies L2 state against L1
- provides an API for interacting with StarkNet state
- participates in the L2 StarkNet network
  - propagating state
  - propagating transactions

The roadmap has been split into milestones, with goals in the later milestones being less certain and well-defined.

### Milestone I

A node which has no p2p capabilities. It synchronises network state using L1 and L2 (StarkNet gateway), and provides an HTTP RPC API.

- [x] retrieve state updates from L1
  - [x] state root
  - [x] contract deployments
  - [x] contract updates
- [x] retrieve state from StarkNet sequencer gateway
  - [x] blocks
  - [x] transactions
  - [x] contract code
- [x] serve RPC API
- [ ] storage
  - [ ] global state
  - [x] contract definitions
  - [x] transactions
  - [x] blocks
- [x] basic user configuration
- [ ] sync state from L1 and L2
- [ ] run `starknet_call` locally
- [ ] validate contract code against L1
- [ ] integrate various components
- [ ] documentation

### Milestone II

Establish p2p network, state is now propagated between nodes.

Add support for syncing completely from L1.

### Milestone III

Create a transaction mempool, transactions are now propagated between nodes.

Add contract calls to RPC API: `invoke` and `deploy`.
