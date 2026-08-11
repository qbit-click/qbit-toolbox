#![deny(unsafe_code)]

//! Durable SQLite storage for application-wide state.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Transaction, params};

const DATABASE_DIRECTORY: &str = "data";
const DATABASE_FILE: &str = "core.db";
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// The durability guarantees requested for a SQLite store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurabilityProfile {
    Critical,
    Standard,
    Rebuildable,
}

/// Errors raised while opening or using the core store.
#[derive(Debug)]
pub enum PersistenceError {
    Io {
        source: io::Error,
    },
    Sqlite {
        source: rusqlite::Error,
    },
    InvalidDurabilityProfile {
        profile: DurabilityProfile,
    },
    Migration {
        version: i64,
        name: &'static str,
        source: rusqlite::Error,
    },
    FutureSchema {
        current_version: i64,
        latest_supported_version: i64,
    },
    InconsistentSchema {
        detail: String,
    },
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { source } => write!(formatter, "persistence I/O error: {source}"),
            Self::Sqlite { source } => write!(formatter, "SQLite error: {source}"),
            Self::InvalidDurabilityProfile { profile } => {
                write!(
                    formatter,
                    "durability profile {profile:?} is not valid for core.db"
                )
            }
            Self::Migration {
                version,
                name,
                source,
            } => {
                write!(formatter, "migration {version} ({name}) failed: {source}")
            }
            Self::FutureSchema {
                current_version,
                latest_supported_version,
            } => write!(
                formatter,
                "database schema version {current_version} is newer than supported version {latest_supported_version}"
            ),
            Self::InconsistentSchema { detail } => {
                write!(formatter, "inconsistent database schema: {detail}")
            }
        }
    }
}

impl Error for PersistenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source } => Some(source),
            Self::Sqlite { source } | Self::Migration { source, .. } => Some(source),
            Self::InvalidDurabilityProfile { .. }
            | Self::FutureSchema { .. }
            | Self::InconsistentSchema { .. } => None,
        }
    }
}

/// A durable, application-wide SQLite store.
#[derive(Debug)]
pub struct CoreStore {
    connection: Connection,
    database_path: PathBuf,
}

impl CoreStore {
    /// Opens the durable core database below the caller-provided application data root.
    pub fn open(
        app_data_root: impl AsRef<Path>,
        profile: DurabilityProfile,
    ) -> Result<Self, PersistenceError> {
        if profile == DurabilityProfile::Rebuildable {
            return Err(PersistenceError::InvalidDurabilityProfile { profile });
        }

        let database_directory = app_data_root.as_ref().join(DATABASE_DIRECTORY);
        fs::create_dir_all(&database_directory)
            .map_err(|source| PersistenceError::Io { source })?;
        let database_path = database_directory.join(DATABASE_FILE);
        let mut connection = Connection::open(&database_path)
            .map_err(|source| PersistenceError::Sqlite { source })?;

        configure_connection(&connection, profile)?;
        run_migrations(&mut connection, MIGRATIONS)?;

        Ok(Self {
            connection,
            database_path,
        })
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn set_feature_enabled(
        &self,
        feature_id: &str,
        enabled: bool,
    ) -> Result<(), PersistenceError> {
        self.connection
            .execute(
                "INSERT INTO feature_state (feature_id, enabled) VALUES (?1, ?2) \
                 ON CONFLICT(feature_id) DO UPDATE SET enabled = excluded.enabled",
                params![feature_id, i64::from(enabled)],
            )
            .map(|_| ())
            .map_err(|source| PersistenceError::Sqlite { source })
    }

    pub fn feature_enabled(&self, feature_id: &str) -> Result<Option<bool>, PersistenceError> {
        self.connection
            .query_row(
                "SELECT enabled FROM feature_state WHERE feature_id = ?1",
                params![feature_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|value| value.map(|enabled| enabled != 0))
            .map_err(|source| PersistenceError::Sqlite { source })
    }

    pub fn schema_version(&self) -> Result<i64, PersistenceError> {
        schema_version(&self.connection)
    }
}

fn configure_connection(
    connection: &Connection,
    profile: DurabilityProfile,
) -> Result<(), PersistenceError> {
    let synchronous = match profile {
        DurabilityProfile::Critical => "FULL",
        DurabilityProfile::Standard => "NORMAL",
        DurabilityProfile::Rebuildable => {
            return Err(PersistenceError::InvalidDurabilityProfile { profile });
        }
    };

    connection
        .execute_batch(&format!(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = {synchronous}; PRAGMA foreign_keys = ON;"
        ))
        .map_err(|source| PersistenceError::Sqlite { source })?;
    connection
        .busy_timeout(BUSY_TIMEOUT)
        .map_err(|source| PersistenceError::Sqlite { source })
}

struct Migration {
    version: i64,
    name: &'static str,
    apply: fn(&Transaction<'_>) -> rusqlite::Result<()>,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "create_feature_state",
    apply: create_feature_state,
}];

fn create_feature_state(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute_batch(
        "CREATE TABLE feature_state (
            feature_id TEXT PRIMARY KEY,
            enabled INTEGER NOT NULL CHECK(enabled IN (0, 1))
        );",
    )
}

fn run_migrations(
    connection: &mut Connection,
    migrations: &[Migration],
) -> Result<(), PersistenceError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .map_err(|source| PersistenceError::Sqlite { source })?;

