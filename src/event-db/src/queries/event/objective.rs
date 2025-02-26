use crate::{
    error::Error,
    types::event::{
        objective::{
            GroupBallotType, Objective, ObjectiveDetails, ObjectiveId, ObjectiveSummary,
            ObjectiveSupplementalData, ObjectiveType, RewardDefintion,
        },
        EventId,
    },
    EventDB,
};
use async_trait::async_trait;

#[async_trait]
pub trait ObjectiveQueries: Sync + Send + 'static {
    async fn get_objectives(
        &self,
        event: EventId,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Objective>, Error>;
}

impl EventDB {
    const OBJECTIVES_QUERY: &'static str =
        "SELECT objective.id, objective.title, objective.description, objective.rewards_currency, objective.rewards_total, objective.extra,
        objective_category.name, objective_category.description as objective_category_description,
        vote_options.objective as choices
        FROM objective
        INNER JOIN objective_category on objective.category = objective_category.name
        LEFT JOIN vote_options on objective.vote_options = vote_options.id
        WHERE objective.event = $1
        LIMIT $2 OFFSET $3;";
}

#[async_trait]
impl ObjectiveQueries for EventDB {
    async fn get_objectives(
        &self,
        event: EventId,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Objective>, Error> {
        let conn = self.pool.get().await?;

        let rows = conn
            .query(
                Self::OBJECTIVES_QUERY,
                &[&event.0, &limit, &offset.unwrap_or(0)],
            )
            .await?;

        let mut objectives = Vec::new();
        for row in rows {
            let summary = ObjectiveSummary {
                id: ObjectiveId(row.try_get("id")?),
                objective_type: ObjectiveType {
                    id: row.try_get("name")?,
                    description: row.try_get("objective_category_description")?,
                },
                title: row.try_get("title")?,
                description: row.try_get("description")?,
            };
            let currency: Option<_> = row.try_get("rewards_currency")?;
            let value: Option<_> = row.try_get("rewards_total")?;
            let reward = match (currency, value) {
                (Some(currency), Some(value)) => Some(RewardDefintion { currency, value }),
                _ => None,
            };
            let extra = row.try_get::<_, Option<serde_json::Value>>("extra")?;
            let url = extra
                .as_ref()
                .and_then(|extra| {
                    extra
                        .get("url")
                        .map(|url| url.as_str().map(|str| str.to_string()))
                })
                .flatten();
            let sponsor = extra
                .as_ref()
                .and_then(|extra| {
                    extra
                        .get("sponsor")
                        .map(|sponsor| sponsor.as_str().map(|str| str.to_string()))
                })
                .flatten();
            let video = extra
                .and_then(|val| {
                    val.get("video")
                        .map(|video| video.as_str().map(|str| str.to_string()))
                })
                .flatten();
            let supplemental = match (sponsor, video) {
                (Some(sponsor), Some(video)) => Some(ObjectiveSupplementalData { sponsor, video }),
                _ => None,
            };
            let details = ObjectiveDetails {
                reward,
                url,
                supplemental,
                choices: row
                    .try_get::<_, Option<Vec<_>>>("choices")?
                    .unwrap_or_default(),
                // TODO fix this, need to fill with the real data
                ballot: vec![
                    GroupBallotType {
                        group: "rep".to_string(),
                        ballot: "private".to_string(),
                    },
                    GroupBallotType {
                        group: "direct".to_string(),
                        ballot: "private".to_string(),
                    },
                ],
            };
            objectives.push(Objective { summary, details });
        }

        Ok(objectives)
    }
}

/// Need to setup and run a test event db instance
/// To do it you can use `cargo make local-event-db-test`
/// Also need establish `EVENT_DB_URL` env variable with the following value
/// ```
/// EVENT_DB_URL="postgres://catalyst-event-dev:CHANGE_ME@localhost/CatalystEventDev"
/// ```
/// https://github.com/input-output-hk/catalyst-core/tree/main/src/event-db/Readme.md
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{establish_connection, types::event::objective::GroupBallotType};

