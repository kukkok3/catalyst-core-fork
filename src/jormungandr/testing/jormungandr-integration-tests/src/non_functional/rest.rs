use crate::startup;
use jormungandr_automation::jormungandr::{Block0ConfigurationBuilder, NodeConfigBuilder};
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, KesUpdateSpeed};
use jortestkit::load::{self, ConfigurationBuilder as LoadConfigurationBuilder, Monitor};
use mjolnir::generators::RestRequestGen;
use std::time::Duration;

#[test]
pub fn rest_load_quick() {
    let faucet = thor::Wallet::default();

    let (mut jormungandr, _) = startup::start_stake_pool(
        &[faucet],
        &[],
        Block0ConfigurationBuilder::default()
            .with_slots_per_epoch(60.try_into().unwrap())
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4.try_into().unwrap())
            .with_epoch_stability_depth(10.try_into().unwrap())
            .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap()),
        NodeConfigBuilder::default(),
    )
    .unwrap();

    jormungandr.steal_temp_dir().unwrap().into_persistent();

    let rest_client = jormungandr.rest();
    let request = RestRequestGen::new(rest_client);
    let config = LoadConfigurationBuilder::duration(Duration::from_secs(40))
        .thread_no(5)
        .step_delay(Duration::from_millis(10))
        .monitor(Monitor::Progress(100))
        .status_pace(Duration::from_secs(1_000))
        .build();
    let stats = load::start_sync(request, config, "Jormungandr rest load test");
    assert!((stats.calculate_passrate() as u32) > 95);
}
