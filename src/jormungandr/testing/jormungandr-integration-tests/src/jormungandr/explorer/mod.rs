use crate::common::{
    jcli_wrapper, jormungandr::ConfigurationBuilder, process_utils::Wait, startup,
    transaction_utils::TransactionHash,
};

use chain_impl_mockchain::fragment::FragmentId;
use chain_impl_mockchain::key::Hash;
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use jormungandr_testing_utils::stake_pool::StakePool;
use jormungandr_testing_utils::testing::node::Explorer;
use std::str::FromStr;
use std::time::Duration;

/// test checks if there is upto date schema
/// prereq:
/// -npm
/// read more: https://github.com/prisma-labs/get-graphql-schema
#[test]
#[cfg(feature = "explorer-schema-gen")]
#[cfg(unix)]
pub fn explorer_schema_diff_test() {
    use crate::common::jormungandr::Starter;
    use assert_fs::{assert::PathAssert, fixture::PathChild, TempDir};

    let temp_dir = TempDir::new().unwrap();
    let config = ConfigurationBuilder::new().with_explorer().build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let schema_temp_dir = TempDir::new().unwrap();
    let actual_schema_path = schema_temp_dir.child("new_schema.graphql");

    Command::new("../jormungandr-testing-utils/resources/explorer/graphql/generate_schema.sh")
        .args(&[
            jormungandr.explorer().uri(),
            actual_schema_path
                .path()
                .as_os_str()
                .to_str()
                .unwrap()
                .to_string(),
        ])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    jormungandr_testing_utils::testing::node::explorer::compare_schema(actual_schema_path.path());
}

#[test]
pub fn explorer_sanity_test() {
    let mut faucet = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();

    let mut config = ConfigurationBuilder::new();
    config
        .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
        .with_explorer();

    let (jormungandr, initial_stake_pools) =
        startup::start_stake_pool(&[faucet.clone()], &[], &mut config).unwrap();

    let transaction = faucet
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            receiver.address(),
            1_000.into(),
        )
        .unwrap()
        .encode();

    let wait = Wait::new(Duration::from_secs(3), 20);
    let fragment_id =
        jcli_wrapper::assert_transaction_in_block_with_wait(&transaction, &jormungandr, &wait);

    let explorer = jormungandr.explorer();

    transaction_by_id(&explorer, fragment_id.into_hash());
    blocks(&explorer, jormungandr.logger.get_created_blocks_hashes());
    stake_pools(&explorer, &initial_stake_pools);
    stake_pool(&explorer, &initial_stake_pools);
    block_at_chain_length(&explorer, jormungandr.logger.get_created_blocks_hashes());
    epoch(&explorer);
}

fn transaction_by_id(explorer: &Explorer, fragment_id: FragmentId) {
    let explorer_transaction = explorer
        .transaction(fragment_id.into())
        .expect("non existing transaction");

    assert_eq!(
        fragment_id,
        Hash::from_str(&explorer_transaction.data.unwrap().transaction.id).unwrap(),
        "incorrect fragment id"
    );
}

fn blocks(explorer: &Explorer, blocks_from_logs: Vec<Hash>) {
    let blocks = explorer.blocks(1000).unwrap();
    // we are skipping first block because log doesn't contains genesis block
    assert_eq!(
        blocks_from_logs,
        blocks
            .data
            .unwrap()
            .all_blocks
            .edges
            .iter()
            .skip(1)
            .map(|x| Hash::from_str(&x.node.id).unwrap())
            .collect::<Vec<Hash>>(),
        "blocks are empty"
    );
}

fn stake_pools(explorer: &Explorer, initial_stake_pools: &[StakePool]) {
    let stake_pools = explorer.stake_pools(1000).unwrap();
    let explorer_stake_pools = stake_pools.data.unwrap().all_stake_pools.edges;
    // we are skipping first block because log doesn't contains genesis block
    assert_eq!(
        initial_stake_pools
            .iter()
            .map(|x| x.id().to_string())
            .collect::<Vec<String>>(),
        explorer_stake_pools
            .iter()
            .map(|x| x.node.id.clone())
            .collect::<Vec<String>>(),
        "blocks are empty"
    );
}

fn stake_pool(explorer: &Explorer, initial_stake_pools: &[StakePool]) {
    let stake_pool_id = initial_stake_pools.first().unwrap().id().to_string();
    let stake_pool = explorer.stake_pool(stake_pool_id, 100).unwrap();
    let explorer_stake_pool_id = stake_pool.data.unwrap().stake_pool.id.clone();

    assert!(
        initial_stake_pools
            .iter()
            .any(|x| x.id().to_string() == explorer_stake_pool_id),
        "stake pool id"
    );
}

fn block_at_chain_length(explorer: &Explorer, blocks_from_logs: Vec<Hash>) {
    let block = explorer.block_at_chain_length(1).unwrap();

    assert_eq!(
        blocks_from_logs.first().unwrap().to_string(),
        block.data.unwrap().block_by_chain_length.unwrap().id,
        "can't find block"
    );
}

fn epoch(explorer: &Explorer) {
    let epoch = explorer.epoch(1, 100).unwrap();

    assert_eq!(epoch.data.unwrap().epoch.id, "1", "can't find epoch");
}
