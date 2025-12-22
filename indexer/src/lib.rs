pub mod cli;
pub mod config;
pub mod telemetry;
pub mod domain {
    pub mod events;
    pub mod indexer;
    pub mod resolvers;
}
pub mod relay {
    pub mod event;
    pub mod record;
}
pub mod utils;
mod runner;

#[cfg(feature = "audit")]
pub mod audit;

#[cfg(not(feature = "audit"))]
pub mod audit {
    use radroots_events::{
        comment::RadrootsCommentEventIndex, listing::RadrootsListingEventIndex,
        profile::RadrootsProfileEventIndex,
    };
    use crate::domain::resolvers::profile::ProfileResolver;

    pub fn log_indexer_event(_: &crate::relay::event::RelayIndexerEvent) {}
    pub fn log_profile_event(_: &RadrootsProfileEventIndex) {}
    pub fn log_listing_event(_: &RadrootsListingEventIndex) {}
    pub fn log_comment_event(_: &RadrootsCommentEventIndex) {}
    pub fn set_profile_resolver(_: ProfileResolver) {}
}
pub use config::Settings;
pub use relay::record::RelayEventRecord;
pub use runner::run;
