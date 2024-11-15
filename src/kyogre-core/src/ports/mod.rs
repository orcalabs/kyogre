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
