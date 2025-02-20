use std::collections::TryReserveError;

/// Attempts to push elem into self, returns error on allocation failure
pub trait TryPush<T> {
    /// Attempts to push elem into self, returns error on allocation failure
    /// # Example
    /// ```
    /// use rs42::extensions::vec::TryPush;
    ///
    /// let mut vec = Vec::new();
    /// match vec.try_push(42) {
    ///     Ok(()) => println!("Successfully pushed new elem"),
    ///     Err(err) => println!("Allocation failed: {err}"),
    /// }
    /// ```
    fn try_push(&mut self, elem: T) -> Result<(), TryReserveError>;
}

impl<T> TryPush<T> for Vec<T> {
    fn try_push(&mut self, elem: T) -> Result<(), TryReserveError> {
        self.try_reserve(1)?;
        self.push(elem);
        Ok(())
    }
}
