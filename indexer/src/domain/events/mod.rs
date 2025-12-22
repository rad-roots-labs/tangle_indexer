pub mod comment;
pub mod follow;
pub mod job_feedback;
pub mod job_request;
pub mod job_result;
pub mod listing;
pub mod post;
pub mod profile;
pub mod reaction;

pub use comment::ToRadrootsCommentEventIndex;
pub use follow::ToRadrootsFollowEventIndex;
pub use job_feedback::ToRadrootsJobFeedbackEventIndex;
pub use job_request::ToRadrootsJobRequestEventIndex;
pub use job_result::ToRadrootsJobResultEventIndex;
pub use listing::ToRadrootsListingEventIndex;
pub use post::ToRadrootsPostEventIndex;
pub use profile::ToRadrootsProfileEventIndex;
pub use reaction::ToRadrootsReactionEventIndex;

#[macro_export]
macro_rules! opt_required {
    ($opt:expr) => {
        $opt.required(stringify!($opt))
    };
}

#[macro_export]
macro_rules! opt_default {
    ($opt:expr) => {
        match $opt {
            Some(val) => val,
            None => "".to_string(),
        }
    };
    ($opt:expr, $default:expr) => {
        match $opt {
            Some(val) => val,
            None => $default.to_string(),
        }
    };
}

pub trait RequiredField {
    type Output;
    fn required(self, field_name: &str) -> Result<Self::Output, String>;
}

impl<T> RequiredField for Option<T> {
    type Output = T;

    fn required(self, field_name: &str) -> Result<T, String> {
        self.ok_or_else(|| format!("Missing {}", field_name))
    }
}
