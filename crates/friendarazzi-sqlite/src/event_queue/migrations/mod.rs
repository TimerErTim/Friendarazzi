use std::sync::LazyLock;

use molting::{MigrationRegistry, UpMigration};

mod v0001_event_queue;

type Registry = MigrationRegistry<dyn UpMigration<libsql::Transaction, libsql::Error>>;

pub static MIGRATIONS: LazyLock<Registry> = LazyLock::new(|| {
    let mut registry: Registry = MigrationRegistry::new();
    registry.add(v0001_event_queue::Migration);
    registry
});
