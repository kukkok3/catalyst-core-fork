use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::TempDir;
use chain_addr::Discrimination;
use chain_core::property::BlockDate as propertyBlockDate;
use chain_crypto::Ed25519;
use chain_impl_mockchain::{
    block::BlockDate,
    certificate::{UpdateProposal, UpdateVote, VoteAction, VoteTallyPayload},
    fee::LinearFee,
    tokens::minting_policy::MintingPolicy,
    transaction::AccountIdentifier,
    vote::Choice,
};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{
        explorer::{configuration::ExplorerParams, verifiers::ExplorerVerifier},
        Block0ConfigurationBuilder,
    },
    testing::{
        keys::create_new_key_pair,
        time::{wait_for_date, wait_for_epoch},
        VotePlanBuilder,
    },
};
use jormungandr_lib::interfaces::{
    BlockContentMaxSize, ConfigParam, ConfigParams, Initial, InitialToken,
};
use thor::{
    Block0ConfigurationBuilderExtension, BlockDateGenerator::Fixed, FragmentBuilder,
    FragmentSender, StakePool, TransactionHash,
};

const TRANSACTION_CERTIFICATE_QUERY_COMPLEXITY_LIMIT: u64 = 140;
const TRANSACTION_CERTIFICATE_QUERY_DEPTH_LIMIT: u64 = 30;
const SLOTS_PER_EPOCH: u32 = 20;
const SLOT_DURATION: u8 = 2;

#[test]
pub fn explorer_stake_pool_registration_test() {
    let temp_dir = TempDir::new().unwrap();
    let mut first_stake_pool_owner = thor::Wallet::default();
    let first_stake_pool = StakePool::new(&first_stake_pool_owner);

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallet(&first_stake_pool_owner, 1_000_000.into()),
        )
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let settings = jormungandr.rest().settings().unwrap();
    let fragment_sender = FragmentSender::from(&settings);

    let fragment_builder =
        FragmentBuilder::from_settings(&settings, BlockDate::first().next_epoch());

    let params = ExplorerParams::new(
        TRANSACTION_CERTIFICATE_QUERY_COMPLEXITY_LIMIT,
        TRANSACTION_CERTIFICATE_QUERY_DEPTH_LIMIT,
        None,
    );
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&first_stake_pool_owner, &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            stake_pool_reg_fragment.clone(),
            &jormungandr,
        )
        .expect("Error while sending registration certificate for first stake pool owner");

    let trans = explorer
        .transaction_certificates(stake_pool_reg_fragment.hash().into())
        .expect("Non existing stake pool registration transaction");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let exp_stake_pool_reg_transaction = trans.data.unwrap().transaction;

    ExplorerVerifier::assert_transaction_certificates(
        stake_pool_reg_fragment,
        exp_stake_pool_reg_transaction,
    )
    .unwrap();
}

#[test]
pub fn explorer_owner_delegation_test() {
    let temp_dir = TempDir::new().unwrap();
    let mut stake_pool_owner = thor::Wallet::default();
    let stake_pool = StakePool::new(&stake_pool_owner);

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_block0_config(
            Block0ConfigurationBuilder::default().with_wallet(&stake_pool_owner, 1_000_000.into()),
        )
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let settings = jormungandr.rest().settings().unwrap();

    let fragment_sender = FragmentSender::from(&settings);

    let fragment_builder =
        FragmentBuilder::from_settings(&settings, BlockDate::first().next_epoch());

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&stake_pool_owner, &stake_pool);

    fragment_sender
        .send_fragment(&mut stake_pool_owner, stake_pool_reg_fragment, &jormungandr)
        .expect("Error while sending registration certificate for stake pool owner");

    let params = ExplorerParams::new(
        TRANSACTION_CERTIFICATE_QUERY_COMPLEXITY_LIMIT,
        TRANSACTION_CERTIFICATE_QUERY_DEPTH_LIMIT,
        None,
    );
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let owner_deleg_fragment = fragment_builder.owner_delegation(&stake_pool_owner, &stake_pool);

    fragment_sender
        .send_fragment(
            &mut stake_pool_owner,
            owner_deleg_fragment.clone(),
            &jormungandr,
        )
        .expect("Error while sending owner delegation cert");

    let trans = explorer
        .transaction_certificates(owner_deleg_fragment.hash().into())
        .expect("Non existing owner delegation transaction");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let owner_deleg_transaction = trans.data.unwrap().transaction;

    ExplorerVerifier::assert_transaction_certificates(
        owner_deleg_fragment,
        owner_deleg_transaction,
    )
    .unwrap();
}

