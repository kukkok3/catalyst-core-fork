use crate::{
    scenario::{
        Blockchain as BlockchainTemplate, Context, Node as NodeTemplate,
        Topology as TopologyTemplate, Wallet as WalletTemplate,
    },
    NodeAlias, Wallet, WalletAlias, WalletType,
};
use chain_crypto::{Curve25519_2HashDH, Ed25519, SumEd25519_12};
use chain_impl_mockchain::{block::ConsensusVersion, fee::LinearFee};
use jormungandr_lib::{
    crypto::{hash::Hash, key::SigningKey},
    interfaces::{Block0Configuration, BlockchainConfiguration, Initial, InitialUTxO},
};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};

#[derive(Debug)]
pub struct Settings {
    pub nodes: HashMap<NodeAlias, NodeSetting>,

    pub wallets: HashMap<WalletAlias, Wallet>,

    pub block0: Block0Configuration,
}

/// contains all the data to start or interact with a node
#[derive(Debug)]
pub struct NodeSetting {
    /// for reference purpose only
    pub alias: NodeAlias,

    /// node secret, this will be passed to the node at start
    /// up of the node. It may contains the necessary crypto
    /// for the node to be a blockchain leader (BFT leader or
    /// stake pool)
    pub secret: NodeSecret,

    pub config: NodeConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeConfig {
    pub rest: Rest,

    pub p2p: P2pConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rest {
    pub listen: SocketAddr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2pConfig {
    /// The public address to which other peers may connect to
    pub public_address: String,

    /// the node identifier
    pub id: poldercast::Id,

    /// the rendezvous points for the peer to connect to in order to initiate
    /// the p2p discovery from.
    pub trusted_peers: Vec<TrustedPeer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustedPeer {
    address: String,
    id: poldercast::Id,
}

/// Node Secret(s)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeSecret {
    bft: Option<Bft>,
    genesis: Option<GenesisPraos>,
}

/// hold the node's bft secret setting
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Bft {
    signing_key: SigningKey<Ed25519>,
}

/// the genesis praos setting
///
#[derive(Debug, Clone, Deserialize, Serialize)]
struct GenesisPraos {
    node_id: Hash,
    sig_key: SigningKey<SumEd25519_12>,
    vrf_key: SigningKey<Curve25519_2HashDH>,
}

impl Settings {
    pub fn prepare<RNG>(
        topology: TopologyTemplate,
        blockchain: BlockchainTemplate,
        context: &mut Context<RNG>,
    ) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        let mut settings = Settings {
            nodes: topology
                .aliases()
                .map(|alias| (alias.clone(), NodeSetting::prepare(alias.clone(), context)))
                .collect(),
            wallets: HashMap::new(),
            block0: Block0Configuration {
                blockchain_configuration: BlockchainConfiguration::new(
                    chain_addr::Discrimination::Test,
                    ConsensusVersion::Bft,
                    LinearFee::new(1, 2, 3),
                ),
                initial: Vec::new(),
            },
        };

        settings.populate_trusted_peers(topology.into_iter());
        settings.populate_block0_blockchain_configuration(&blockchain, context);
        settings.populate_block0_blockchain_initials(blockchain.wallets(), context);

        settings
    }

    fn populate_block0_blockchain_initials<'a, RNG, I>(
        &'a mut self,
        wallet_templates: I,
        context: &mut Context<RNG>,
    ) where
        RNG: RngCore + CryptoRng,
        I: Iterator<Item = &'a WalletTemplate>,
    {
        let discrimination = self.block0.blockchain_configuration.discrimination;

        for wallet_template in wallet_templates {
            // TODO: check the wallet does not already exist ?
            let wallet = match wallet_template.wallet_type() {
                WalletType::UTxO => Wallet::generate_utxo(context.rng_mut()),
                WalletType::Account => Wallet::generate_account(context.rng_mut()),
            };

            let initial_address = wallet.address(discrimination);

            // TODO add support for sharing fragment with multiple utxos
            let initial_fragment = Initial::Fund(vec![InitialUTxO {
                address: initial_address,
                value: (*wallet_template.value()).into(),
            }]);

            self.wallets
                .insert(wallet_template.alias().clone(), wallet.clone());
            self.block0.initial.push(initial_fragment);

            if let Some(delegation) = wallet_template.delegate() {
                use chain_impl_mockchain::certificate::{
                    Certificate, CertificateContent, StakeDelegation,
                };
                use chain_impl_mockchain::stake::StakePoolId;

                // 1. retrieve the public data (we may need to create a stake pool
                //    registration here)
                let stake_pool_id: StakePoolId = if let Some(node) = self.nodes.get_mut(delegation)
                {
                    if let Some(genesis) = &node.secret.genesis {
                        StakePoolId::from(genesis.node_id.clone().into_hash())
                    } else {
                        // create and register the stake pool
                        use chain_impl_mockchain::{
                            leadership::genesis::GenesisPraosLeader, stake::StakePoolInfo,
                        };
                        use rand::{distributions::Standard, Rng as _};
                        let serial: u128 = context.rng_mut().sample(Standard);
                        let kes_signing_key = SigningKey::generate(context.rng_mut());
                        let vrf_signing_key = SigningKey::generate(context.rng_mut());
                        let stake_pool_info = StakePoolInfo {
                            serial,
                            owners: Vec::new(),
                            initial_key: GenesisPraosLeader {
                                kes_public_key: kes_signing_key.identifier().into_public_key(),
                                vrf_public_key: vrf_signing_key.identifier().into_public_key(),
                            },
                        };
                        let node_id = stake_pool_info.to_id();
                        node.secret.genesis = Some(GenesisPraos {
                            sig_key: kes_signing_key,
                            vrf_key: vrf_signing_key,
                            node_id: {
                                let bytes: [u8; 32] = node_id.clone().into();
                                bytes.into()
                            },
                        });

                        let stake_pool_registration_certificate = Certificate {
                            signatures: Vec::new(),
                            content: CertificateContent::StakePoolRegistration(stake_pool_info),
                        };

                        self.block0
                            .initial
                            .push(Initial::Cert(stake_pool_registration_certificate.into()));

                        node_id
                    }
                } else {
                    // delegating to a node that does not exist in the topology
                    // so generate valid stake pool registration and delegation
                    // to that node.
                    unimplemented!("delegating stake to a stake pool that is not a node is not supported (yet)")
                };

                // 2. retrieve the wallet delegation identifier
                let stake_key_id = if let Some(stake_key_id) = wallet.stake_key() {
                    stake_key_id
                } else {
                    unimplemented!(
                        "delegation from a wallet that is not an Account is not supported (yet)"
                    )
                };

                // 3. create delegation certificate and add it to the block0.initial array
                let delegation_certificate = Certificate {
                    content: CertificateContent::StakeDelegation(StakeDelegation {
                        stake_key_id,           // 2
                        pool_id: stake_pool_id, // 1
                    }),
                    signatures: Vec::new(), // Leave empty
                };

                self.block0
                    .initial
                    .push(Initial::Cert(delegation_certificate.into()));
            }
        }
    }

    fn populate_block0_blockchain_configuration<RNG>(
        &mut self,
        blockchain: &BlockchainTemplate,
        context: &mut Context<RNG>,
    ) where
        RNG: RngCore + CryptoRng,
    {
        let mut blockchain_configuration = &mut self.block0.blockchain_configuration;

        // TODO blockchain_configuration.block0_date = ;
        blockchain_configuration.discrimination = chain_addr::Discrimination::Test;
        blockchain_configuration.block0_consensus = *blockchain.consensus();
        blockchain_configuration.consensus_leader_ids = {
            let mut leader_ids = Vec::new();
            for leader_alias in blockchain.leaders() {
                let identifier = if let Some(node) = self.nodes.get_mut(leader_alias) {
                    if let Some(bft) = &node.secret.bft {
                        bft.signing_key.identifier()
                    } else {
                        let signing_key = SigningKey::generate(context.rng_mut());
                        let identifier = signing_key.identifier();
                        node.secret.bft = Some(Bft { signing_key });
                        identifier
                    }
                } else {
                    SigningKey::<Ed25519>::generate(context.rng_mut()).identifier()
                };
                leader_ids.push(identifier.into());
            }
            leader_ids
        };
        blockchain_configuration.slots_per_epoch = *blockchain.slots_per_epoch();
        blockchain_configuration.slot_duration = *blockchain.slot_duration();

        // TODO blockchain_configuration.linear_fees = ;
        // TODO blockchain_configuration.kes_update_speed = ;
        // TODO blockchain_configuration.consensus_genesis_praos_active_slot_coeff = ;
        // TODO blockchain_configuration.bft_slots_ratio = ;
    }

    fn populate_trusted_peers<I>(&mut self, i: I)
    where
        I: Iterator<Item = (NodeAlias, NodeTemplate)>,
    {
        for (alias, node_template) in i {
            let mut trusted_peers = Vec::new();

            for trusted_peer in node_template.trusted_peers() {
                let trusted_peer = self.nodes.get(trusted_peer).unwrap();

                trusted_peers.push(trusted_peer.config.p2p.make_trusted_peer_setting());
            }

            let node = self.nodes.get_mut(&alias).unwrap();
            node.config.p2p.trusted_peers = trusted_peers;
        }
    }
}

impl NodeSetting {
    fn prepare<RNG>(alias: NodeAlias, context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        NodeSetting {
            alias,
            config: NodeConfig::prepare(context),
            secret: NodeSecret::prepare(context),
        }
    }

    pub fn config(&self) -> &NodeConfig {
        &self.config
    }

    pub fn secrets(&self) -> &NodeSecret {
        &self.secret
    }
}

impl NodeSecret {
    pub fn prepare<RNG>(_context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        NodeSecret {
            bft: None,
            genesis: None,
        }
    }
}

impl NodeConfig {
    pub fn prepare<RNG>(context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore,
    {
        NodeConfig {
            rest: Rest::prepare(context),
            p2p: P2pConfig::prepare(context),
        }
    }
}

impl Rest {
    pub fn prepare<RNG>(context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore,
    {
        Rest {
            listen: context.generate_new_rest_listen_address(),
        }
    }
}

impl P2pConfig {
    pub fn prepare<RNG>(context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore,
    {
        P2pConfig {
            public_address: context.generate_new_grpc_public_address(),
            id: poldercast::Id::generate(&mut context.rng_mut()),
            trusted_peers: Vec::new(),
        }
    }

    fn make_trusted_peer_setting(&self) -> TrustedPeer {
        TrustedPeer {
            id: self.id.clone(),
            address: self.public_address.clone(),
        }
    }
}
