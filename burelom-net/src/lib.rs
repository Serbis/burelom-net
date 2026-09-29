extern crate alloc;

pub mod burelom_node;
pub mod traits;
pub mod cobs;
pub mod routing_table;
pub mod roles;
pub mod node_registry;
mod packetizer;
mod handlers;
mod action_api;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/packet.rs"));
}

pub(crate) mod logging {
    #[cfg(feature = "defmt")]
    pub use defmt::{debug, error, info, trace, warn};

    #[cfg(all(feature = "log", not(feature = "defmt")))]
    pub use log::{debug, error, info, trace, warn};
}
