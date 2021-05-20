//! Backends persist the database. They allow reading and writing bytes. Bytes-to-data conversion,
//! and back, is handled by a [`Format`](crate::format::Format).
//!
//! # Examples
//!
//! ```
//! use std::default::Default;
//! use koit::{Database, format::Json, backend::Memory};
//!
//! type Messages = Vec<String>;
//!
//! #[async_std::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let db: Database<Messages, Memory, Json> = Database::from_parts(
//!         Messages::default(), Memory::default()
//!     );
//!
//!     db.write(|messages| {
//!         messages.push("a message".to_owned());
//!         messages.push("from me to you".to_owned());
//!     }).await;
//!     db.save().await?;
//!
//!     let (_data, mut backend) = db.into_parts();
//!     assert_eq!(&mut backend.take(),
//! br#"[
//!   "a message",
//!   "from me to you"
//! ]"#
//!     );
//!
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;

/// Trait implementable by bytes storage providers.
///
/// # Examples
///
/// See the [backend module documentation](crate::backend).
#[async_trait]
pub trait Backend {
  type Error: std::error::Error + Send + Sync + 'static;

  /// Read all data from the backend.
  ///
  /// # Errors
  ///
  /// If the bytes failed to be read by the backend, an error variant is returned.
  async fn read(&mut self) -> Result<Vec<u8>, Self::Error>;

  /// Overwrite the backend with the given data.
  ///
  /// # Errors
  ///
  /// If the bytes failed to be written to the backend, an error variant is returned.
  /// This may mean the backend is now corrupted.
  async fn write(&mut self, data: Vec<u8>) -> Result<(), Self::Error>;
}

/// An in-memory backend.
///
/// # Examples
///
/// See the [backend module documentation](crate::backend).
#[derive(std::default::Default, Debug, Clone, PartialEq, Eq)]
pub struct Memory(Vec<u8>);

impl Memory {
  pub fn new() -> Self {
    Self(Vec::new())
  }

  /// Take the data out of the backend, leaving an empty backend in its place.
  pub fn take(&mut self) -> Vec<u8> {
    std::mem::replace(&mut self.0, Vec::new())
  }
}

impl From<Vec<u8>> for Memory {
  fn from(buf: Vec<u8>) -> Self {
    Self(buf)
  }
}

#[async_trait]
impl Backend for Memory {
  type Error = std::convert::Infallible;

  async fn read(&mut self) -> Result<Vec<u8>, Self::Error> {
    Ok(self.0.clone())
  }
  async fn write(&mut self, data: Vec<u8>) -> Result<(), Self::Error> {
    Ok(self.0 = data)
  }
}

#[cfg(feature = "file-backend")]
pub use self::file::File;

#[cfg(feature = "file-backend")]
mod file {
  use async_std::io::prelude::{ReadExt, SeekExt, WriteExt};
  use async_trait::async_trait;

  use super::Backend;

  /// A file-backed backend.
  ///
  /// Note: this requires its futures to be executed on the Tokio 0.3 runtime.
  #[cfg_attr(docsrs, doc(cfg(feature = "file-backend")))]
  #[derive(Debug)]
  pub struct File(async_std::fs::File);

  impl File {
    /// Creates the backend by opening the file at the given path.
    ///
    /// # Errors
    ///
    /// If the file does not exist or could not be opened for reading and writing, an error
    /// variant is returned.
    pub async fn from_path<P>(path: P) -> Result<Self, std::io::Error>
    where
      P: AsRef<std::path::Path>,
    {
      Ok(Self(
        async_std::fs::OpenOptions::new()
          .read(true)
          .write(true)
          .open(path.as_ref())
          .await?,
      ))
    }

    /// Creates the backend by opening a file at the given path. Creates the file if it
    /// does not exist yet.
    ///
    /// # Errors
    ///
    /// If the file does not exist, but could not be created, or could not be opened for
    /// reading and writing, an error variant is returned.
    pub async fn from_path_or_create<P>(path: P) -> Result<(Self, bool), std::io::Error>
    where
      P: AsRef<std::path::Path>,
    {
      let backend = Self::from_path(&path).await;
      match backend {
        Ok(self_) => Ok((self_, true)),
        Err(err) => match err.kind() {
          std::io::ErrorKind::NotFound => Ok((
            Self(
              async_std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(path.as_ref())
                .await?,
            ),
            false,
          )),
          _ => Err(err),
        },
      }
    }
  }

  #[async_trait]
  impl Backend for File {
    type Error = std::io::Error;

    async fn read(&mut self) -> Result<Vec<u8>, Self::Error> {
      let mut buffer = Vec::new();
      self.0.seek(std::io::SeekFrom::Start(0)).await?;
      self.0.read_to_end(&mut buffer).await?;
      Ok(buffer)
    }

    async fn write(&mut self, data: Vec<u8>) -> Result<(), Self::Error> {
      self.0.seek(std::io::SeekFrom::Start(0)).await?;
      self.0.set_len(0).await?;
      self.0.write_all(&data).await?;
      self.0.sync_all().await?;
      Ok(())
    }
  }
}