    #[tokio::test]
    async fn get_objectives_test() {
        let event_db = establish_connection(None).await.unwrap();

        let objectives = event_db
            .get_objectives(EventId(1), None, None)
            .await
            .unwrap();
        assert_eq!(
            objectives,
            vec![
                Objective {
                    summary: ObjectiveSummary {
                        id: ObjectiveId(1),
                        objective_type: ObjectiveType {
                            id: "catalyst-simple".to_string(),
                            description: "A Simple choice".to_string()
                        },
                        title: "title 1".to_string(),
                        description: "description 1".to_string(),
                    },
                    details: ObjectiveDetails {
                        reward: Some(RewardDefintion {
                            currency: "ADA".to_string(),
                            value: 100
                        }),
                        choices: vec!["yes".to_string(), "no".to_string()],
                        ballot: vec![
                            GroupBallotType {
                                group: "rep".to_string(),
                                ballot: "private".to_string(),
                            },
                            GroupBallotType {
                                group: "direct".to_string(),
                                ballot: "private".to_string(),
                            },
                        ],
                        url: Some("objective 1 url".to_string()),
                        supplemental: Some(ObjectiveSupplementalData {
                            sponsor: "objective 1 sponsor".to_string(),
                            video: "objective 1 video".to_string()
                        }),
                    }
                },
                Objective {
                    summary: ObjectiveSummary {
                        id: ObjectiveId(2),
                        objective_type: ObjectiveType {
                            id: "catalyst-native".to_string(),
                            description: "??".to_string()
                        },
                        title: "title 2".to_string(),
                        description: "description 2".to_string(),
                    },
                    details: ObjectiveDetails {
                        reward: None,
                        choices: vec![],
                        ballot: vec![
                            GroupBallotType {
                                group: "rep".to_string(),
                                ballot: "private".to_string(),
                            },
                            GroupBallotType {
                                group: "direct".to_string(),
                                ballot: "private".to_string(),
                            },
                        ],
                        url: None,
                        supplemental: None,
                    }
                }
            ]
        );

        let objectives = event_db
            .get_objectives(EventId(1), Some(1), None)
            .await
            .unwrap();
        assert_eq!(
            objectives,
            vec![Objective {
                summary: ObjectiveSummary {
                    id: ObjectiveId(1),
                    objective_type: ObjectiveType {
                        id: "catalyst-simple".to_string(),
                        description: "A Simple choice".to_string()
                    },
                    title: "title 1".to_string(),
                    description: "description 1".to_string(),
                },
                details: ObjectiveDetails {
                    reward: Some(RewardDefintion {
                        currency: "ADA".to_string(),
                        value: 100
                    }),
                    choices: vec!["yes".to_string(), "no".to_string()],
                    ballot: vec![
                        GroupBallotType {
                            group: "rep".to_string(),
                            ballot: "private".to_string(),
                        },
                        GroupBallotType {
                            group: "direct".to_string(),
                            ballot: "private".to_string(),
                        },
                    ],
                    url: Some("objective 1 url".to_string()),
                    supplemental: Some(ObjectiveSupplementalData {
                        sponsor: "objective 1 sponsor".to_string(),
                        video: "objective 1 video".to_string()
                    }),
                }
            },]
        );

        let objectives = event_db
            .get_objectives(EventId(1), None, Some(1))
            .await
            .unwrap();
        assert_eq!(
            objectives,
            vec![Objective {
                summary: ObjectiveSummary {
                    id: ObjectiveId(2),
                    objective_type: ObjectiveType {
                        id: "catalyst-native".to_string(),
                        description: "??".to_string()
                    },
                    title: "title 2".to_string(),
                    description: "description 2".to_string(),
                },
                details: ObjectiveDetails {
                    reward: None,
                    choices: vec![],
                    ballot: vec![
                        GroupBallotType {
                            group: "rep".to_string(),
                            ballot: "private".to_string(),
                        },
                        GroupBallotType {
                            group: "direct".to_string(),
                            ballot: "private".to_string(),
                        },
                    ],
                    url: None,
                    supplemental: None,
                }
            }]
        );

        let objectives = event_db
            .get_objectives(EventId(1), Some(1), Some(2))
            .await
            .unwrap();
        assert_eq!(objectives, vec![]);
    }
}
