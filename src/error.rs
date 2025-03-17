pub type AppResult<T = ()> = Result<T, anyhow::Error>;

#[macro_export]
macro_rules! app_error {
    ($tag:expr, $message: literal) => {
        anyhow::anyhow!("[{}] {}", $tag, $message)
    };

     ($tag:expr, $fmt:expr, $($arg:tt)*) => {
        anyhow::anyhow!("[{}] {}", $tag, format!($fmt, $($arg)*))
    };
}
