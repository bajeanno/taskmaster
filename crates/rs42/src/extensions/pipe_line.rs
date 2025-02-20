pub trait PipeLine: Sized {
    /// Calls the given function passing self as argument and returning the output
    ///
    /// # Examples
    /// ```
    /// use rs42::extensions::PipeLine;
    ///
    /// let a = 40_i32.pipe(|nb| nb + 2);
    /// assert_eq!(a, 42);
    ///
    /// let b = 42_i32.pipe(Some);
    /// assert_eq!(b, Some(42));
    /// ```
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }

    /// Calls the given function passing &mut self as argument and returning the output
    ///
    /// # Examples
    /// ```
    /// use rs42::extensions::PipeLine;
    ///
    /// let mut vec = vec![42];
    /// let new_vec = vec.pipe_ref_mut(std::mem::take);
    /// assert!(vec.is_empty());
    /// assert_eq!(new_vec, [42]);
    /// ```
    fn pipe_ref_mut<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        f(self)
    }

    /// Calls the given function passing &self as argument and returning the output
    ///
    /// # Examples
    /// ```
    /// use rs42::extensions::PipeLine;
    ///
    /// fn get_first_elem(i: &Vec<i32>) -> i32 {
    ///     i[0]
    /// }
    ///
    /// let vec = vec![42_i32];
    /// assert_eq!(vec.pipe_ref(get_first_elem), 42);
    /// ```
    fn pipe_ref<T>(&self, f: impl FnOnce(&Self) -> T) -> T {
        f(self)
    }

    /// Calls the given unsafe function passing self as argument and returning the output
    ///
    /// # Safety
    /// f() will be called only once, ensure it is safe to call
    ///
    /// # Examples
    /// ```
    /// use rs42::extensions::PipeLine;
    ///
    /// unsafe fn foo(nb: i32) -> i32 {
    ///     // Unsafe code...
    ///     return nb + 2;
    /// }
    ///
    /// unsafe { assert_eq!(40.pipe_unsafe(foo), 42) }
    /// ```
    #[allow(dead_code)]
    unsafe fn pipe_unsafe<T>(self, f: unsafe fn(Self) -> T) -> T {
        f(self)
    }

    /// Calls the given unsafe function passing &mut self as argument and returning the output
    ///
    /// # Safety
    /// f() will be called only once, ensure it is safe to call
    ///
    /// # Examples
    /// ```
    /// use rs42::extensions::PipeLine;
    ///
    /// unsafe fn foo(nb: &mut i32) -> i32 {
    ///     // unsafe code
    ///     let tmp = *nb;
    ///     *nb += 2;
    ///     tmp
    /// }
    ///
    /// let mut nb = 40;
    /// unsafe {
    ///     assert_eq!(nb.pipe_ref_mut_unsafe(foo), 40);
    ///     assert_eq!(nb, 42);
    /// }
    /// ```
    #[allow(dead_code)]
    unsafe fn pipe_ref_mut_unsafe<T>(&mut self, f: unsafe fn(&mut Self) -> T) -> T {
        f(self)
    }

    /// Calls the given unsafe function passing &self as argument and returning the output
    ///
    /// # Safety
    /// f() will be called only once, ensure it is safe to call
    ///
    /// # Examples
    /// ```
    /// use rs42::extensions::PipeLine;
    ///
    /// unsafe fn foo(nb: &i32) -> i32 {
    ///     *nb
    /// }
    ///
    /// unsafe { assert_eq!(42.pipe_ref_unsafe(foo), 42) }
    /// ```
    #[allow(dead_code)]
    unsafe fn pipe_ref_unsafe<T>(&self, f: unsafe fn(&Self) -> T) -> T {
        f(self)
    }

    /// Calls the given c function passing self as argument and returning the output
    ///
    /// # Safety
    /// f() will be called only once, ensure it is safe to call
    ///
    /// # Examples
    /// ```
    /// use rs42::extensions::PipeLine;
    /// use std::ffi::c_char;
    ///
    /// extern "C" { fn strlen(s: *const c_char) -> usize; }
    ///
    /// unsafe {
    ///     assert_eq!(c"42".as_ptr().pipe_c_fn(strlen), 2);
    /// }
    /// ```
    #[allow(dead_code)]
    unsafe fn pipe_c_fn<T>(self, f: unsafe extern "C" fn(Self) -> T) -> T {
        f(self)
    }
}

impl<T> PipeLine for T {}
