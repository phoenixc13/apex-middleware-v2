//! APEX Memory Governance
//!
//! Memory in APEX is a bounded resource, not an infinite well.
//! Every buffer has an owner, a state, and a maximum lifetime.
//! No queue grows without ceiling. No loan lives forever.
//!
//! Architecture:
//! - Pools are created at startup with a fixed capacity.
//! - Publishers borrow slots from a pool, write into them, then publish.
//! - The runtime passes ownership (or a reference) to subscribers.
//! - Once all subscribers release, the slot is reclaimed.
//! - Orphaned loans (held beyond timeout) are forcibly reclaimed with diagnostic.

pub mod pool;
pub mod loan;
pub mod pressure;

pub use pool::{MemoryPool, PoolConfig, PoolStats};
pub use loan::{BufferLoan, LoanState};
pub use pressure::{MemoryPressure, PressureLevel};
