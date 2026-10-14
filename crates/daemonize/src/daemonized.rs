use libc::sys::types::Pid;

use crate::{ForkFailed, RedirectIOError};

#[must_use = "InsideDaemon and OutsideDaemon variants may contain an error"]
#[derive(Debug)]
pub enum Daemonized<InsideDaemonOk, InsideDaemonErr, OutsideDaemonOk, OutsideDaemonErr> {
    InsideDaemon(Result<InsideDaemonOk, InsideDaemonErr>),
    OutsideDaemon(Result<OutsideDaemonOk, OutsideDaemonErr>),
}

pub enum DaemonizedOk<InsideDaemonOk, OutsideDaemonOk> {
    InsideDaemon(InsideDaemonOk),
    OutsideDaemon(OutsideDaemonOk),
}

pub type DefaultDaemonized = Daemonized<(), RedirectIOError, Pid, ForkFailed>;

/// Use this when you want the `?` operator behavior for a `Daemonized` value.
///
/// The macro unwraps the successful value and returns early with the error in the
/// same daemon branch when the inner result is an `Err`.
///
/// This is especially useful inside a function that already returns a `Daemonized`:
/// it lets the happy path read like ordinary Rust while preserving the daemon-aware
/// error return semantics.
///
/// # Examples
///
/// ```rust
/// use std::assert_matches;
///
/// use daemonize::{try_daemonized, Daemonized, DaemonizedOk};
///
/// fn try_daemonized_add_1_if_ok(
///     value: Daemonized<i32, i32, i32, i32>,
/// ) -> Daemonized<i32, i32, i32, i32> {
///     let value = try_daemonized!(value);
///     match value {
///         DaemonizedOk::InsideDaemon(value) => Daemonized::InsideDaemon(Ok(value + 1)),
///         DaemonizedOk::OutsideDaemon(value) => Daemonized::OutsideDaemon(Ok(value + 1)),
///     }
/// }
///
/// assert_matches!(
///     try_daemonized_add_1_if_ok(Daemonized::InsideDaemon(Ok(42))),
///     Daemonized::InsideDaemon(Ok(43))
/// );
/// assert_matches!(
///     try_daemonized_add_1_if_ok(Daemonized::OutsideDaemon(Ok(42))),
///     Daemonized::OutsideDaemon(Ok(43))
/// );
///
/// assert_matches!(
///     try_daemonized_add_1_if_ok(Daemonized::InsideDaemon(Err(42))),
///     Daemonized::InsideDaemon(Err(42))
/// );
/// assert_matches!(
///     try_daemonized_add_1_if_ok(Daemonized::OutsideDaemon(Err(42))),
///     Daemonized::OutsideDaemon(Err(42))
/// );
/// ```
#[macro_export]
macro_rules! try_daemonized {
    ($expr:expr $(,)?) => {
        match $expr {
            $crate::Daemonized::InsideDaemon(::core::result::Result::Ok(ok)) => {
                $crate::DaemonizedOk::InsideDaemon(ok)
            }
            $crate::Daemonized::OutsideDaemon(::core::result::Result::Ok(ok)) => {
                $crate::DaemonizedOk::OutsideDaemon(ok)
            }
            $crate::Daemonized::InsideDaemon(::core::result::Result::Err(err)) => {
                return $crate::Daemonized::InsideDaemon(::core::result::Result::Err(
                    ::std::convert::From::from(err),
                ));
            }
            $crate::Daemonized::OutsideDaemon(::core::result::Result::Err(err)) => {
                return $crate::Daemonized::OutsideDaemon(::core::result::Result::Err(
                    ::std::convert::From::from(err),
                ));
            }
        }
    };
}

/// Unwraps a `Result` and returns the successful value, propagating errors by
/// returning a `Daemonized::OutsideDaemon(Err(...))` from the current function.
///
/// This behaves like the `?` operator for the outside-daemon branch: on `Ok(value)`,
/// it yields `value`; on `Err(err)`, it returns early with the error converted via
/// `From` into the outside-daemon error type.
///
/// # Examples
///
/// ```rust
/// use std::assert_matches;
/// use daemonize::Daemonized;
///
/// fn add_1_if_ok(value: Result<i32, i32>) -> Daemonized<i32, i32, i32, i32> {
///     let value = daemonize::try_daemonized_outside!(value);
///     Daemonized::OutsideDaemon(Ok(value + 1))
/// }
///
/// assert_matches!(add_1_if_ok(Ok(42)), Daemonized::OutsideDaemon(Ok(43)));
/// assert_matches!(add_1_if_ok(Err(42)), Daemonized::OutsideDaemon(Err(42)));
/// ```
#[macro_export]
macro_rules! try_daemonized_outside {
    ($expr:expr $(,)?) => {
        match $expr {
            ::core::result::Result::Ok(ok) => ok,
            ::core::result::Result::Err(err) => {
                return $crate::Daemonized::OutsideDaemon(::core::result::Result::Err(
                    ::std::convert::From::from(err),
                ))
            }
        }
    };
}

