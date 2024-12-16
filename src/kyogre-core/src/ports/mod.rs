//! Ports defines all possible ways to modify and read the current system state.
//!
//! They are divided into two types:
//! - `Inbound` (new data is added or existing data is modified)
//! - `Outbound` (data is retrieved from the system).
//!
//! Some Ports may contain both inbound and outbound methods where appropriate.

mod inbound;
mod outbound;

pub use inbound::*;
pub use outbound::*;

#[cfg(feature = "test")]
pub trait TestStorage:
    ScraperInboundPort
    + WebApiOutboundPort
    + AisConsumeLoop
    + TripAssemblerOutboundPort
    + TestHelperOutbound
    + TestHelperInbound
    + Send
    + Sync
    + 'static
{
}
