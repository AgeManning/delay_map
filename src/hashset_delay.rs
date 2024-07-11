/// The default delay for entries, in seconds. This is only used when [`HashSetDelay::default`] is
/// used to instantiate the [`HashSetDelay`] and then [`HashSetDelay::insert`] is used to add
/// entries.
const DEFAULT_DELAY: u64 = 30;

use futures::prelude::*;
use std::{
    collections::HashMap,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
pub use tokio::time::Instant;
use tokio_util::time::delay_queue::{self, DelayQueue};

/// A simple hashset object coupled with a `delay_queue` which has entries that expire after a
/// fixed time.
///
/// A `HashSetDelay` implements `Stream` which removes expired items from the map.
#[derive(Debug)]
pub struct HashSetDelay<K>
where
    K: std::cmp::Eq + std::hash::Hash + std::clone::Clone + Unpin,
{
    /// The given entries.
    entries: HashMap<K, delay_queue::Key>,
    /// A queue holding the timeouts of each entry.
    expirations: DelayQueue<K>,
    /// The default expiration timeout of an entry.
    default_entry_timeout: Duration,
}

impl<K> Default for HashSetDelay<K>
where
    K: std::cmp::Eq + std::hash::Hash + std::clone::Clone + Unpin,
{
    fn default() -> Self {
        HashSetDelay::new(Duration::from_secs(DEFAULT_DELAY))
    }
}

impl<K> HashSetDelay<K>
where
    K: std::cmp::Eq + std::hash::Hash + std::clone::Clone + Unpin,
{
    /// Creates a new instance of `HashSetDelay`.
    pub fn new(default_entry_timeout: Duration) -> Self {
        HashSetDelay {
            entries: HashMap::new(),
            expirations: DelayQueue::new(),
            default_entry_timeout,
        }
    }

    /// Creates an empty `HashSetDelay` with at least the specified capacity.
    ///
    /// The hash map will be able to hold at least `capacity` elements without
    /// reallocating. This method is allowed to allocate for more elements than
    /// `capacity`. If `capacity` is 0, the hash map will not allocate.
    ///
    pub fn with_capacity(default_entry_timeout: Duration, capacity: usize) -> Self {
        HashSetDelay {
            entries: HashMap::with_capacity(capacity),
            expirations: DelayQueue::with_capacity(capacity),
            default_entry_timeout,
        }
    }

    /// Insert an entry into the mapping. Entries will expire after the `default_entry_timeout`.
    pub fn insert(&mut self, key: K) {
        self.insert_at(key, self.default_entry_timeout);
    }

    /// Inserts an entry that will expire at a given instant.
    /// Panics if the duration is too far in the future.
    pub fn insert_at(&mut self, key: K, entry_duration: Duration) {
        if let Some(entry) = self.entries.get(&key) {
            // update the timeout
            self.expirations.reset(entry, entry_duration);
        } else {
            let delay_key = self.expirations.insert(key.clone(), entry_duration);
            self.entries.insert(key, delay_key);
        }
    }

    /// Updates the timeout for a given key. Returns true if the key existed, false otherwise.
    ///
    /// Panics if the duration is too far in the future.
    pub fn update_timeout(&mut self, key: &K, timeout: Duration) -> bool {
        if let Some(delay_key) = self.entries.get_mut(key) {
            self.expirations.reset(delay_key, timeout);
            true
        } else {
            false
        }
    }

    /// Returns true if the key exists, false otherwise.
    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// Returns the length of the mapping.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the mapping is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Removes a key from the map returning the value associated with the key that was in the map.
    ///
    /// Return None if the key was not in the map.
    pub fn remove(&mut self, key: &K) -> bool {
        if let Some(delay_key) = self.entries.remove(key) {
            self.expirations.remove(&delay_key);
            true
        } else {
            false
        }
    }

    /// Removes all entries from the map.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.expirations.clear();
    }

    /// Returns an iterator referencing all items in the set.
    pub fn iter(&self) -> impl Iterator<Item = &K> {
        self.entries.keys()
    }

    /// Shrink the capacity of the underlying hash map to fit the current elements.
    pub fn shrink_to_fit(&mut self) {
        self.entries.shrink_to_fit();
    }

    /// Shrink the capacity of the underlying data structures to hold a specific number of elements
    pub fn shrink_to(&mut self, capacity: usize) {
        self.entries.shrink_to(capacity);
    }

    /// Find the approximate (to the ms) time that a specific entry will expire
    pub fn deadline(&self, key: &K) -> Option<Instant> {
        self.entries
            .get(key)
            .map(|queue_key| self.expirations.deadline(queue_key))
    }

    /// Removes expired entries and returns them.
    pub fn poll_expired(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<K, String>>> {
        match self.expirations.poll_expired(cx) {
            Poll::Ready(Some(key)) => match self.entries.remove(key.get_ref()) {
                Some(_delay_key) => Poll::Ready(Some(Ok(key.into_inner()))),
                None => Poll::Ready(Some(Err("Value no longer exists in expirations".into()))),
            },
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<K> Stream for HashSetDelay<K>
where
    K: std::cmp::Eq + std::hash::Hash + std::clone::Clone + Unpin,
{
    type Item = Result<K, String>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        HashSetDelay::poll_expired(self.get_mut(), cx)
    }
}
