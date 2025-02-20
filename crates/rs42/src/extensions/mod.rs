mod pipe_line;

/// Trait implementations for std::iter::Iterator
pub mod iterator;
/// Trait implementations for std::vec::Vec
pub mod vec;

/// Trait implemented for all types that pipes self into a given function
pub use pipe_line::PipeLine;
