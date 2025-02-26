use crate::{
    service::{handle_result, Error},
    state::State,
};
use axum::{
    extract::{Path, Query},
    routing::get,
    Router,
};
use event_db::types::{
    event::EventId,
    registration::{Delegator, Voter},
};
use serde::Deserialize;
use std::sync::Arc;

pub fn registration(state: Arc<State>) -> Router {
    Router::new()
        .route(
            "/registration/voter/:voting_key",
            get({
                let state = state.clone();
                move |path, query| async {
                    handle_result(voter_exec(path, query, state).await).await
                }
            }),
        )
        .route(
            "/registration/delegations/:stake_public_key",
            get({
                move |path, query| async {
                    handle_result(delegations_exec(path, query, state).await).await
                }
            }),
        )
}

#[derive(Deserialize)]
struct EventIdQuery {
    eid: Option<EventId>,
}

async fn voter_exec(
    Path(voting_key): Path<String>,
    eid_query: Query<EventIdQuery>,
    state: Arc<State>,
) -> Result<Voter, Error> {
    tracing::debug!(
        "voter_query: voting_key: {0}, eid: {1:?}",
        voting_key,
        eid_query.eid
    );

    let voter = state.event_db.get_voter(&eid_query.eid, voting_key).await?;
    Ok(voter)
}

async fn delegations_exec(
    Path(stake_public_key): Path<String>,
    eid_query: Query<EventIdQuery>,
    state: Arc<State>,
) -> Result<Delegator, Error> {
    tracing::debug!(
        "delegator_query: stake_public_key: {0}, eid: {1:?}",
        stake_public_key,
        eid_query.eid
    );

    let delegator = state
        .event_db
        .get_delegator(&eid_query.eid, stake_public_key)
        .await?;
    Ok(delegator)
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
    use crate::service::app;
    use axum::{
        body::{Body, HttpBody},
        http::{Request, StatusCode},
    };
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
    use event_db::types::registration::{Delegation, VoterInfo};
    use tower::ServiceExt;

    #[tokio::test]
    async fn voter_test() {
        let state = Arc::new(State::new(None).await.unwrap());
        let app = app(state);

        let request = Request::builder()
            .uri(format!("/api/v1/registration/voter/{0}", "voting_key_1"))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            String::from_utf8(response.into_body().data().await.unwrap().unwrap().to_vec())
                .unwrap(),
            serde_json::to_string(&Voter {
                voter_info: VoterInfo {
                    voting_power: 250,
                    voting_group: "rep".to_string(),
                    delegations_power: 250,
                    delegations_count: 2,
                    voting_power_saturation: 0.625,
                },
                as_at: DateTime::<Utc>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2022, 3, 31).unwrap(),
                        NaiveTime::from_hms_opt(12, 0, 0).unwrap()
                    ),
                    Utc
                ),
                last_updated: DateTime::<Utc>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2022, 3, 31).unwrap(),
                        NaiveTime::from_hms_opt(12, 0, 0).unwrap()
                    ),
                    Utc
                ),
                is_final: true,
            })
            .unwrap()
        );

        let request = Request::builder()
            .uri(format!(
                "/api/v1/registration/voter/{0}?eid={1}",
                "voting_key_1", 1
            ))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            String::from_utf8(response.into_body().data().await.unwrap().unwrap().to_vec())
                .unwrap(),
            serde_json::to_string(&Voter {
                voter_info: VoterInfo {
                    voting_power: 250,
                    voting_group: "rep".to_string(),
                    delegations_power: 250,
                    delegations_count: 2,
                    voting_power_saturation: 0.625,
                },
                as_at: DateTime::<Utc>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2020, 3, 31).unwrap(),
                        NaiveTime::from_hms_opt(12, 0, 0).unwrap()
                    ),
                    Utc
                ),
                last_updated: DateTime::<Utc>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2020, 3, 31).unwrap(),
                        NaiveTime::from_hms_opt(12, 0, 0).unwrap()
                    ),
                    Utc
                ),
                is_final: true,
            })
            .unwrap()
        );

        let request = Request::builder()
            .uri(format!("/api/v1/registration/voter/{0}", "voting_key"))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let request = Request::builder()
            .uri(format!(
                "/api/v1/registration/voter/{0}?eid={1}",
                "voting_key", 1
            ))
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delegations_test() {
        let state = Arc::new(State::new(None).await.unwrap());
        let app = app(state);

        let request = Request::builder()
            .uri(format!(
                "/api/v1/registration/delegations/{0}",
                "stake_public_key_1"
            ))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            String::from_utf8(response.into_body().data().await.unwrap().unwrap().to_vec())
                .unwrap(),
            serde_json::to_string(&Delegator {
                delegations: vec![
                    Delegation {
                        voting_key: "voting_key_1".to_string(),
                        group: "rep".to_string(),
                        weight: 1,
                        value: 140
                    },
                    Delegation {
                        voting_key: "voting_key_2".to_string(),
                        group: "rep".to_string(),
                        weight: 1,
                        value: 100
                    }
                ],
                raw_power: 240,
                total_power: 1000,
                as_at: DateTime::<Utc>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2022, 3, 31).unwrap(),
                        NaiveTime::from_hms_opt(12, 0, 0).unwrap()
                    ),
                    Utc
                ),
                last_updated: DateTime::<Utc>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2022, 3, 31).unwrap(),
                        NaiveTime::from_hms_opt(12, 0, 0).unwrap()
                    ),
                    Utc
                ),
                is_final: true
            })
            .unwrap()
        );

        let request = Request::builder()
            .uri(format!(
                "/api/v1/registration/delegations/{0}?eid={1}",
                "stake_public_key_1", 1
            ))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            String::from_utf8(response.into_body().data().await.unwrap().unwrap().to_vec())
                .unwrap(),
            serde_json::to_string(&Delegator {
                delegations: vec![
                    Delegation {
                        voting_key: "voting_key_1".to_string(),
                        group: "rep".to_string(),
                        weight: 1,
                        value: 140
                    },
                    Delegation {
                        voting_key: "voting_key_2".to_string(),
                        group: "rep".to_string(),
                        weight: 1,
                        value: 100
                    }
                ],
                raw_power: 240,
                total_power: 1000,
                as_at: DateTime::<Utc>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2020, 3, 31).unwrap(),
                        NaiveTime::from_hms_opt(12, 0, 0).unwrap()
                    ),
                    Utc
                ),
                last_updated: DateTime::<Utc>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2020, 3, 31).unwrap(),
                        NaiveTime::from_hms_opt(12, 0, 0).unwrap()
                    ),
                    Utc
                ),
                is_final: true
            })
            .unwrap()
        );

        let request = Request::builder()
            .uri(format!(
                "/api/v1/registration/delegations/{0}",
                "stake_public_key"
            ))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let request = Request::builder()
            .uri(format!(
                "/api/v1/registration/delegations/{0}?eid={1}",
                "stake_public_key", 1
            ))
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
