use futures::prelude::*;
use std::{
    collections::HashMap,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
pub use tokio::time::Instant;
use tokio_util::time::delay_queue::{self, DelayQueue};

/// The default delay for entries, in seconds. This is only used when [`HashMapDelay::default`] is
/// used to instantiate the [`HashMapDelay`] and then [`HashMapDelay::insert`] is used to add
/// entries. Specific times can be specified using [`HashMapDelay::insert_at`].
const DEFAULT_DELAY: u64 = 30;

/// A data structure that behaves like a hashmap whose entries expire after a given amount of time.
/// This implements [`Stream`] and should be polled for expired entries. Duplicate entires reset
/// the expiration time.
#[derive(Debug)]
pub struct HashMapDelay<K, V>
where
    K: std::cmp::Eq + std::hash::Hash + std::clone::Clone,
{
    /// The given entries.
    entries: HashMap<K, MapEntry<V>>,
    /// A queue holding the timeouts of each entry.
    expirations: DelayQueue<K>,
    /// The default expiration timeout of an entry.
    default_entry_timeout: Duration,
}

/// A wrapping around entries that adds the link to the entry's expiration, via a `delay_queue` key.
#[derive(Debug)]
struct MapEntry<V> {
    /// The expiration key for the entry.
    key: delay_queue::Key,
    /// The actual entry.
    value: V,
}

impl<K, V> Default for HashMapDelay<K, V>
where
    K: std::cmp::Eq + std::hash::Hash + std::clone::Clone,
{
    fn default() -> Self {
        HashMapDelay::new(Duration::from_secs(DEFAULT_DELAY))
    }
}

impl<K, V> HashMapDelay<K, V>
where
    K: std::cmp::Eq + std::hash::Hash + std::clone::Clone,
{
    /// Creates a new instance of [`HashMapDelay`]. The `default_entry_timeout` parameter specifies
    /// the default timeout for new entries inserted using the [`HashMapDelay::insert`] function.
    pub fn new(default_entry_timeout: Duration) -> Self {
        HashMapDelay {
            entries: HashMap::new(),
            expirations: DelayQueue::new(),
            default_entry_timeout,
        }
    }

    /// Creates an empty `HashMapDelay` with at least the specified capacity.
    ///
    /// The hash map will be able to hold at least `capacity` elements without
    /// reallocating. This method is allowed to allocate for more elements than
    /// `capacity`. If `capacity` is 0, the hash map will not allocate.
    ///
    pub fn with_capacity(default_entry_timeout: Duration, capacity: usize) -> Self {
        HashMapDelay {
            entries: HashMap::with_capacity(capacity),
            expirations: DelayQueue::with_capacity(capacity),
            default_entry_timeout,
        }
    }

    /// Insert an entry into the mapping. Entries will expire after the `default_entry_timeout`,
    /// specified when creating a [`HashMapDelay`] via [`HashMapDelay::new`].
    /// NOTE: This will panic if run outside of a tokio runtime.
    pub fn insert(&mut self, key: K, value: V) {
        self.insert_at(key, value, self.default_entry_timeout);
    }

    /// Inserts an entry that will expire at a given instant. If the entry already exists, the
    /// timeout is updated.
    /// NOTE: This will panic if run outside of a tokio runtime or if the duration is too far in
    /// the future.
    pub fn insert_at(&mut self, key: K, value: V, entry_duration: Duration) {
        if let Some(entry) = self.entries.get(&key) {
            // update the timeout
            self.expirations.reset(&entry.key, entry_duration);
        } else {
            let delay_key = self.expirations.insert(key.clone(), entry_duration);
            let entry = MapEntry {
                key: delay_key,
                value,
            };
            self.entries.insert(key, entry);
        }
    }

    /// Gets a reference to an entry if it exists.
    ///
    /// Returns None if the entry does not exist.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key).map(|entry| &entry.value)
    }

    /// Gets a mutable reference to an entry if it exists.
    ///
    /// Returns None if the entry does not exist.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.entries.get_mut(key).map(|entry| &mut entry.value)
    }

    /// Returns true if the key exists, false otherwise.
    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// Returns the length of the mapping.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Checks if the mapping is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Updates the timeout for a given key. Returns true if the key existed, false otherwise.
    ///
    /// Panics if the duration is too far in the future.
    pub fn update_timeout(&mut self, key: &K, timeout: Duration) -> bool {
        if let Some(entry) = self.entries.get(key) {
            self.expirations.reset(&entry.key, timeout);
            true
        } else {
            false
        }
    }

    /// Removes a key from the map returning the value associated with the key that was in the map.
    ///
    /// Return None if the key was not in the map.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let entry = self.entries.remove(key)?;
        self.expirations.remove(&entry.key);
        Some(entry.value)
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all pairs `(k, v)` such that `f(&k,&mut v)` returns false.
    pub fn retain<F: FnMut(&K, &V) -> bool>(&mut self, mut f: F) {
        let expiration = &mut self.expirations;
        self.entries.retain(|key, entry| {
            let result = f(key, &entry.value);
            if !result {
                expiration.remove(&entry.key);
            }
            result
        })
    }

    /// Removes all entries from the map.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.expirations.clear();
    }

    /// Returns an iterator referencing all keys in the map.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.keys()
    }

    /// Returns a vector of referencing all values in the map.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.values().map(|entry| &entry.value)
    }

    /// Returns an iterator referencing all items in the map.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, entry)| (k, &entry.value))
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
            .map(|map_entry| self.expirations.deadline(&map_entry.key))
    }

    /// Removes expired entries and returns them.
    pub fn poll_expired(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<(K, V), String>>> {
        match self.expirations.poll_expired(cx) {
            Poll::Ready(Some(key)) => match self.entries.remove(key.get_ref()) {
                Some(entry) => Poll::Ready(Some(Ok((key.into_inner(), entry.value)))),
                None => Poll::Ready(Some(Err("Value no longer exists in expirations".into()))),
            },
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<K, V> Stream for HashMapDelay<K, V>
where
    K: std::cmp::Eq + std::hash::Hash + std::clone::Clone + Unpin,
    V: Unpin,
{
    type Item = Result<(K, V), String>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        HashMapDelay::poll_expired(self.get_mut(), cx)
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    #[tokio::test]
    async fn should_not_panic() {
        let key = 2u8;
        let value = 0;

        let mut map = HashMapDelay::default();

        map.insert(key, value);
        map.update_timeout(&key, Duration::from_secs(100));

        let fut = |cx: &mut Context<'_>| {
            let _ = map.poll_next_unpin(cx);
            let _ = map.poll_next_unpin(cx);
            Poll::Ready(())
        };

        future::poll_fn(fut).await;

        map.insert(key, value);
        map.update_timeout(&key, Duration::from_secs(100));
    }

    #[tokio::test]
    async fn basic_insert() {
        let key = 2u8;
        let value = 10;

        let mut map = HashMapDelay::default();
        map.insert(key, value);

        // Check the map contains the key
        assert!(map.contains_key(&key));

        // Check that we can remove the key
        map.remove(&key);
        assert!(!map.contains_key(&key));

        // Check that the expiry works for the same key.
        map.insert_at(key, value, Duration::from_millis(50));
        assert!(map.contains_key(&key));

        // Make sure it expires correctly
        match tokio::time::timeout_at(
            tokio::time::Instant::now() + Duration::from_millis(100),
            map.next(),
        )
        .await
        {
            Err(_) => panic!("Entry did not expire"),
            Ok(Some(Ok((k, v)))) => {
                assert_eq!(v, value);
                assert_eq!(k, key);
            }
            Ok(Some(_)) => panic!("Polling the map failed"),
            Ok(None) => panic!("Entry did not exist, stream terminated"),
        }

        // The entry should be removed and the size of the mapping has finished.
        assert_eq!(map.len(), 0);
    }

    #[tokio::test]
    async fn insert_clear() {
        let key = 2u8;
        let value = 10;

        let mut map = HashMapDelay::default();
        map.insert(key, value);

        // Check the map contains the key
        assert!(map.contains_key(&key));
        // Check that we can remove the key
        map.clear();
        assert!(!map.contains_key(&key));
        assert_eq!(map.expirations.len(), 0);
    }
}
