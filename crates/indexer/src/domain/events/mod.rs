pub mod metadata;

#[macro_export]
macro_rules! opt_required {
    ($opt:expr) => {
        $opt.required(stringify!($opt))
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