/// Unwraps a `Result` and returns the successful value, propagating errors by
/// returning a `Daemonized::InsideDaemon(Err(...))` from the current function.
///
/// This behaves like the `?` operator for the inside-daemon branch: on `Ok(value)`,
/// it yields `value`; on `Err(err)`, it returns early with the error converted via
/// `From` into the inside-daemon error type.
///
/// # Examples
///
/// ```rust
/// use std::assert_matches;
/// use daemonize::Daemonized;
///
/// fn add_1_if_ok(value: Result<i32, i32>) -> Daemonized<i32, i32, i32, i32> {
///     let value = daemonize::try_daemonized_inside!(value);
///     Daemonized::InsideDaemon(Ok(value + 1))
/// }
///
/// assert_matches!(add_1_if_ok(Ok(42)), Daemonized::InsideDaemon(Ok(43)));
/// assert_matches!(add_1_if_ok(Err(42)), Daemonized::InsideDaemon(Err(42)));
/// ```
#[macro_export]
macro_rules! try_daemonized_inside {
    ($expr:expr $(,)?) => {
        match $expr {
            ::core::result::Result::Ok(ok) => ok,
            ::core::result::Result::Err(err) => {
                return $crate::Daemonized::InsideDaemon(::core::result::Result::Err(
                    ::std::convert::From::from(err),
                ))
            }
        }
    };
}