#[test]
pub fn explorer_full_delegation_test() {
    let temp_dir = TempDir::new().unwrap();
    let mut stake_pool_owner = thor::Wallet::default();
    let mut full_delegator = thor::Wallet::default();

    let stake_pool = StakePool::new(&stake_pool_owner);

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallet(&stake_pool_owner, 1_000_000.into())
                .with_wallet(&full_delegator, 2_000_000.into()),
        )
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let fragment_sender = FragmentSender::from(&jormungandr.rest().settings().unwrap());

    let fragment_builder = FragmentBuilder::from_settings(
        &jormungandr.rest().settings().unwrap(),
        BlockDate::first().next_epoch(),
    );

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&stake_pool_owner, &stake_pool);

    fragment_sender
        .send_fragment(&mut stake_pool_owner, stake_pool_reg_fragment, &jormungandr)
        .expect("Error while sending registration certificate for stake pool owner");

    let params = ExplorerParams::new(
        TRANSACTION_CERTIFICATE_QUERY_COMPLEXITY_LIMIT,
        TRANSACTION_CERTIFICATE_QUERY_DEPTH_LIMIT,
        None,
    );
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let full_deleg_fragment = fragment_builder.delegation(&full_delegator, &stake_pool);

    fragment_sender
        .send_fragment(
            &mut full_delegator,
            full_deleg_fragment.clone(),
            &jormungandr,
        )
        .unwrap();

    let trans = explorer
        .transaction_certificates(full_deleg_fragment.hash().into())
        .expect("Non existing full delegation transaction");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let stake_pool_reg_transaction = trans.data.unwrap().transaction;

    ExplorerVerifier::assert_transaction_certificates(
        full_deleg_fragment,
        stake_pool_reg_transaction,
    )
    .unwrap();
}

#[test]
pub fn explorer_split_delegation_test() {
    let temp_dir = TempDir::new().unwrap();
    let mut first_stake_pool_owner = thor::Wallet::default();
    let mut split_delegator = thor::Wallet::default();
    let mut second_stake_pool_owner = thor::Wallet::default();

    let first_stake_pool = StakePool::new(&first_stake_pool_owner);
    let second_stake_pool = StakePool::new(&second_stake_pool_owner);

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallet(&first_stake_pool_owner, 1_000_000.into())
                .with_wallet(&second_stake_pool_owner, 2_000_000.into())
                .with_wallet(&split_delegator, 2_000_000.into()),
        )
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let fragment_sender = FragmentSender::from(&jormungandr.rest().settings().unwrap());

    let fragment_builder = FragmentBuilder::from_settings(
        &jormungandr.rest().settings().unwrap(),
        BlockDate::first().next_epoch(),
    );
    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&first_stake_pool_owner, &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            stake_pool_reg_fragment,
            &jormungandr,
        )
        .expect("Error while sending registration certificate for stake pool owner");

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&second_stake_pool_owner, &second_stake_pool);

    fragment_sender
        .send_fragment(
            &mut second_stake_pool_owner,
            stake_pool_reg_fragment,
            &jormungandr,
        )
        .expect("Error while sending registration certificate for stake pool owner");

    let params = ExplorerParams::new(
        TRANSACTION_CERTIFICATE_QUERY_COMPLEXITY_LIMIT,
        TRANSACTION_CERTIFICATE_QUERY_DEPTH_LIMIT,
        None,
    );
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let split_delegation_fragment = fragment_builder.delegation_to_many(
        &split_delegator,
        vec![(&first_stake_pool, 1u8), (&second_stake_pool, 1u8)],
    );

    fragment_sender
        .send_fragment(
            &mut split_delegator,
            split_delegation_fragment.clone(),
            &jormungandr,
        )
        .unwrap();

    let trans = explorer
        .transaction_certificates(split_delegation_fragment.hash().into())
        .expect("Non existing split delegation transaction");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let exp_split_delegation_transaction = trans.data.unwrap().transaction;

    ExplorerVerifier::assert_transaction_certificates(
        split_delegation_fragment,
        exp_split_delegation_transaction,
    )
    .unwrap();
}

