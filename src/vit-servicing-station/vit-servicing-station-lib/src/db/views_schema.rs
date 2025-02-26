use diesel::table;

table! {
    full_proposals_info {
        id -> Integer,
        proposal_id -> Text,
        proposal_category -> Text,
        proposal_title -> Text,
        proposal_summary -> Text,
        proposal_public_key -> Text,
        proposal_funds -> BigInt,
        proposal_url -> Text,
        proposal_files_url -> Text,
        proposal_impact_score -> BigInt,
        proposer_name -> Text,
        proposer_contact -> Text,
        proposer_url -> Text,
        proposer_relevant_experience -> Text,
        chain_proposal_id -> Binary,
        chain_vote_options -> Array<Text>,
        challenge_id -> Integer,
        extra -> Nullable<Text>,

        reviews_count -> Integer,

        chain_vote_start_time -> BigInt,
        chain_vote_end_time -> BigInt,
        chain_committee_end_time -> BigInt,
        chain_voteplan_payload -> Text,
        chain_vote_encryption_key -> Text,
        fund_id -> Integer,

        challenge_type -> Text,
        proposal_solution -> Nullable<Text>,
        proposal_brief -> Nullable<Text>,
        proposal_importance -> Nullable<Text>,
        proposal_goal -> Nullable<Text>,
        proposal_metrics -> Nullable<Text>,

        chain_proposal_index -> BigInt,
        chain_voteplan_id -> Text,

        group_id -> Text,
    }
}
