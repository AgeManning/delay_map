delay_map
============

[![Build Status]][Build Link] [![Doc Status]][Doc Link] [![Crates
Status]][Crates Link]

[Build Status]: https://github.com/agemanning/delay_map/workflows/build/badge.svg?branch=master
[Build Link]: https://github.com/agemanning/delay_map/actions
[Doc Status]: https://docs.rs/delay_map/badge.svg
[Doc Link]: https://docs.rs/delay_map
[Crates Status]: https://img.shields.io/crates/v/delay_map.svg
[Crates Link]: https://crates.io/crates/delay_map

[Documentation at docs.rs](https://docs.rs/delay_map)

# Overview

This crate contains two data structures, [`HashSetDelay`] and [`HashMapDelay`]. These
behave like the standard library HashSet and HashMaps with the added feature that entries
inserted into the mappings expire after a fixed period of time.


# Usage

## Creating a map

```rust
    use delay_map::HashMapDelay;
    use futures::prelude::*;

    // Set a default timeout for entries
    let mut delay_map = HashMapDelay::new(std::time::Duration::from_secs(1));


    tokio_test::block_on(async {

    delay_map.insert(1, "entry_1");
    delay_map.insert(2, "entry_2");
    
    if let Some(Ok((key, value))) = delay_map.next().await {
        println!("Entry 1: {}, {}", key, value);  
    }

    if let Some(Ok((key, value))) = delay_map.next().await {
        println!("Entry 2: {}, {}", key,value);  
    }
    });
```
