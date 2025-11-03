pub mod comment;
pub mod listing;
pub mod profile;
pub mod reaction;

pub use comment::ToRadrootsCommentEventIndex;
pub use listing::ToRadrootsListingEventIndex;
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
