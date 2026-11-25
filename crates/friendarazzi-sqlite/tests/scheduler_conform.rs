#![cfg(feature = "job_store")]

#[path = "../../../backend/scheduler/tests/cases/mod.rs"]
mod cases;

use std::sync::Arc;

use friendarazzi_sqlite::SQLiteInstance;
use friendarazzi_sqlite::job_store::SQLiteStateStore;

#[test_log::test(tokio::test)]
async fn scheduler_framework_with_sqlite_state_store() {
    let sqlite = Arc::new(SQLiteInstance::in_temp_dir().await.unwrap());
    sqlite.migrate_repos_to_latest().await.unwrap();
    let mut store = SQLiteStateStore::new(Arc::clone(&sqlite));
    store.migrate_to_latest().await.unwrap();

    cases::scheduler_framework_runs_and_stops_cleanly(Box::new(sqlite), store).await;
}
