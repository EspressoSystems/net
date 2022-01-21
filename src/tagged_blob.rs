use ark_serialize::*;
use fmt::Debug;
use jf_utils::Tagged;
use snafu::{ResultExt, Snafu};
use std::fmt;
use tagged_base64::TaggedBase64;

// Helper trait with a blanket implementation allowing us to convert TaggedBase64 to any type which
// implements Tagged and CanonicalDeserialize.
pub trait TaggedBlob: Sized + Tagged + CanonicalDeserialize {
    fn from_tagged_blob(b64: &TaggedBase64) -> Result<Self, TaggedBlobError>;
}

#[derive(Debug, Snafu)]
pub enum TaggedBlobError {
    SerError { source: SerializationError },
    TagMismatch { actual: String, expected: String },
}

impl<T: Tagged + CanonicalDeserialize> TaggedBlob for T {
    fn from_tagged_blob(b64: &TaggedBase64) -> Result<Self, TaggedBlobError> {
        if b64.tag() == Self::tag() {
            Self::deserialize(&*b64.value()).context(SerError)
        } else {
            Err(TaggedBlobError::TagMismatch {
                actual: b64.tag(),
                expected: Self::tag(),
            })
        }
    }
}
