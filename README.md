# Koit
[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]

[crates-badge]: https://img.shields.io/crates/v/koit.svg
[crates-url]: https://crates.io/crates/koit
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/tomcur/koit/blob/master/LICENSE

Koit is a simple, asynchronous, pure-Rust, structured, embedded database.

```toml
[dependencies]
koit = "0.2"
```

## Example

```rust
use std::default::Default;

use koit::{FileDatabase, format::Json};
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
struct Data {
    cats: u64,
    yaks: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = FileDatabase::<Data, Json>::load_from_path_or_default("./db.json").await?;
  
    db.write(|data| {
        data.cats = 10;
        data.yaks = 32;
    }).await;
    
    assert_eq!(db.read(|data| data.cats + data.yaks).await, 42);

    db.save().await?;

    Ok(())
}
```

## Features
- built-in, future-aware, reader-writer synchronization
- works with arbitrary data formatters
- works with arbitrary storage backends
- comes with default formatters and backends that fit more purposes

By default, Koit comes with its file-backend, JSON formatter and Bincode
formatter enabled. You can cherry-pick features instead.

```toml
[dependencies.koit]
version = "0.2"
default-features = false
features = ["bincode-format"]
```

## Purpose

Koit enables quickly implementing persistence and concurrent access to
structured data. It is meant to be used with relatively small amounts
(megabytes) of data.

It is not a performant database. Upon loading, the entire data structure is
kept in memory. Upon saving, the entire data structure is formatted and written
to the storage backend.

## Other crates

Koit is inspired by [Rustbreak](https://github.com/TheNeikos/rustbreak), a
similar (synchronous) database.

## License

This project is licensed under the [MIT license].

[MIT license]: https://github.com/tomcur/koit/blob/master/LICENSE

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Koit by you, shall be licensed as MIT, without any additional
terms or conditions.
