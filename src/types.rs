// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the Net library.

// This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.

use ark_serialize::*;
use commit::{Commitment, Committable};
use fmt::{Debug, Display, Formatter};
use generic_array::{ArrayLength, GenericArray};
use jf_cap::{
    structs::{ReceiverMemo, RecordCommitment},
    Signature,
};
use jf_utils::tagged_blob;
use serde::{Deserialize, Serialize};
use std::fmt;

#[tagged_blob("HASH")]
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize, PartialEq, Eq)]
pub struct Hash(pub Vec<u8>);

impl<const N: usize> From<[u8; N]> for Hash {
    fn from(h: [u8; N]) -> Self {
        Self(h.as_ref().to_vec())
    }
}

impl<U: ArrayLength<u8>> From<GenericArray<u8, U>> for Hash {
    fn from(a: GenericArray<u8, U>) -> Self {
        Self((&*a).to_vec())
    }
}

impl<T: Committable> From<Commitment<T>> for Hash {
    fn from(c: Commitment<T>) -> Self {
        Self::from(<[u8; 32]>::from(c))
    }
}

#[tagged_blob("BK")]
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize, PartialEq, Eq)]
pub struct BlockId(pub usize);

#[tagged_blob("TX")]
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize, PartialEq, Eq)]
pub struct TransactionId(pub BlockId, pub usize);

// UserAddress from jf_cap is just a type alias for VerKey, which serializes with the tag VERKEY,
// which is confusing. This newtype struct lets us a define a more user-friendly tag.
#[tagged_blob("ADDR")]
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize, PartialEq, Eq, Hash)]
pub struct UserAddress(pub jf_cap::keys::UserAddress);

impl From<jf_cap::keys::UserAddress> for UserAddress {
    fn from(addr: jf_cap::keys::UserAddress) -> Self {
        Self(addr)
    }
}

impl From<UserAddress> for jf_cap::keys::UserAddress {
    fn from(addr: UserAddress) -> Self {
        addr.0
    }
}

pub use jf_cap::keys::UserPubKey;

#[tagged_blob("RECPROOF")]
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize, PartialEq, Eq)]
pub struct MerklePath(pub jf_cap::MerklePath);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnspentRecord {
    pub commitment: RecordCommitment,
    pub uid: u64,
    pub memo: Option<ReceiverMemo>,
}

impl Display for UnspentRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt_as_json(self, f)
    }
}

/// Request body for the bulletin board endpoint POST /memos.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostMemos {
    pub memos: Vec<ReceiverMemo>,
    pub signature: Signature,
}

impl Display for PostMemos {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt_as_json(self, f)
    }
}

// Display implementation for types which serialize to JSON. Displays as a valid JSON object.
pub fn fmt_as_json<T: Serialize>(v: &T, f: &mut Formatter<'_>) -> fmt::Result {
    let string = serde_json::to_string(v).map_err(|_| fmt::Error)?;
    write!(f, "{}", string)
}
