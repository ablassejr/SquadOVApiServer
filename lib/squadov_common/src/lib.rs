pub mod error;
pub mod parse;
pub mod hal;
pub mod vod;
pub mod gcp;
pub mod oauth;
pub mod encode;
pub mod sql;
pub mod stats;
pub mod hearthstone;
pub mod blob;
pub mod squad;
pub mod riot;
pub mod job;
pub mod session;
pub mod wow;
pub mod kafka;
pub mod analytics;
pub mod rabbitmq;
pub mod matches;

pub use error::*;
pub use parse::*;
pub use hal::*;
pub use vod::*;
pub use gcp::*;
pub use oauth::*;
pub use encode::*;
pub use sql::*;
pub use blob::*;
pub use squad::*;
pub use riot::*;
pub use job::*;
pub use session::*;
pub use wow::*;
pub use kafka::*;

#[macro_use]
extern crate lazy_static;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/squadov.hearthstone.game_state.rs"));
}