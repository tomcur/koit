//! Koit is a simple, asynchronous, pure-Rust, structured, embedded database.
//!
//! # Examples
//!
//! ```
//! use std::default::Default;
//!
//! use koit::{FileDatabase, format::Toml};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Default, Deserialize, Serialize)]
//! struct Data {
//!     cats: u64,
//!     yaks: u64,
//! }
//!
//! #[async_std::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let db = FileDatabase::<Data, Toml>::load_from_path_or_default("./db.toml").await?;
//!   
//!     db.write(|data| {
//!         data.cats = 10;
//!         data.yaks = 32;
//!     }).await;
//!     
//!     assert_eq!(db.read(|data| data.cats + data.yaks).await, 42);
//!
//!     db.save().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Features
//!
//! Koit comes with a [file-backed database](crate::FileDatabase) and [JSON](crate::format::Json)
//! and [Bincode](crate::format::Bincode) formatters. You can also define your own storage
//! [format](crate::format) or [backend](crate::backend).
//!
//! Note that the file-backed database requires the Tokio 0.3 runtime to function.

#![cfg_attr(docsrs, feature(doc_cfg))]

use async_std::sync::{Mutex, RwLock};
use std::future::Future;
use std::marker::PhantomData;

mod error;
pub use error::KoitError;

pub mod backend;
pub use backend::Backend;

pub mod format;
pub use format::Format;

/// The Koit database.
///
/// The database provides reading, writing, saving and reloading functionality.
/// It uses a reader-writer lock on the internal data structure, allowing
/// concurrent access by readers, while writers are given exclusive access.
///
/// It requires a [`Format`](crate::format::Format) marker type
#[derive(Debug)]
pub struct Database<D, B, F> {
  data: RwLock<D>,
  backend: Mutex<B>,
  _format: PhantomData<F>,
}

impl<D, B, F> Database<D, B, F>
where
  B: Backend,
  F: Format<D>,
{
  /// Create a database from its constituents.
  pub fn from_parts(data: D, backend: B) -> Self {
    Self {
      data: RwLock::new(data),
      backend: Mutex::new(backend),
      _format: PhantomData,
    }
  }

  /// Write to the data contained in the database.  This gives exclusive access to the underlying
  /// data structure. The value your closure returns will be passed on as the return value of this
  /// function.
  ///
  /// This write-locks the data structure.
  pub async fn write<T, R>(&self, task: T) -> R
  where
    T: FnOnce(&mut D) -> R,
  {
    let mut data = self.data.write().await;
    task(&mut data)
  }

  /// Same as [`crate::Database::write`], except the task returns a future.
  pub async fn write_and_then<T, Fut, R>(&self, task: T) -> R
  where
    T: FnOnce(&mut D) -> Fut,
    Fut: Future<Output = R>,
  {
    let mut data = self.data.write().await;
    task(&mut data).await
  }

  /// Read the data contained in the database. Many readers can read in parallel.
  /// The value your closure returns will be passed on as the return value of this function.
  ///
  /// This read-locks the data structure.
  pub async fn read<T, R>(&self, task: T) -> R
  where
    T: FnOnce(&D) -> R,
  {
    let data = self.data.read().await;
    task(&data)
  }

  /// Same as [`crate::Database::read`], except the task returns a future.
  pub async fn read_and_then<T, Fut, R>(&self, task: T) -> R
  where
    T: FnOnce(&D) -> Fut,
    Fut: Future<Output = R>,
  {
    let data = self.data.read().await;
    task(&data).await
  }

  /// Replace the actual data in the database by the given data in the parameter, returning the
  /// old data.
  ///
  /// This write-locks the data structure.
  pub async fn replace(&self, data: D) -> D {
    self
      .write(|actual_data| std::mem::replace(actual_data, data))
      .await
  }

  /// Returns a reference to the underlying data lock.
  ///
  /// It is recommended to use the `read` and `write` methods instead of this, to ensure
  /// locks are only held for as long as needed.
  ///
  /// # Examples
  ///
  /// ```
  /// use koit::{Database, format::Json, backend::Memory};
  ///
  /// type Messages = Vec<String>;
  /// let db: Database<_, _, Json> = Database::from_parts(1, Memory::default());
  ///
  /// futures::executor::block_on(async move {
  ///     let data_lock = db.get_data_lock();
  ///     let mut data = data_lock.write().await;
  ///     *data = 42;
  ///     drop(data);
  ///
  ///     db.read(|n| assert_eq!(*n, 42)).await;
  /// });
  /// ```
  pub fn get_data_lock(&self) -> &RwLock<D> {
    &self.data
  }

  /// Returns a mutable reference to the underlying data.
  ///
  /// This borrows `Database` mutably; no locking takes place.
  ///
  /// # Examples
  ///
  /// ```
  /// use koit::{Database, format::Json, backend::Memory};
  ///
  /// let mut db: Database<_, _, Json> = Database::from_parts(1, Memory::default());
  ///
  /// let n = db.get_data_mut();
  /// *n += 41;
  ///
  /// futures::executor::block_on(db.read(|n| assert_eq!(*n, 42)));
  /// ```
  pub fn get_data_mut(&mut self) -> &mut D {
    self.data.get_mut()
  }

  /// Flush the data contained in the database to the backend.
  ///
  /// This read-locks the data structure.
  ///
  /// # Errors
  ///
  /// - If the data in the database failed to be encoded by the format, an error variant is returned.
  /// - If the bytes failed to be written to the backend, an error variant is returned. This may mean
  /// the backend is now corrupted.
  ///
  /// # Panics
  ///
  /// Some back-ends (such as [`crate::backend::File`]) might panic on some async runtimes.
  pub async fn save(&self) -> Result<(), KoitError> {
    let mut backend = self.backend.lock().await;
    let data = self.data.read().await;
    backend
      .write(F::to_bytes(&data).map_err(|err| KoitError::ToFormat(err.into()))?)
      .await
      .map_err(|err| KoitError::BackendWrite(err.into()))?;
    Ok(())
  }

  /// Load data from the backend.
  async fn load_from_backend(&self) -> Result<D, KoitError> {
    let mut backend = self.backend.lock().await;
    let bytes = backend
      .read()
      .await
      .map_err(|err| KoitError::BackendRead(err.into()))?;
    Ok(F::from_bytes(bytes).map_err(|err| KoitError::FromFormat(err.into()))?)
  }

  /// Update this database with data from the backend, returning the old data.
  ///
  /// This will write-lock the internal data structure.
  ///
  /// # Errors
  ///
  /// - If the bytes from teh backend failed to be decoded by the format, an error variant is returned.
  /// - If the bytes failed to be read by the backend, an error variant is returned.
  ///
  /// # Panics
  ///
  /// Some back-ends (such as [`crate::backend::File`]) might panic on some async runtimes.
  pub async fn reload(&self) -> Result<D, KoitError> {
    let new_data = self.load_from_backend().await?;
    Ok(self.replace(new_data).await)
  }

  /// Consume the database and return its data and backend.
  pub fn into_parts(self) -> (D, B) {
    (self.data.into_inner(), self.backend.into_inner())
  }
}

