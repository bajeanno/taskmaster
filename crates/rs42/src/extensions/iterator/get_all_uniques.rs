use std::collections::HashSet;
use std::hash::Hash;

/// Collects all unique elements of an iterator into type R
/// Does not preserve element order
pub trait GetAllUniques<T> {
    /// Collects all unique elements of an iterator into type R
    /// Does not preserve element order
    ///
    /// Example
    /// ```
    /// use rs42::extensions::iterator::GetAllUniques;
    ///
    /// let arr = [0, 0, 1, 1, 2, 2];
    /// let uniques = arr.into_iter().get_all_uniques::<Vec<_>>();
    /// assert_eq!(uniques.len(), 3);
    /// assert!(uniques.contains(&0));
    /// assert!(uniques.contains(&1));
    /// assert!(uniques.contains(&2));
    /// ```
    fn get_all_uniques<R>(self) -> R
    where
        R: FromIterator<T>;
}

impl<I, T> GetAllUniques<T> for I
where
    I: Iterator<Item = T>,
    T: Eq + Hash,
{
    fn get_all_uniques<R>(self) -> R
    where
        R: FromIterator<T>,
    {
        self.collect::<HashSet<T>>().into_iter().collect()
    }
}
