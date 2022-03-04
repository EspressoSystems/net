// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the Net library.

// This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.

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
