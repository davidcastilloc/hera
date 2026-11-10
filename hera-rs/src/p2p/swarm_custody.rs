//! Federated R2 Swarm Orchestrator, Constrained Water-Filling, Power-of-Two-Choices (`P2C`)
//! 60s Signed Tickets, 64-Byte `Proof-of-Custody` Range Audits, Anti-Poisoning Quarantine,
//! MoQ Harmonic Topic Pub/Sub, and `Delta-CRDT` State Convergence (`src/p2p/swarm_custody.rs`).

mod crdt_pubsub;
mod custody_audit;
mod heartbeat;
mod merkle_delivery;
mod routing_tickets;
mod water_filling;
