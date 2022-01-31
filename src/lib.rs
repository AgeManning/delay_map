#![warn(rust_2018_idioms)]
#![deny(rustdoc::broken_intra_doc_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::needless_doctest_main)]
//! # Overview
//!
//! This crate contains two data structures, [`HashSetDelay`] and [`HashMapDelay`]. These
//! behave like the standard library HashSet and HashMaps with the added feature that entries
//! inserted into the mappings expire after a fixed period of time.
//!
//!
//! # Usage
//!
//! ## Creating a map
//!
//! ```rust
//!     use delay_map::HashMapDelay;
//!     use futures::prelude::*;
//!
//!     // Set a default timeout for entries
//!     let mut delay_map = HashMapDelay::new(std::time::Duration::from_secs(1));
//!
//!
//!     tokio_test::block_on(async {
//!
//!     delay_map.insert(1, "entry_1");
//!     delay_map.insert(2, "entry_2");
//!     
//!     if let Some(Ok((key, value))) = delay_map.next().await {
//!         println!("Entry 1: {}, {}", key, value);  
//!     }
//!
//!     if let Some(Ok((key, value))) = delay_map.next().await {
//!         println!("Entry 2: {}, {}", key,value);  
//!     }
//!     });
//! ```

pub mod hashmap_delay;
pub mod hashset_delay;

pub use hashmap_delay::HashMapDelay;
pub use hashset_delay::HashSetDelay;