    let current_version = schema_version(connection)?;
    let latest_supported_version = migrations.last().map_or(0, |migration| migration.version);
    if current_version > latest_supported_version {
        return Err(PersistenceError::FutureSchema {
            current_version,
            latest_supported_version,
        });
    }

    for migration in migrations {
        let applied = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = ?1",
                params![migration.version],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|source| PersistenceError::Sqlite { source })?;

        if let Some(applied_name) = applied {
            if applied_name != migration.name {
                return Err(PersistenceError::InconsistentSchema {
                    detail: format!(
                        "migration {} is recorded as `{applied_name}`",
                        migration.version
                    ),
                });
            }
            continue;
        }
        if migration.version <= current_version {
            return Err(PersistenceError::InconsistentSchema {
                detail: format!(
                    "migration {} is missing below recorded version {current_version}",
                    migration.version
                ),
            });
        }

        let transaction =
            connection
                .transaction()
                .map_err(|source| PersistenceError::Migration {
                    version: migration.version,
                    name: migration.name,
                    source,
                })?;
        if let Err(source) = (migration.apply)(&transaction) {
            return Err(PersistenceError::Migration {
                version: migration.version,
                name: migration.name,
                source,
            });
        }
        if let Err(source) = transaction.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            params![migration.version, migration.name],
        ) {
            return Err(PersistenceError::Migration {
                version: migration.version,
                name: migration.name,
                source,
            });
        }
        transaction
            .commit()
            .map_err(|source| PersistenceError::Migration {
                version: migration.version,
                name: migration.name,
                source,
            })?;
    }

    Ok(())
}

fn schema_version(connection: &Connection) -> Result<i64, PersistenceError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|source| PersistenceError::Sqlite { source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn fresh_open_creates_expected_schema_and_version() {
        let directory = tempdir().unwrap();
        let store = CoreStore::open(directory.path(), DurabilityProfile::Critical).unwrap();

        assert_eq!(store.schema_version().unwrap(), 1);
        assert!(store.database_path().ends_with("data/core.db"));
        let table: String = store
            .connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'feature_state'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table, "feature_state");
    }

    #[test]
    fn reopening_is_idempotent() {
        let directory = tempdir().unwrap();
        CoreStore::open(directory.path(), DurabilityProfile::Standard).unwrap();
        let reopened = CoreStore::open(directory.path(), DurabilityProfile::Standard).unwrap();

        assert_eq!(reopened.schema_version().unwrap(), 1);
        let count: i64 = reopened
            .connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn feature_enabled_state_roundtrips_and_updates() {
        let directory = tempdir().unwrap();
        let store = CoreStore::open(directory.path(), DurabilityProfile::Standard).unwrap();

        assert_eq!(store.feature_enabled("qbit.example").unwrap(), None);
        store.set_feature_enabled("qbit.example", true).unwrap();
        assert_eq!(store.feature_enabled("qbit.example").unwrap(), Some(true));
        store.set_feature_enabled("qbit.example", false).unwrap();
        assert_eq!(store.feature_enabled("qbit.example").unwrap(), Some(false));
    }

    #[test]
    fn rebuildable_is_rejected_before_creating_a_database() {
        let directory = tempdir().unwrap();
        assert!(matches!(
            CoreStore::open(directory.path(), DurabilityProfile::Rebuildable),
            Err(PersistenceError::InvalidDurabilityProfile { .. })
        ));
        assert!(
            !directory
                .path()
                .join(DATABASE_DIRECTORY)
                .join(DATABASE_FILE)
                .exists()
        );
    }

    #[test]
    fn failed_migration_rolls_back_schema_and_metadata() {
        fn failing_migration(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
            transaction.execute_batch("CREATE TABLE should_rollback (value TEXT); THIS IS NOT SQL;")
        }

        let directory = tempdir().unwrap();
        let database_path = directory.path().join("migration-test.db");
        let mut connection = Connection::open(&database_path).unwrap();
        let migrations = [Migration {
            version: 1,
            name: "fails",
            apply: failing_migration,
        }];

        assert!(matches!(
            run_migrations(&mut connection, &migrations),
            Err(PersistenceError::Migration { version: 1, .. })
        ));
        let table = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'should_rollback'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .unwrap();
        assert_eq!(table, None);
        assert_eq!(schema_version(&connection).unwrap(), 0);
    }
}