#[test]
pub fn explorer_pool_update_test() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap();
    let mut first_stake_pool_owner = thor::Wallet::default();
    let second_stake_pool_owner = thor::Wallet::default();
    let first_stake_pool = StakePool::new(&first_stake_pool_owner);

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallet(&first_stake_pool_owner, 1_000_000.into())
                .with_wallet(&second_stake_pool_owner, 2_000_000.into()),
        )
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let fragment_sender = FragmentSender::from(&jormungandr.rest().settings().unwrap());

    let fragment_builder = FragmentBuilder::from_settings(
        &jormungandr.rest().settings().unwrap(),
        BlockDate::first().next_epoch(),
    );

    let params = ExplorerParams::new(
        TRANSACTION_CERTIFICATE_QUERY_COMPLEXITY_LIMIT,
        TRANSACTION_CERTIFICATE_QUERY_DEPTH_LIMIT,
        None,
    );
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&first_stake_pool_owner, &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            stake_pool_reg_fragment,
            &jormungandr,
        )
        .expect("Error while sending registration certificate for first stake pool owner");

    let mut new_stake_pool = first_stake_pool.clone();
    let mut stake_pool_info = new_stake_pool.info_mut();

    stake_pool_info.reward_account = Some(AccountIdentifier::Single(
        second_stake_pool_owner
            .identifier()
            .into_public_key()
            .into(),
    ));

    let stake_pool_update_fragment = fragment_builder.stake_pool_update(
        vec![&first_stake_pool_owner],
        &first_stake_pool,
        &new_stake_pool,
    );

    jcli.fragment_sender(&jormungandr)
        .send(&stake_pool_update_fragment.encode())
        .assert_in_block();
    first_stake_pool_owner.confirm_transaction();

    let trans = explorer
        .transaction_certificates(stake_pool_update_fragment.hash().into())
        .expect("Non existing stake pool update transaction");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let stake_pool_update_transaction = trans.data.unwrap().transaction;

    ExplorerVerifier::assert_transaction_certificates(
        stake_pool_update_fragment,
        stake_pool_update_transaction,
    )
    .unwrap();
}

#[test]
pub fn explorer_pool_retire_test() {
    let temp_dir = TempDir::new().unwrap();
    let mut first_stake_pool_owner = thor::Wallet::default();
    let first_stake_pool = StakePool::new(&first_stake_pool_owner);

    let second_stake_pool_owner = thor::Wallet::default();
    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallet(&first_stake_pool_owner, 1_000_000.into())
                .with_wallet(&second_stake_pool_owner, 2_000_000.into()),
        )
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let fragment_sender = FragmentSender::from(&jormungandr.rest().settings().unwrap());

    let fragment_builder = FragmentBuilder::from_settings(
        &jormungandr.rest().settings().unwrap(),
        BlockDate::first().next_epoch(),
    );

    let params = ExplorerParams::new(
        TRANSACTION_CERTIFICATE_QUERY_COMPLEXITY_LIMIT,
        TRANSACTION_CERTIFICATE_QUERY_DEPTH_LIMIT,
        None,
    );
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&first_stake_pool_owner, &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            stake_pool_reg_fragment,
            &jormungandr,
        )
        .expect("Error while sending registration certificate for first stake pool owner");
    let stake_pool_retire_fragment =
        fragment_builder.stake_pool_retire(vec![&first_stake_pool_owner], &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            stake_pool_retire_fragment.clone(),
            &jormungandr,
        )
        .unwrap();

    let trans = explorer
        .transaction_certificates(stake_pool_retire_fragment.hash().into())
        .expect("Non existing stake pool update transaction");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let stake_pool_retire_transaction = trans.data.unwrap().transaction;

    ExplorerVerifier::assert_transaction_certificates(
        stake_pool_retire_fragment,
        stake_pool_retire_transaction,
    )
    .unwrap();
}

