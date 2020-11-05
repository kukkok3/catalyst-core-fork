use crate::testing::{
    network_builder::NodeAlias,
    node::{grpc::JormungandrClient, uri_from_socket_addr, JormungandrLogger, JormungandrRest},
    FragmentNode, FragmentNodeError, MemPoolCheck, SyncNode,
};
use chain_core::property::Fragment as _;
use chain_impl_mockchain::{fragment::Fragment, fragment::FragmentId};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{BlockDate, FragmentLog, NodeConfig},
};
use std::{collections::HashMap, net::SocketAddr, path::PathBuf};

pub struct RemoteJormungandr {
    rest: Option<JormungandrRest>,
    grpc: Option<JormungandrClient>,
    logger: Option<JormungandrLogger>,
    node_config: Option<NodeConfig>,
    alias: NodeAlias,
}

impl RemoteJormungandr {
    pub fn new(
        rest: Option<JormungandrRest>,
        grpc: Option<JormungandrClient>,
        logger: Option<JormungandrLogger>,
        node_config: Option<NodeConfig>,
        alias: NodeAlias,
    ) -> Self {
        Self {
            rest,
            grpc,
            logger,
            node_config,
            alias,
        }
    }

    pub fn alias(&self) -> &NodeAlias {
        &self.alias
    }

    pub fn rest(&self) -> &JormungandrRest {
        self.rest.as_ref().unwrap()
    }

    pub fn grpc(&self) -> &JormungandrClient {
        self.grpc.as_ref().unwrap()
    }

    pub fn logger(&self) -> &JormungandrLogger {
        self.logger.as_ref().unwrap()
    }

    pub fn node_config(&self) -> &NodeConfig {
        self.node_config.as_ref().unwrap()
    }
}

impl SyncNode for RemoteJormungandr {
    fn alias(&self) -> &str {
        self.alias()
    }

    fn last_block_height(&self) -> u32 {
        let docs = self.rest().stats().unwrap();
        docs.stats
            .expect("no stats object in response")
            .last_block_height
            .expect("last_block_height field is missing")
            .parse()
            .unwrap()
    }

    fn log_stats(&self) {
        println!("{:?}", self.rest().stats());
    }

    fn tip(&self) -> Hash {
        self.rest().tip().expect("cannot get tip from rest")
    }

    fn log_content(&self) -> String {
        self.logger().get_log_content()
    }

    fn get_lines_with_error_and_invalid(&self) -> Vec<String> {
        self.logger().get_lines_with_error_and_invalid().collect()
    }

    fn is_running(&self) -> bool {
        todo!()
    }
}

impl FragmentNode for RemoteJormungandr {
    fn alias(&self) -> &str {
        self.alias()
    }
    fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, FragmentNodeError> {
        self.rest()
            .fragment_logs()
            .map_err(|e| FragmentNodeError::ListFragmentError(e.to_string()))
    }
    fn send_fragment(&self, fragment: Fragment) -> Result<MemPoolCheck, FragmentNodeError> {
        self.rest().send_fragment(fragment.clone()).map_err(|e| {
            FragmentNodeError::CannotSendFragment {
                reason: e.to_string(),
                alias: self.alias().to_string(),
                fragment_id: fragment.id(),
                logs: FragmentNode::log_content(self),
            }
        })
    }

    fn send_batch_fragments(
        &self,
        fragments: Vec<Fragment>,
    ) -> Result<Vec<MemPoolCheck>, FragmentNodeError> {
        self.rest()
            .send_fragment_batch(fragments.clone())
            .map_err(|e| FragmentNodeError::CannotSendFragmentBatch {
                reason: e.to_string(),
                alias: self.alias().to_string(),
                fragment_ids: fragments.iter().map(|x| x.id()).collect(),
                logs: FragmentNode::log_content(self),
            })
    }

    fn log_pending_fragment(&self, fragment_id: FragmentId) {
        println!("Fragment '{}' is still pending", fragment_id);
    }
    fn log_rejected_fragment(&self, fragment_id: FragmentId, reason: String) {
        println!("Fragment '{}' rejected: {}", fragment_id, reason);
    }
    fn log_in_block_fragment(&self, fragment_id: FragmentId, date: BlockDate, block: Hash) {
        println!("Fragment '{}' in block: {} ({})", fragment_id, block, date);
    }
    fn log_content(&self) -> Vec<String> {
        self.logger().get_lines_from_log().collect()
    }
}

pub struct RemoteJormungandrBuilder {
    rest: Option<JormungandrRest>,
    grpc: Option<JormungandrClient>,
    logger: Option<JormungandrLogger>,
    node_config: Option<NodeConfig>,
    node_alias: NodeAlias,
}

impl RemoteJormungandrBuilder {
    pub fn new(node_alias: NodeAlias) -> Self {
        Self {
            rest: None,
            grpc: None,
            logger: None,
            node_config: None,
            node_alias,
        }
    }

    pub fn from_config(&mut self, node_config_path: PathBuf) -> &mut Self {
        self.with_node_config(node_config_path);
        let node_config = self.node_config.clone().unwrap();

        let rest_address = node_config.rest.listen;
        self.with_rest(rest_address);

        let grpc_address = node_config.p2p.get_listen_address();
        self.with_grpc(grpc_address.to_string());

        if let Some(log) = node_config.log {
            if let Some(path) = log.file_path() {
                self.with_logger(PathBuf::from(path));
            }
        }
        self
    }

    pub fn with_rest(&mut self, address: SocketAddr) -> &mut Self {
        self.rest = Some(JormungandrRest::new(uri_from_socket_addr(address)));
        self
    }

    pub fn with_grpc<S: Into<String>>(&mut self, address: S) -> &mut Self {
        self.grpc = Some(JormungandrClient::from_address(&address.into()).unwrap());
        self
    }

    pub fn with_logger(&mut self, log_file: PathBuf) -> &mut Self {
        self.logger = Some(JormungandrLogger::new(log_file));
        self
    }

    pub fn with_node_config(&mut self, node_config: PathBuf) -> &mut Self {
        self.node_config =
            Some(serde_yaml::from_str(&jortestkit::file::read_file(node_config)).unwrap());
        self
    }

    pub fn build(self) -> RemoteJormungandr {
        RemoteJormungandr::new(
            self.rest,
            self.grpc,
            self.logger,
            self.node_config,
            self.node_alias,
        )
    }
}
