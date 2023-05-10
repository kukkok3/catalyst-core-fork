INSERT INTO event
(row_id, name, description, registration_snapshot_time, snapshot_start,
 voting_power_threshold, max_voting_power_pct, start_time, end_time, insight_sharing_start,
 proposal_submission_start, refine_proposals_start, finalize_proposals_start, proposal_assessment_start, assessment_qa_start,
 voting_start, voting_end, tallying_end, block0, block0_hash, committee_size, committee_threshold)
VALUES

(1,
 'Test Fund 1', 'Test Fund 1 description',
 '2020-03-31 12:00:00', '2020-03-31 12:00:00', 1, 100,
 '2020-05-01 12:00:00', '2020-06-01 12:00:00',
 '2020-03-01 12:00:00', '2020-03-01 12:00:00', '2020-03-01 12:00:00', '2020-03-01 12:00:00', '2020-03-01 12:00:00', '2020-03-01 12:00:00', '2020-05-01 12:00:00', '2020-06-01 12:00:00', '2020-07-01 12:00:00',
 'x0000000000000000000000000000000000000000000000000000000000000000', 'x0000000000000000000000000000000000000000000000000000000000000000',
 4, 4),

(2,
 'Test Fund 2', 'Test Fund 2 description',
 '2021-03-31 12:00:00', '2021-03-31 12:00:00', 1, 100,
 '2021-05-01 12:00:00', '2021-06-01 12:00:00',
 '2021-03-01 12:00:00', '2021-03-01 12:00:00', '2021-03-01 12:00:00', '2021-03-01 12:00:00', '2021-03-01 12:00:00', '2021-03-01 12:00:00', '2021-05-01 12:00:00', '2021-06-01 12:00:00', '2021-07-01 12:00:00',
 'x0000000000000000000000000000000000000000000000000000000000000000', 'x0000000000000000000000000000000000000000000000000000000000000000',
 5, 5),

(3,
 'Test Fund 3', 'Test Fund 3 description',
 '2022-03-31 12:00:00', '2022-03-31 12:00:00', 1, 100,
 '2022-05-01 12:00:00', '2022-06-01 12:00:00',
 '2022-03-01 12:00:00', '2022-03-01 12:00:00', '2022-03-01 12:00:00', '2022-03-01 12:00:00', '2022-03-01 12:00:00', '2022-03-01 12:00:00', '2022-05-01 12:00:00', '2022-06-01 12:00:00', '2022-07-01 12:00:00',
 'x0000000000000000000000000000000000000000000000000000000000000000', 'x0000000000000000000000000000000000000000000000000000000000000000',
 6, 6),

 (4,
 'Test Fund 4', 'Test Fund 4 description',
 NULL, NULL, NULL, NULL,
 NULL, NULL,
 NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL,
 NULL, NULL,
 6, 6);
 