#[test]
pub fn explorer_vote_plan_certificates_test() {
    let temp_dir = TempDir::new().unwrap();
    let mut first_stake_pool_owner = thor::Wallet::default();
    let bob = thor::Wallet::default();
    let discrimination = Discrimination::Test;

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::OffChain)
        .vote_start(propertyBlockDate::from_epoch_slot_id(1, 0))
        .tally_start(propertyBlockDate::from_epoch_slot_id(20, 0))
        .tally_end(propertyBlockDate::from_epoch_slot_id(30, 0))
        .public()
        .build();

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallets_having_some_values(vec![&first_stake_pool_owner, &bob])
                .with_discrimination(discrimination)
                .with_slots_per_epoch(SLOTS_PER_EPOCH.try_into().unwrap())
                .with_slot_duration(SLOT_DURATION.try_into().unwrap())
                .with_linear_fees(LinearFee::new(0, 0, 0)),
        )
        .build()
        .start_node(temp_dir)
        .unwrap();

    let fragment_sender = FragmentSender::from(&jormungandr.rest().settings().unwrap());

    let fragment_builder = FragmentBuilder::from_settings(
        &jormungandr.rest().settings().unwrap(),
        BlockDate::first().next_epoch(),
    );

    let params = ExplorerParams::new(
        TRANSACTION_CERTIFICATE_QUERY_COMPLEXITY_LIMIT,
        TRANSACTION_CERTIFICATE_QUERY_DEPTH_LIMIT,
        None,
    );
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let vote_plan_fragment = fragment_builder.vote_plan(&first_stake_pool_owner, &vote_plan);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            vote_plan_fragment.clone(),
            &jormungandr,
        )
        .unwrap();

    let trans = explorer
        .transaction_certificates(vote_plan_fragment.hash().into())
        .expect("vote plan transaction not found");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let vote_plan_transaction = trans.data.unwrap().transaction;

    ExplorerVerifier::assert_transaction_certificates(vote_plan_fragment, vote_plan_transaction)
        .unwrap();
}

#[test]
pub fn explorer_vote_cast_certificates_test() {
    let temp_dir = TempDir::new().unwrap();
    let mut alice = thor::Wallet::default();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .tally_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_end(BlockDate::from_epoch_slot_id(2, 0))
        .public()
        .build();

    let vote_plan_cert = Initial::Cert(
        thor::vote_plan_cert(
            &alice,
            BlockDate {
                epoch: 1,
                slot_id: 0,
            },
            &vote_plan,
        )
        .into(),
    );
    let wallets = [&alice];
    let config = Block0ConfigurationBuilder::default()
        .with_utxos(wallets.iter().map(|x| x.to_initial_fund(1000)).collect())
        .with_token(InitialToken {
            token_id: vote_plan.voting_token().clone().into(),
            policy: MintingPolicy::new().into(),
            to: vec![alice.to_initial_token(1000)],
        })
        .with_committees(&[alice.to_committee_id()])
        .with_slots_per_epoch(SLOTS_PER_EPOCH.try_into().unwrap())
        .with_certs(vec![vote_plan_cert])
        .with_treasury(1_000.into());

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(config)
        .build()
        .start_node(temp_dir)
        .unwrap();

    let fragment_sender = FragmentSender::from(&jormungandr.rest().settings().unwrap());

    let fragment_builder = FragmentBuilder::from_settings(
        &jormungandr.rest().settings().unwrap(),
        BlockDate::first().next_epoch(),
    );

    let params = ExplorerParams::new(
        TRANSACTION_CERTIFICATE_QUERY_COMPLEXITY_LIMIT,
        TRANSACTION_CERTIFICATE_QUERY_DEPTH_LIMIT,
        None,
    );
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let vote_cast_fragment = fragment_builder.vote_cast(&alice, &vote_plan, 2, &Choice::new(0));

    fragment_sender
        .send_fragment(&mut alice, vote_cast_fragment.clone(), &jormungandr)
        .unwrap();

    let trans = explorer
        .transaction_certificates(vote_cast_fragment.hash().into())
        .expect("vote cast transaction not found");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let vote_cast_transaction = trans.data.unwrap().transaction;
    ExplorerVerifier::assert_transaction_certificates(vote_cast_fragment, vote_cast_transaction)
        .unwrap();
}

