use std::{collections::BTreeMap, ffi::OsString};

use bigdecimal::BigDecimal;
use bytekind::{Bytes, HexString};
use clap::builder::OsStr;
use hex::FromHexError;
use microtype::microtype;
use serde::{Deserialize, Serialize};

pub(crate) mod arbitrary;
mod cbor;
// mod crypto;
pub use crypto2::{PubKey, Sig};
mod crypto2;
// pub use crypto::{PubKey, Sig};
pub mod hex_bytes;
mod network_id;
pub use network_id::NetworkId;
use test_strategy::Arbitrary;

/// The source of voting power for a given registration
///
/// The voting power can either come from:
///  - a single wallet, OR
///  - a set of delegations
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
#[derive(Debug, Clone, PartialEq)]
pub enum VotingPowerSource {
    /// Direct voting
    ///
    /// Voting power is based on the staked ada of the given key
    Direct(VotingKeyHex),

    /// Delegated voting
    ///
    /// Voting power is based on the staked ada of the delegated keys
    Delegated(BTreeMap<VotingKeyHex, BigDecimal>),
}

impl VotingPowerSource {
    /// Create a direct voting power source from a hex string representing a voting key
    ///
    /// # Errors
    ///
    /// Returns an error if `s` is not a hex string representing an array of 32 bytes (i.e. a 64
    /// character string)
    #[inline]
    pub fn direct_from_hex(s: &str) -> Result<Self, FromHexError> {
        let mut bytes = [0; 32];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Self::Direct(PubKey(bytes).into()))
    }
}

/// A catalyst registration on Cardano in either CIP-15 or CIP-36 format
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Registration {
    #[serde(rename = "1")]
    pub voting_power_source: VotingPowerSource,
    #[serde(rename = "2")]
    pub stake_key: StakeKeyHex,
    #[serde(rename = "3")]
    pub rewards_address: RewardsAddress,
    // note, this must be monotonically increasing. Typically, the current slot
    // number is used
    #[serde(rename = "4")]
    pub nonce: Nonce,
    #[serde(rename = "5")]
    pub voting_purpose: Option<VotingPurpose>,
}

/// A signature for a registration as defined in CIP-15
///
/// This is compatible with CIP-36 registrations
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Signature {
    /// The actual signature
    ///
    /// CIP-15 specifies this must be a field, so an extra layer of nesting is required
    #[serde(rename = "1")]
    pub inner: Sig,
}

/// A Catalyst registration in either CIP-15 or CIP-36 format, along with its signature
///
/// The signature is generated by:
///  - CBOR encoding the registration as a single entry map with a key of `61284` and a value of
///  the registration
///  - Hashing the bytes of the CBOR encoding with `blake2b_2561`
///  - Signing the hash with the public key
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SignedRegistration {
    /// The registration
    pub registration: Registration,
    /// The signature
    pub signature: Signature,

    /// The id of the transaction that created this registration
    pub tx_id: TxId,
}

/// Single element in a snapshot
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotEntry {
    /// Registration content
    pub voting_power_source: VotingPowerSource,

    /// Mainnet rewards address
    pub rewards_address: RewardsAddress,

    /// Stake public key
    pub stake_key: StakeKeyHex,

    /// Voting power expressed in ada
    ///
    /// This is computed from `voting_power_source`
    pub voting_power: BigDecimal,

    /// Voting purpose
    ///
    /// Catalyst expects the voting purpose is set to `0`
    pub voting_purpose: Option<VotingPurpose>,

    /// Registration transaction id
    pub tx_id: TxId,
}

// Create newtype wrappers for better type safety
microtype! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Arbitrary)]
    pub PubKey {
        VotingKeyHex,
        StakeKeyHex,
    }


    #[derive(Debug, PartialEq, Clone)]
    #[string]
    pub String {
        /// Database name
        DbName,
        /// Database user
        DbUser,
        /// Database host
        DbHost,
        StakeAddr,
        StakePubKey,
    }

    #[secret]
    #[string]
    pub String {
        /// Database password
        DbPass,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Arbitrary)]
    #[int]
    pub u64 {
        Nonce,
        /// A slot number
        SlotNo,

        /// A `u64` used to identify the purpose of a particular registration
        ///
        /// `0` is used for catalyst voting
        VotingPurpose,
        TxId,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub Bytes<HexString> {
        /// A rewards address in a catalyst registation
        ///
        /// This type deliberately does not enforce a particular format for addresses, since the
        /// spec only requires this field to be a byte array, with no other constraints
        RewardsAddress,
    }
}

impl VotingPurpose {
    /// The voting purpose for catalyst registrations
    pub const CATALYST: VotingPurpose = VotingPurpose(0);
}

impl From<VotingPurpose> for OsStr {
    fn from(purpose: VotingPurpose) -> Self {
        OsStr::from(OsString::from(purpose.to_string()))
    }
}

impl From<NetworkId> for OsStr {
    fn from(id: NetworkId) -> Self {
        OsStr::from(match id {
            NetworkId::Mainnet => "mainnet",
            NetworkId::Testnet => "testnet",
        })
    }
}

impl SlotNo {
    /// Attempt to convert this to an `i64`
    ///
    /// Returns none if the underlying `u64` doesn't fit into an `i64`
    #[inline]
    #[must_use]
    pub fn into_i64(self) -> Option<i64> {
        self.0.try_into().ok()
    }
}

impl RewardsAddress {
    /// Decode a [`RewardsAddress`] from a hex string
    ///
    /// Errors if the string is not valid hex
    ///
    /// ```
    /// # use crate::data::RewardsAddress;
    /// let address = RewardsAddress::from_hex("0000").unwrap();
    /// assert_eq!(address.0, vec![0, 0]);
    /// ```
    #[inline]
    pub fn from_hex(s: &str) -> Result<Self, FromHexError> {
        let bytes = hex::decode(s)?;
        Ok(Self(bytes.into()))
    }
}

macro_rules! hex_impls {
    ($t:ty) => {
        impl core::fmt::Display for $t {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "0x{}", hex::encode(&self.0))
            }
        }

        impl AsRef<[u8]> for $t {
            fn as_ref(&self) -> &[u8] {
                self.0.as_ref()
            }
        }
    };
}

hex_impls!(VotingKeyHex);
hex_impls!(StakeKeyHex);
