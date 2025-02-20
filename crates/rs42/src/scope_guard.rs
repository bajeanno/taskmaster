use std::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

/// Structure that wraps a value and calls a callback function when it exits scope
pub struct ScopeGuard<T, F>
where
    F: FnOnce(T),
{
    content: ManuallyDrop<T>,
    callback: ManuallyDrop<F>,
}

impl<T, F> ScopeGuard<T, F>
where
    F: FnOnce(T),
{
    /// Wraps content in a ScopeGuard and calls callback when the returned value exits scope
    ///
    /// # Example
    /// ```
    /// use rs42::scope_guard::ScopeGuard;
    /// use std::cell::Cell;
    ///
    /// let nb = Cell::new(0_i32);
    /// {
    ///     let _scope_guard = ScopeGuard::new(42, |e| nb.set(e));
    ///     assert_eq!(nb.get(), 0);
    /// }
    /// assert_eq!(nb.get(), 42);
    /// ```
    #[allow(dead_code)]
    #[must_use]
    pub fn new(content: T, callback: F) -> Self {
        ScopeGuard {
            content: ManuallyDrop::new(content),
            callback: ManuallyDrop::new(callback),
        }
    }

    /// Returns the scope guard content and cancels the call to callback
    ///
    /// # Example
    /// ```
    /// use rs42::scope_guard::ScopeGuard;
    /// use std::cell::Cell;
    ///
    /// let nb = Cell::new(0_i32);
    /// {
    ///     let scope_guard = ScopeGuard::new(42, |e| nb.set(e));
    ///     let _ = ScopeGuard::into_inner(scope_guard);
    /// }
    /// assert_eq!(nb.get(), 0);
    /// ```
    #[allow(dead_code)]
    #[must_use]
    pub fn into_inner(mut scope_guard: Self) -> T {
        unsafe {
            ManuallyDrop::drop(&mut scope_guard.callback);
            let content = ManuallyDrop::take(&mut scope_guard.content);
            let _ = ManuallyDrop::new(scope_guard);
            content
        }
    }
}

impl<T, F> Deref for ScopeGuard<T, F>
where
    F: FnOnce(T),
{
    type Target = T;

    /// Returns a reference to the wrapped value
    fn deref(&self) -> &T {
        &self.content
    }
}

impl<T, F> DerefMut for ScopeGuard<T, F>
where
    F: FnOnce(T),
{
    /// Returns a mutable reference to the wrapped value
    fn deref_mut(&mut self) -> &mut T {
        &mut self.content
    }
}

impl<T, F> Drop for ScopeGuard<T, F>
where
    F: FnOnce(T),
{
    /// Calls the callback and drops the wrapped value
    fn drop(&mut self) {
        unsafe {
            let callback = ManuallyDrop::take(&mut self.callback);
            let content = ManuallyDrop::take(&mut self.content);
            callback(content);
        }
    }
}

/// Calls the given closure at the end of the current scope
/// # Example
/// ```
/// use rs42::defer;
/// use std::cell::Cell;
///
/// let nb = Cell::new(0);
/// {
///     defer!(nb.set(1));
///     assert_eq!(nb.get(), 0);
/// }
/// assert_eq!(nb.get(), 1);
/// ```
#[macro_export]
macro_rules! defer {
    ($($t:tt)*) => {
        let _scope_guard = $crate::scope_guard::ScopeGuard::new((), |_| { $($t)* });
    };
}

/// Wraps self in a ScopeGuard and calls callback when the returned value exits scope
#[allow(dead_code)]
pub trait Defer: Sized {
    /// Wraps self in a ScopeGuard and calls callback when the returned value exits scope
    ///
    /// # Example
    /// ```
    /// use rs42::scope_guard::Defer;
    /// use std::cell::Cell;
    ///
    /// let nb = Cell::new(0_i32);
    /// {
    ///     let _scope_guard = 42_i32.defer(|e| nb.set(e));
    ///     assert_eq!(nb.get(), 0);
    /// }
    /// assert_eq!(nb.get(), 42);
    /// ```
    fn defer<F>(self, callback: F) -> ScopeGuard<Self, F>
    where
        F: FnOnce(Self),
    {
        ScopeGuard::new(self, callback)
    }
}

impl<T> Defer for T {}
