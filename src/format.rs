//! Formats handle transforming structured data to and from bytes for persisting.

/// Trait implementable by format providers.
///
/// By implementing this trait, a type becomes a marker for the specified format.
/// That type can then be used for formatting (without instantiating an object of that type).
pub trait Format<T>: Sized {
  type SerError: std::error::Error + Send + Sync + 'static;
  type DeError: std::error::Error + Send + Sync + 'static;

  /// Convert data to bytes.
  ///
  /// # Errors
  ///
  /// If the data failed to be encoded by the format, an error variant is returned.
  fn to_bytes(value: &T) -> Result<Vec<u8>, Self::SerError>;

  /// Convert bytes to data.
  ///
  /// # Errors
  ///
  /// If the bytes failed to be decoded by the format, an error variant is returned.
  fn from_bytes(data: Vec<u8>) -> Result<T, Self::DeError>;
}

#[cfg(feature = "json-format")]
pub use self::json::Json;

#[cfg(feature = "json-format")]
mod json {
  use serde::{de::DeserializeOwned, Serialize};

  use super::Format;

  #[cfg_attr(docsrs, doc(cfg(feature = "json-format")))]
  /// A pretty-printed JSON [`Format`](crate::format::Format).
  #[derive(Debug, std::default::Default)]
  pub struct Json;

  impl<T: DeserializeOwned + Serialize> Format<T> for Json {
    type DeError = serde_json::Error;
    type SerError = Self::DeError;

    fn to_bytes(value: &T) -> Result<Vec<u8>, Self::SerError> {
      Ok(serde_json::to_vec_pretty(value)?)
    }
    fn from_bytes(data: Vec<u8>) -> Result<T, Self::DeError> {
      Ok(serde_json::from_slice(&data)?)
    }
  }
}

#[cfg(feature = "bincode-format")]
pub use self::bincode::Bincode;

#[cfg(feature = "bincode-format")]
mod bincode {
  use serde::{de::DeserializeOwned, Serialize};

  use super::Format;

  #[cfg_attr(docsrs, doc(cfg(feature = "bincode-format")))]
  /// A Bincode [`Format`](crate::format::Format).
  #[derive(Debug, std::default::Default)]
  pub struct Bincode;

  impl<T: Serialize + DeserializeOwned> Format<T> for Bincode {
    type DeError = bincode::Error;
    type SerError = Self::DeError;

    fn to_bytes(value: &T) -> Result<Vec<u8>, Self::SerError> {
      Ok(bincode::serialize(value)?)
    }
    fn from_bytes(data: Vec<u8>) -> Result<T, Self::DeError> {
      Ok(bincode::deserialize(&data)?)
    }
  }
}

#[cfg(feature = "toml-format")]
pub use self::toml::Toml;

#[cfg(feature = "toml-format")]
mod toml {
  use serde::{de::DeserializeOwned, Serialize};

  use super::Format;

  #[cfg_attr(docsrs, doc(cfg(feature = "toml-format")))]
  /// A pretty-printed Toml [`Format`](crate::format::Format).
  #[derive(Debug, std::default::Default)]
  pub struct Toml;

  impl<T: DeserializeOwned + Serialize> Format<T> for Toml {
    type SerError = toml::ser::Error;
    type DeError = toml::de::Error;

    fn to_bytes(value: &T) -> Result<Vec<u8>, Self::SerError> {
      Ok(toml::to_vec(value)?)
    }
    fn from_bytes(data: Vec<u8>) -> Result<T, Self::DeError> {
      Ok(toml::from_slice(&data)?)
    }
  }
}
