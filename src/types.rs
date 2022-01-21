use ark_serialize::*;
use commit::{Commitment, Committable};
use fmt::{Debug, Display, Formatter};
use generic_array::{ArrayLength, GenericArray};
use jf_aap::{
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

// UserAddress from jf_aap is just a type alias for VerKey, which serializes with the tag VERKEY,
// which is confusing. This newtype struct lets us a define a more user-friendly tag.
#[tagged_blob("ADDR")]
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize, PartialEq, Eq, Hash)]
pub struct UserAddress(pub jf_aap::keys::UserAddress);

impl From<jf_aap::keys::UserAddress> for UserAddress {
    fn from(addr: jf_aap::keys::UserAddress) -> Self {
        Self(addr)
    }
}

impl From<UserAddress> for jf_aap::keys::UserAddress {
    fn from(addr: UserAddress) -> Self {
        addr.0
    }
}

pub use jf_aap::keys::UserPubKey;

#[tagged_blob("RECPROOF")]
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize, PartialEq, Eq)]
pub struct MerklePath(pub jf_aap::MerklePath);

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
