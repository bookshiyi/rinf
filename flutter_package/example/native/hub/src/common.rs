use std::error::Error;

/// This `Result` type alias allows handling any error type
/// that implements the `Error` trait.
/// In practice, it is recommended to use custom solutions
/// or crates like `anyhow` dedicated to error handling.
/// Building an app differs from writing a library, as apps
/// may encounter numerous error situations, which is why
/// a single, flexible error type is needed.
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Because spawn functions are used very often,
/// we make them accessible from everywhere.
pub use tokio::task::{spawn, spawn_blocking};