/// A file-backed database.
///
/// Note: this requires its futures to be executed on the Tokio 0.3 runtime.
#[cfg(feature = "file-backend")]
#[cfg_attr(docsrs, doc(cfg(feature = "file-backend")))]
pub type FileDatabase<D, F> = Database<D, backend::File, F>;

#[cfg(feature = "file-backend")]
impl<D, F> FileDatabase<D, F>
where
  F: Format<D>,
{
  /// Construct the file-backed database from the given path. This attempts to load data
  /// from the given file.
  ///
  /// # Errors
  /// If the file cannot be read, or the [formatter](crate::format::Format) cannot decode the data,
  /// an error variant will be returned.
  pub async fn load_from_path<P>(path: P) -> Result<Self, KoitError>
  where
    P: AsRef<std::path::Path>,
  {
    let mut backend = backend::File::from_path(path)
      .await
      .map_err(|err| KoitError::BackendCreation(err.into()))?;

    let bytes = backend
      .read()
      .await
      .map_err(|err| KoitError::BackendRead(err.into()))?;
    let data = F::from_bytes(bytes).map_err(|err| KoitError::FromFormat(err.into()))?;

    Ok(Database {
      data: RwLock::new(data),
      backend: Mutex::new(backend),
      _format: PhantomData,
    })
  }

  /// Construct the file-backed database from the given path. If the file does not exist,
  /// the file is created. Then `factory` is called and its return value is used as the initial value.
  /// This data is immediately and saved to file.
  pub async fn load_from_path_or_else<P, T>(path: P, factory: T) -> Result<Self, KoitError>
  where
    P: AsRef<std::path::Path>,
    T: FnOnce() -> D,
  {
    let (mut backend, exists) = backend::File::from_path_or_create(path)
      .await
      .map_err(|e| KoitError::BackendCreation(e.into()))?;

    let data = if exists {
      let bytes = backend
        .read()
        .await
        .map_err(|err| KoitError::BackendRead(err.into()))?;
      F::from_bytes(bytes).map_err(|err| KoitError::FromFormat(err.into()))?
    } else {
      factory()
    };

    let db = Database {
      data: RwLock::new(data),
      backend: Mutex::new(backend),
      _format: PhantomData,
    };

    db.save().await?;
    Ok(db)
  }

  /// Same as `load_from_path_or_else`, except it uses [`Default`](`std::default::Default`) instead of a factory.
  pub async fn load_from_path_or_default<P>(path: P) -> Result<Self, KoitError>
  where
    P: AsRef<std::path::Path>,
    D: std::default::Default,
  {
    Self::load_from_path_or_else(path, || std::default::Default::default()).await
  }
}
