use std::path::PathBuf;
use std::sync::Arc;

use chrono_tz::Tz;
use clap::Parser;
use error_stack::{Report, ResultExt};
use friendarazzi_core::App;
use friendarazzi_core::application::IdentityProviderConfiguration;
use friendarazzi_filesystemstore::FileSystemObjectStore;
use friendarazzi_scheduler::{FriendarazziScheduler, FriendarazziSchedulerConfig};
use friendarazzi_sqlite::SQLiteInstance;
use friendarazzi_sqlite::job_store::SQLiteStateStore;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

#[derive(Debug, Parser)]
#[command(
    name = "friendarazzi-scheduler",
    about = "Friendarazzi background scheduler"
)]
struct Cli {
    #[arg(
        long,
        env = "FRIENDARAZZI_DATABASE_URL",
        default_value = "friendarazzi.db"
    )]
    database_url: String,

    #[arg(
        long,
        env = "FRIENDARAZZI_OBJECT_STORE_DIR",
        default_value = "storage/"
    )]
    object_store_dir: PathBuf,

    #[arg(
        long,
        env = "FRIENDARAZZI_SCHEDULER_TIMEZONE",
        default_value = "Europe/Vienna"
    )]
    timezone: Tz,

    #[arg(
        long,
        env = "FRIENDARAZZI_TOKEN_ISSUER",
        default_value = "friendarazzi.app"
    )]
    token_issuer: String,

    #[arg(
        long,
        env = "FRIENDARAZZI_TOKEN_AUDIENCE",
        default_value = "friendarazzi-api"
    )]
    token_audience: String,
}

#[derive(Debug, thiserror::Error)]
#[error("friendarazzi-scheduler failed")]
struct SchedulerMainError;

#[tokio::main]
async fn main() -> Result<(), Report<SchedulerMainError>> {
    let cli = Cli::parse();
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .with_env_var("FRIENDARAZZI_LOG_LEVEL")
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let sqlite = Arc::new(
        SQLiteInstance::local(&cli.database_url)
            .await
            .change_context(SchedulerMainError)
            .attach_opaque("sqlite instance creation")?,
    );
    sqlite
        .migrate_repos_to_latest()
        .await
        .change_context(SchedulerMainError)
        .attach_opaque("sqlite repository migration")?;

    let mut store = SQLiteStateStore::new(Arc::clone(&sqlite));
    store
        .migrate_to_latest()
        .await
        .change_context(SchedulerMainError)
        .attach_opaque("sqlite job state store migration")?;

    let object_store = FileSystemObjectStore::new(&cli.object_store_dir)
        .change_context(SchedulerMainError)
        .attach_opaque("file system object store initialization")?;

    let app = App::builder()
        .uow_provider(Box::new(sqlite))
        .object_store(Arc::new(object_store))
        .konto(IdentityProviderConfiguration::neu(
            cli.token_issuer,
            cli.token_audience,
        ))
        .build();

    let (handle, fut) = FriendarazziScheduler::new(app)
        .state_store(store)
        .start_with_handle(FriendarazziSchedulerConfig {
            timezone: cli.timezone,
        });
    let mut join = tokio::spawn(fut);

    info!("friendarazzi-scheduler running");
    tokio::signal::ctrl_c()
        .await
        .change_context(SchedulerMainError)
        .attach_opaque("ctrl+c signal handling")?;
    handle.shutdown();

    tokio::select! {
        res = &mut join => {
            if let Ok(Err(err)) = res {
                tracing::error!(?err, "friendarazzi-scheduler exited with error");
            }
        }
        () = tokio::time::sleep(std::time::Duration::from_millis(250)) => {
            join.abort();
            let _ = join.await;
            info!("friendarazzi-scheduler aborted after shutdown grace timeout");
        }
    }
    info!("friendarazzi-scheduler stopped");
    Ok(())
}
