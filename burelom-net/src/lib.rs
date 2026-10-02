#![cfg_attr(feature = "embassy", no_std)]
#[allow(warnings)]

extern crate alloc;

pub mod prelude {
    pub use alloc::boxed::Box;
    pub use alloc::vec::Vec;
    pub use alloc::vec;
    pub use alloc::string::String;
    pub use alloc::string;
    pub use alloc::format;
    pub use alloc::string::ToString;
}

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
    pub use defmt::{error, info, warn};

    #[cfg(all(feature = "log", not(feature = "defmt")))]
    pub use log::{error, info, warn};
}