#[test]
pub fn explorer_vote_tally_certificate_test() {
    let temp_dir = TempDir::new().unwrap();
    let mut alice = thor::Wallet::default();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .tally_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_end(BlockDate::from_epoch_slot_id(2, 0))
        .public()
        .build();

    let vote_plan_cert = Initial::Cert(
        thor::vote_plan_cert(
            &alice,
            chain_impl_mockchain::block::BlockDate {
                epoch: 1,
                slot_id: 0,
            },
            &vote_plan,
        )
        .into(),
    );
    let wallets = [&alice];
    let config = Block0ConfigurationBuilder::default()
        .with_utxos(wallets.iter().map(|x| x.to_initial_fund(1000)).collect())
        .with_token(InitialToken {
            token_id: vote_plan.voting_token().clone().into(),
            policy: MintingPolicy::new().into(),
            to: vec![alice.to_initial_token(1000)],
        })
        .with_committees(&[alice.to_committee_id()])
        .with_slots_per_epoch(SLOTS_PER_EPOCH.try_into().unwrap())
        .with_certs(vec![vote_plan_cert])
        .with_treasury(1_000.into());

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_block0_config(config)
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let fragment_sender = FragmentSender::from(&jormungandr.rest().settings().unwrap());

    let mut fragment_builder = FragmentBuilder::from_settings(
        &jormungandr.rest().settings().unwrap(),
        BlockDate::first().next_epoch(),
    );

    let params = ExplorerParams::new(
        TRANSACTION_CERTIFICATE_QUERY_COMPLEXITY_LIMIT,
        TRANSACTION_CERTIFICATE_QUERY_DEPTH_LIMIT,
        None,
    );
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let vote_cast_fragment = fragment_builder.vote_cast(&alice, &vote_plan, 2, &Choice::new(0));

    fragment_sender
        .send_fragment(&mut alice, vote_cast_fragment, &jormungandr)
        .unwrap();

    wait_for_epoch(1, jormungandr.rest());

    fragment_builder.update_valid_until(BlockDate::first().next_epoch().next_epoch());
    let vote_tally_fragment =
        fragment_builder.vote_tally(&alice, &vote_plan, VoteTallyPayload::Public);

    fragment_sender
        .send_fragment(&mut alice, vote_tally_fragment.clone(), &jormungandr)
        .unwrap();

    let trans = explorer
        .transaction_certificates(vote_tally_fragment.hash().into())
        .expect("vote tally transaction not found");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let vote_tally_transaction = trans.data.unwrap().transaction;
    ExplorerVerifier::assert_transaction_certificates(vote_tally_fragment, vote_tally_transaction)
        .unwrap();
}