impl<InsideDaemonOk, InsideDaemonErr, OutsideDaemonOk, OutsideDaemonErr>
    Daemonized<InsideDaemonOk, InsideDaemonErr, OutsideDaemonOk, OutsideDaemonErr>
{
    /// Maps the entire inside-daemon result while leaving the outside-daemon value unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::assert_matches;
    ///
    /// use daemonize::Daemonized;
    ///
    /// fn bump_or_bang(value: Result<i32, &str>) -> Result<i32, String> {
    ///     match value {
    ///         Ok(n) => Ok(n + 1),
    ///         Err(err) => Err(format!("{err}!")),
    ///     }
    /// }
    ///
    /// let inside: Daemonized<i32, String, &str, i32> = Daemonized::InsideDaemon(Ok(1))
    ///     .map_inside_daemon(bump_or_bang);
    /// assert_matches!(inside, Daemonized::InsideDaemon(Ok(2)));
    ///
    /// let outside: Daemonized<i32, String, &str, i32> = Daemonized::OutsideDaemon(Ok("keep me"))
    ///     .map_inside_daemon(bump_or_bang);
    /// assert_matches!(outside, Daemonized::OutsideDaemon(Ok("keep me")));
    ///
    /// let errored: Daemonized<i32, String, &str, i32> = Daemonized::InsideDaemon(Err("nope"))
    ///     .map_inside_daemon(bump_or_bang);
    /// assert_matches!(errored, Daemonized::InsideDaemon(Err(ref value)) if value == "nope!");
    /// ```
    pub fn map_inside_daemon<NewInsideDaemonOk, NewInsideDaemonErr>(
        self,
        f: impl FnOnce(
            Result<InsideDaemonOk, InsideDaemonErr>,
        ) -> Result<NewInsideDaemonOk, NewInsideDaemonErr>,
    ) -> Daemonized<NewInsideDaemonOk, NewInsideDaemonErr, OutsideDaemonOk, OutsideDaemonErr> {
        match self {
            Daemonized::InsideDaemon(tmp) => Daemonized::InsideDaemon(f(tmp)),
            Daemonized::OutsideDaemon(tmp) => Daemonized::OutsideDaemon(tmp),
        }
    }

    /// Maps the successful inside-daemon value while leaving the outside-daemon value unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::assert_matches;
    ///
    /// use daemonize::Daemonized;
    ///
    /// let inside: Daemonized<i32, &str, &str, i32> = Daemonized::InsideDaemon(Ok(1))
    ///     .map_inside_daemon_ok(|value| value + 1);
    /// assert_matches!(inside, Daemonized::InsideDaemon(Ok(2)));
    ///
    /// let outside: Daemonized<i32, &str, &str, i32> = Daemonized::OutsideDaemon(Ok("keep me"))
    ///     .map_inside_daemon_ok(|value: i32| value + 1);
    /// assert_matches!(outside, Daemonized::OutsideDaemon(Ok("keep me")));
    ///
    /// let errored: Daemonized<i32, &str, &str, i32> = Daemonized::InsideDaemon(Err("nope"))
    ///     .map_inside_daemon_ok(|value: i32| value + 1);
    /// assert_matches!(errored, Daemonized::InsideDaemon(Err("nope")));
    /// ```
    pub fn map_inside_daemon_ok<NewInsideDaemonOk>(
        self,
        f: impl FnOnce(InsideDaemonOk) -> NewInsideDaemonOk,
    ) -> Daemonized<NewInsideDaemonOk, InsideDaemonErr, OutsideDaemonOk, OutsideDaemonErr> {
        match self {
            Daemonized::InsideDaemon(Ok(tmp)) => Daemonized::InsideDaemon(Ok(f(tmp))),
            Daemonized::InsideDaemon(Err(err)) => Daemonized::InsideDaemon(Err(err)),
            Daemonized::OutsideDaemon(tmp) => Daemonized::OutsideDaemon(tmp),
        }
    }

    /// Maps the error from the inside-daemon value while leaving the outside-daemon value unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::assert_matches;
    ///
    /// use daemonize::Daemonized;
    ///
    /// let inside: Daemonized<i32, String, &str, i32> = Daemonized::InsideDaemon(Err(1))
    ///     .map_inside_daemon_err(|err| err.to_string());
    /// assert_matches!(inside, Daemonized::InsideDaemon(Err(ref value)) if value == "1");
    ///
    /// let outside: Daemonized<i32, i32, &str, i32> = Daemonized::OutsideDaemon(Ok("keep me"))
    ///     .map_inside_daemon_err(|err: i32| err + 1);
    /// assert_matches!(outside, Daemonized::OutsideDaemon(Ok("keep me")));
    ///
    /// let ok_value: Daemonized<i32, i32, &str, i32> = Daemonized::InsideDaemon(Ok(7))
    ///     .map_inside_daemon_err(|err: i32| err + 1);
    /// assert_matches!(ok_value, Daemonized::InsideDaemon(Ok(7)));
    /// ```
    pub fn map_inside_daemon_err<NewInsideDaemonErr>(
        self,
        f: impl FnOnce(InsideDaemonErr) -> NewInsideDaemonErr,
    ) -> Daemonized<InsideDaemonOk, NewInsideDaemonErr, OutsideDaemonOk, OutsideDaemonErr> {
        match self {
            Daemonized::InsideDaemon(Ok(tmp)) => Daemonized::InsideDaemon(Ok(tmp)),
            Daemonized::InsideDaemon(Err(err)) => Daemonized::InsideDaemon(Err(f(err))),
            Daemonized::OutsideDaemon(tmp) => Daemonized::OutsideDaemon(tmp),
        }
    }

    /// Maps the entire outside-daemon result while leaving the inside-daemon value unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::assert_matches;
    ///
    /// use daemonize::Daemonized;
    ///
    /// fn bump_or_bang(value: Result<i32, &str>) -> Result<i32, String> {
    ///     match value {
    ///         Ok(n) => Ok(n + 1),
    ///         Err(err) => Err(format!("{err}!")),
    ///     }
    /// }
    ///
    /// let outside: Daemonized<&str, i32, i32, String> = Daemonized::OutsideDaemon(Ok(1))
    ///     .map_outside_daemon(bump_or_bang);
    /// assert_matches!(outside, Daemonized::OutsideDaemon(Ok(2)));
    ///
    /// let inside: Daemonized<&str, i32, i32, String> = Daemonized::InsideDaemon(Ok("keep me"))
    ///     .map_outside_daemon(bump_or_bang);
    /// assert_matches!(inside, Daemonized::InsideDaemon(Ok("keep me")));
    ///
    /// let errored: Daemonized<&str, i32, i32, String> = Daemonized::OutsideDaemon(Err("nope"))
    ///     .map_outside_daemon(bump_or_bang);
    /// assert_matches!(errored, Daemonized::OutsideDaemon(Err(ref value)) if value == "nope!");
    /// ```
    pub fn map_outside_daemon<NewOutsideDaemonOk, NewOutsideDaemonErr>(
        self,
        f: impl FnOnce(
            Result<OutsideDaemonOk, OutsideDaemonErr>,
        ) -> Result<NewOutsideDaemonOk, NewOutsideDaemonErr>,
    ) -> Daemonized<InsideDaemonOk, InsideDaemonErr, NewOutsideDaemonOk, NewOutsideDaemonErr> {
        match self {
            Daemonized::InsideDaemon(tmp) => Daemonized::InsideDaemon(tmp),
            Daemonized::OutsideDaemon(tmp) => Daemonized::OutsideDaemon(f(tmp)),
        }
    }

    /// Maps the successful outside-daemon value while leaving the inside-daemon value unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::assert_matches;
    ///
    /// use daemonize::Daemonized;
    ///
    /// let outside: Daemonized<&str, i32, i32, &str> = Daemonized::OutsideDaemon(Ok(1))
    ///     .map_outside_daemon_ok(|value| value + 1);
    /// assert_matches!(outside, Daemonized::OutsideDaemon(Ok(2)));
    ///
    /// let inside: Daemonized<&str, i32, i32, &str> = Daemonized::InsideDaemon(Ok("keep me"))
    ///     .map_outside_daemon_ok(|value: i32| value + 1);
    /// assert_matches!(inside, Daemonized::InsideDaemon(Ok("keep me")));
    ///
    /// let errored: Daemonized<&str, i32, i32, &str> = Daemonized::OutsideDaemon(Err("nope"))
    ///     .map_outside_daemon_ok(|value: i32| value + 1);
    /// assert_matches!(errored, Daemonized::OutsideDaemon(Err("nope")));
    /// ```
    pub fn map_outside_daemon_ok<NewOutsideDaemonOk>(
        self,
        f: impl FnOnce(OutsideDaemonOk) -> NewOutsideDaemonOk,
    ) -> Daemonized<InsideDaemonOk, InsideDaemonErr, NewOutsideDaemonOk, OutsideDaemonErr> {
        match self {
            Daemonized::InsideDaemon(tmp) => Daemonized::InsideDaemon(tmp),
            Daemonized::OutsideDaemon(Ok(tmp)) => Daemonized::OutsideDaemon(Ok(f(tmp))),
            Daemonized::OutsideDaemon(Err(err)) => Daemonized::OutsideDaemon(Err(err)),
        }
    }

    /// Maps the error from the outside-daemon value while leaving the inside-daemon value unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::assert_matches;
    ///
    /// use daemonize::Daemonized;
    ///
    /// let outside: Daemonized<&str, i32, i32, String> = Daemonized::OutsideDaemon(Err(1))
    ///     .map_outside_daemon_err(|err| err.to_string());
    /// assert_matches!(outside, Daemonized::OutsideDaemon(Err(ref value)) if value == "1");
    ///
    /// let inside: Daemonized<&str, i32, i32, i32> = Daemonized::InsideDaemon(Ok("keep me"))
    ///     .map_outside_daemon_err(|err: i32| err + 1);
    /// assert_matches!(inside, Daemonized::InsideDaemon(Ok("keep me")));
    ///
    /// let ok_value: Daemonized<&str, i32, i32, i32> = Daemonized::OutsideDaemon(Ok(7))
    ///     .map_outside_daemon_err(|err: i32| err + 1);
    /// assert_matches!(ok_value, Daemonized::OutsideDaemon(Ok(7)));
    /// ```
    pub fn map_outside_daemon_err<NewOutsideDaemonErr>(
        self,
        f: impl FnOnce(OutsideDaemonErr) -> NewOutsideDaemonErr,
    ) -> Daemonized<InsideDaemonOk, InsideDaemonErr, OutsideDaemonOk, NewOutsideDaemonErr> {
        match self {
            Daemonized::InsideDaemon(tmp) => Daemonized::InsideDaemon(tmp),
            Daemonized::OutsideDaemon(Ok(tmp)) => Daemonized::OutsideDaemon(Ok(tmp)),
            Daemonized::OutsideDaemon(Err(err)) => Daemonized::OutsideDaemon(Err(f(err))),
        }
    }
}
