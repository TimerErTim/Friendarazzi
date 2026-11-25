#![cfg(feature = "job_store")]

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use friendarazzi_sqlite::SQLiteInstance;
use friendarazzi_sqlite::job_store::{JobState, SQLiteStateStore, StateStore};

fn test_time() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).expect("valid timestamp")
}

#[test_log::test(tokio::test)]
async fn sqlite_state_store_migrates_and_roundtrips_without_domain_tables() {
    let sqlite = Arc::new(SQLiteInstance::in_temp_dir().await.unwrap());
    let mut store = SQLiteStateStore::new(Arc::clone(&sqlite));

    store.migrate_to_latest().await.unwrap();
    store.migrate_to_latest().await.unwrap();

    assert!(store.load("start_ready_games").await.unwrap().is_none());

    let state = JobState {
        job_id: "start_ready_games".to_owned(),
        trigger_count: 3,
        last_run_at: Some(test_time()),
        last_success_at: Some(test_time()),
        next_run_at: Some(test_time() + Duration::seconds(30)),
        last_error: Some("transient".to_owned()),
    };
    store.save(&state).await.unwrap();

    let loaded = store.load("start_ready_games").await.unwrap().unwrap();
    assert_eq!(loaded.job_id, state.job_id);
    assert_eq!(loaded.trigger_count, state.trigger_count);
    assert_eq!(loaded.last_run_at, state.last_run_at);
    assert_eq!(loaded.last_success_at, state.last_success_at);
    assert_eq!(loaded.next_run_at, state.next_run_at);
    assert_eq!(loaded.last_error, state.last_error);

    store.delete("start_ready_games").await.unwrap();
    assert!(store.load("start_ready_games").await.unwrap().is_none());
}
