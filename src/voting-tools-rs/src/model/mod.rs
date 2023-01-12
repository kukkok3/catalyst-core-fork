use bigdecimal::BigDecimal;
use cardano_serialization_lib::{address::NetworkInfo, chain_crypto::ed25519::Pub};
use microtype::microtype;
use serde::{Deserialize, Serialize};

use crate::data::crypto::PublicKeyHex;

/// The source of voting power for a given registration
///
/// The voting power can either come from: 
///  - a single wallet, OR
///  - a set of delegations
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
#[derive(Debug, Clone, PartialEq)]
pub enum VotingPowerSource {
    /// Direct voting. String should contain catalyst identifier
    Legacy(Pub),

    /// Delegated one. Collection of catalyst identifiers joined it weights
    Delegated(Vec<(Pub, u32)>),
}

/// A registration on Cardano in either CIP-15 or CIP-36 format
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Registration {
    #[serde(rename = "1")]
    pub voting_power_source: VotingPowerSource,
    #[serde(rename = "2")]
    pub stake_key: StakeKeyHex,
    #[serde(rename = "3")]
    pub rewards_addr: RewardsAddr,
    // note, this must be monotonically increasing. Typically, the current slot
    // number is used
    #[serde(rename = "4")]
    pub nonce: Nonce,  
    #[serde(rename = "5")]
    #[serde(default)]
    pub purpose: VotingPurpose,
}

/// A signature for a registration as defined in CIP-15
///
/// This is compatible with CIP-36 registrations
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Signature {
    #[serde(rename = "1")]
    pub inner: SignatureHex,
}

/// A Catalyst registration in either CIP-15 or CIP-36 format, along with its signature
///
/// The signature is generated by:
///  - CBOR encoding the registration as a single entry map with a key of `61284` and a value of
///  the registration
///  - Hashing the bytes of the CBOR encoding with blake2b_256
///  - Signing the hash with the public key 
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SignedRegistration {
    /// The registration
    pub registration: Registration,
    /// The signature
    pub signature: Signature,
}

/// Single output element of voting tools calculations
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Output {
    /// registration content
    pub voting_power_source: VotingPowerSource,

    /// mainnet rewards address
    pub rewards_address: RewardsAddr,

    /// stake public key
    pub stake_public_key: StakePubKey,

    /// voting power expressed in ada
    pub voting_power: BigDecimal,

    /// voting purpose
    ///
    /// Catalyst expects the voting purpose is set to `0`
    pub voting_purpose: VotingPurpose,

    /// registration transaction id
    pub tx_id: TxId,
}


#[derive(Debug, Clone, PartialEq)]
pub struct Reg {
    pub tx_id: TxId,
    pub metadata: Registration,
    pub signature: Signature,
}

// Create newtype wrappers for better type safety
microtype! {


    #[derive(Debug, PartialEq, Clone)]
    #[string]
    pub String {
        /// Database name
        DbName,
        /// Database user
        DbUser,
        /// Database host
        DbHost,
        RewardsAddr,
        StakeAddr,
        StakePubKey,
        SignatureHex,
        #[derive(Hash, PartialOrd, Eq)]
        StakeKeyHex,
    }

    #[secret]
    #[string]
    pub String {
        /// Database password
        DbPass,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
    #[int]
    pub u64 {
        #[cfg_attr(test, derive(test_strategy::Arbitrary))]
        Nonce,
        #[cfg_attr(test, derive(test_strategy::Arbitrary))]
        SlotNo,
        VotingPurpose,
        TxId,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
    #[int]
    pub u32 {
        TestnetMagic
    }
}

impl RewardsAddr {
    pub fn without_leading_0x(&self) -> Self {
        self.trim_start_matches("0x").into()
    }
}

pub fn network_info(testnet_magic: Option<TestnetMagic>) -> NetworkInfo {
    match testnet_magic {
        None => NetworkInfo::mainnet(),
        Some(TestnetMagic(magic)) => {
            NetworkInfo::new(NetworkInfo::testnet_preview().network_id(), magic)
        }
    }
}

impl SlotNo {
    pub fn into_i64(self) -> color_eyre::eyre::Result<i64> {
        Ok(self.0.try_into()?)
    }
}
