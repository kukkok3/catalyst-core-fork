mod controller;
pub mod mock;
pub mod result;
mod starter;
mod voter_hirs_asserts;

use crate::common::snapshot::result::{do_snapshot as do_snapshot_internal, SnapshotResult};
pub use controller::SnapshotServiceController;
use snapshot_trigger_service::config::JobParameters;
pub use starter::SnapshotServiceStarter;
use thiserror::Error;
pub use voter_hirs_asserts::RegistrationAsserts;

pub fn do_snapshot(job_params: JobParameters) -> Result<SnapshotResult, result::Error> {
    let snapshot_token = std::env::var("SNAPSHOT_TOKEN").expect("SNAPSHOT_TOKEN not defined");
    let snapshot_address = std::env::var("SNAPSHOT_ADDRESS").expect("SNAPSHOT_ADDRESS not defined");

    do_snapshot_internal(job_params, snapshot_token, snapshot_address)
}

pub fn wait_for_db_sync() {
    println!("Waiting 5 mins before running snapshot");
    std::thread::sleep(std::time::Duration::from_secs(5 * 60));
    println!("Wait finished.");
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("spawn error")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Config(#[from] snapshot_trigger_service::config::Error),
    #[error("cannot bootstrap snapshot service on port {0}")]
    Bootstrap(u16),
}
