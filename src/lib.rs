//! # Generic interfaces for Espresso web APIs.
//!
//! All data structures returned by Espresso API endpoints correspond directly to Rust data
//! structures via the serde serialization and deserialization interfaces. For query responses which
//! do not directly correspond to data structures elsewhere in the ecosystem, data structures are
//! defined in this crate which can be serialized to and from the API requests and responses.
//!
//! Types which must be embeddable in URLs (e.g. hashes and identifiers) and binary blob types are
//! serialized as tagged base 64 strings. Other structures use derived serde implementations, which
//! allows them to serialize as human-readable JSON objects or as binary strings, depending on the
//! serializer used. This makes it easy for the API to support multiple content types in its
//! responses, as long as each endpoint handler returns an object with the appropriate Serialize
//! implementation.
//!
//! This crate also provides some helper functions and middleware to encourage interfacing with
//! `tide` and `surf` in a consistent, idiomatic way. The `server` and `client` modules contain
//! middleware which can be attached to a `tide::Server` and `surf::Client` respectively, in order
//! to automatically convert from serializable Rust types to properly formatted HTTP requests and
//! responses, supporting a number of different serialization content types. Errors compatible with
//! the `Error` trait are also automatically serialized into the body of an error response and
//! deserialized into a Rust `Result` in the client.

pub mod client;
pub mod error;
pub mod server;
pub mod tagged_blob;
pub mod types;

pub use error::*;
pub use tagged_blob::*;
pub use types::*;
