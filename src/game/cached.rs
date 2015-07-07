use std::cell::{UnsafeCell, Cell};

/// Stores a cached value with explicity invalidation.
///
/// The `get` method takes a `refresh` function as argument which is called with a mutable
/// reference to the previously cached value if the cache was invalidated in the meantime (with an
/// explicit call to `invalidate`).
///
/// The `Cached<T>` type provides a limited kind of interior mutability which cannot be modelled
/// easily with the standard cell types since (a) it allows non-Copy types (ruling out `Cell`) and
/// (b) it provides access to an 'unadorned' reference to the cached value. Safety is achieved by
/// ensuring that invalidation cannot occur while this reference is alive.
///
/// Example:
///
/// ```rust
///# use ::game::cached::Cached;
///
/// struct Stats {
///     mean: f32,
///     stddev: f32,
/// }
///
/// struct StatsVec {
///     elems: Vec<f32>,
///     stats: Cached<Stats>,
/// }
///
/// impl StatsVec {
///     fn stats(&self) -> &Stats {
///         self.stats.get(|| {
///             // Only executed if push() has been called since last time stats() was called.
///             let num_elems = self.elems.len() as f32;
///             let mean = self.elems.iter().fold(0.0, |a, x| a + x) / num_elems;
///             Stats {
///                 mean: mean,
///                 stddev: (self.elems.iter()
///                     .map(|&x| x - mean)
///                     .map(|x| x * x)
///                     .fold(0.0, |a, x| a + x) / (num_elems - 1.0)).sqrt(),
///             }
///         })
///     }
///
///     fn push(&mut self, value: f32) {
///         self.elems.push(value);
///         self.stats.invalidate();
///     }
/// }
/// ```
pub struct Cached<T> {
    value: UnsafeCell<T>,
    invalidated: Cell<bool>,
}

impl<T> Cached<T> {
    /// Creates new 'clean' cache containing `initial`.
    pub fn new(initial: T) -> Cached<T> {
        Cached {
            value: UnsafeCell::new(initial),
            invalidated: Cell::new(false),
        }
    }

    /// Creates new 'invalidated' cache. The value in `initial` will never be returned as such, since the
    /// refresh closure will be called on the first get.
    pub fn invalidated(initial: T) -> Cached<T> {
        Cached {
            value: UnsafeCell::new(initial),
            invalidated: Cell::new(true),
        }
    }

    /// Marks the cache as invalidated. This will trigger a refresh next time get() is called.
    pub fn invalidate(&mut self) {
        self.invalidated.set(true);
    }

    /// Retrieves a reference to the contained value.
    ///
    /// If the cache was invalidated since the last call to ```get```, the ```refresh_with``` is
    /// called to obtain and replace the contained value.
    ///
    /// Note: if ```get``` is called inside the ```refresh_with``` closure, it will return the
    /// *old* contained value. This is memory safe, but potentially counterintuitive.
    pub fn get<F>(&self, refresh_with: F) -> &T
            where F: FnOnce() -> T {
        if self.invalidated.get() {
            self.invalidated.set(false);
            // We know there is a single mutable reference because invalidated is now false, so a
            // recursive .get() call will harmlessly return the old value.
            let new_value = refresh_with();
            *unsafe { &mut *self.value.get() } = new_value;
        }
        // It's safe to return an immutable reference to the value since the borrow will prevent a
        // call to invalidate and thus any subsequent refresh.
        unsafe { &*self.value.get() }
    }
}

#[cfg(test)]
mod test {
    use super::Cached;
    #[test]
    fn new_doesnt_call_refresh() {
        let cache = Cached::new(0);
        assert_eq!(cache.get(|| 1), &0);
        assert_eq!(cache.get(|| 1), &0);
    }

    #[test]
    fn invalidated_calls_refresh() {
        let cache = Cached::invalidated(0);
        assert_eq!(cache.get(|| 1), &1);
        assert_eq!(cache.get(|| 2), &1);
    }

    #[test]
    fn invalidate_causes_refresh() {
        let mut cache = Cached::new(0);
        cache.invalidate();
        assert_eq!(cache.get(|| 1), &1);
        assert_eq!(cache.get(|| 2), &1);
        cache.invalidate();
        assert_eq!(cache.get(|| 2), &2);
    }
}