#[test]
pub fn explorer_update_proposal_and_vote_certificate_test() {
    let temp_dir = TempDir::new().unwrap();
    let mut alice = thor::Wallet::default();
    let mut bob = thor::Wallet::default();
    let bft_secret_alice = create_new_key_pair::<Ed25519>();
    let bft_secret_bob = create_new_key_pair::<Ed25519>();
    let wallet_initial_funds = 5_000_000;

    let config = Block0ConfigurationBuilder::default()
        .with_utxos(vec![
            alice.to_initial_fund(wallet_initial_funds),
            bob.to_initial_fund(wallet_initial_funds),
        ])
        .with_consensus_leaders_ids(vec![
            bft_secret_alice.identifier().into(),
            bft_secret_bob.identifier().into(),
        ])
        .with_proposal_expiration(20.try_into().unwrap())
        .with_slots_per_epoch(SLOTS_PER_EPOCH.try_into().unwrap())
        .with_linear_fees(LinearFee::new(0, 0, 0));

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_block0_config(config)
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let new_block_context_max_size = 1000;
    let change_params = ConfigParams::new(vec![ConfigParam::BlockContentMaxSize(
        BlockContentMaxSize::from(new_block_context_max_size),
    )]);

    let update_proposal = UpdateProposal::new(
        change_params.into(),
        bft_secret_alice.identifier().into_public_key().into(),
    );

    let settings = jormungandr.rest().settings().unwrap();

    let fragment_sender = FragmentSender::from_settings(
        &settings,
        Fixed(BlockDate {
            epoch: 10,
            slot_id: 0,
        }),
        Default::default(),
    );

    let fragment_builder = FragmentBuilder::from_settings(
        &settings,
        BlockDate {
            epoch: 10,
            slot_id: 0,
        },
    );

    let params = ExplorerParams::new(
        TRANSACTION_CERTIFICATE_QUERY_COMPLEXITY_LIMIT,
        TRANSACTION_CERTIFICATE_QUERY_DEPTH_LIMIT,
        None,
    );
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let proposal_update_fragment = fragment_builder.update_proposal(
        &alice,
        update_proposal,
        &bft_secret_alice.signing_key().into_secret_key(),
    );

    fragment_sender
        .send_fragment(&mut alice, proposal_update_fragment.clone(), &jormungandr)
        .unwrap();

    wait_for_date(
        BlockDate {
            epoch: 0,
            slot_id: 10,
        }
        .into(),
        jormungandr.rest(),
    );

    let update_vote = UpdateVote::new(
        proposal_update_fragment.hash(),
        bft_secret_alice.identifier().into_public_key().into(),
    );

    fragment_sender
        .send_update_vote(
            &mut alice,
            &bft_secret_alice.signing_key().into_secret_key(),
            update_vote,
            &jormungandr,
        )
        .unwrap();

    let update_vote = UpdateVote::new(
        proposal_update_fragment.hash(),
        bft_secret_bob.identifier().into_public_key().into(),
    );

    let update_vote_fragment = fragment_builder.update_vote(
        &bob,
        update_vote,
        &bft_secret_bob.signing_key().into_secret_key(),
    );

    fragment_sender
        .send_fragment(&mut bob, update_vote_fragment.clone(), &jormungandr)
        .unwrap();

    wait_for_date(
        BlockDate {
            epoch: 1,
            slot_id: 0,
        }
        .into(),
        jormungandr.rest(),
    );

    let update_vote_resp = explorer
        .transaction_certificates(update_vote_fragment.hash().into())
        .expect("update vote transaction not found");

    assert!(
        update_vote_resp.errors.is_none(),
        "{:?}",
        update_vote_resp.errors.unwrap()
    );
    let update_vote_transaction = update_vote_resp.data.unwrap().transaction;
    ExplorerVerifier::assert_transaction_certificates(
        update_vote_fragment,
        update_vote_transaction,
    )
    .unwrap();

    let proposal_update_resp = explorer
        .transaction_certificates(proposal_update_fragment.hash().into())
        .expect("update proposal transaction not found");

    assert!(
        proposal_update_resp.errors.is_none(),
        "{:?}",
        proposal_update_resp.errors.unwrap()
    );
    let proposal_update_transaction = proposal_update_resp.data.unwrap().transaction;
    ExplorerVerifier::assert_transaction_certificates(
        proposal_update_fragment,
        proposal_update_transaction,
    )
    .unwrap();
}
