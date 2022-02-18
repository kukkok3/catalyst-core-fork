use super::WalletProxySettings;
use std::net::SocketAddr;
use valgrind::{Certs, VitVersion};

pub struct WalletProxySpawnParams {
    pub alias: String,
    pub base_address: Option<SocketAddr>,
    pub certs: Option<Certs>,
    pub version: VitVersion,
}

impl WalletProxySpawnParams {
    pub fn new<S: Into<String>>(alias: S) -> Self {
        Self {
            alias: alias.into(),
            base_address: None,
            certs: None,
            version: VitVersion {
                service_version: "2.0".to_string(),
            },
        }
    }

    pub fn with_base_address<S: Into<String>>(&mut self, base_address: S) -> &mut Self {
        self.base_address = Some(base_address.into().parse().unwrap());
        self
    }

    pub fn with_certs(&mut self, certs: Certs) -> &mut Self {
        self.certs = Some(certs);
        self
    }

    pub fn with_version(&mut self, version: VitVersion) -> &mut Self {
        self.version = version;
        self
    }

    pub fn override_settings(&self, settings: &mut WalletProxySettings) {
        if let Some(address) = self.base_address {
            settings.proxy_address = address;
        }
    }
}
