use super::{DatabaseError, Result};
use rusqlite::{Connection, TransactionBehavior};

const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_initial.sql"),
    include_str!("migrations/0002_import.sql"),
    include_str!("migrations/0003_manifest_identity.sql"),
    include_str!("migrations/0004_identification.sql"),
    include_str!("migrations/0005_archive.sql"),
];

pub(crate) fn apply(connection: &mut Connection) -> Result<()> {
    apply_steps(connection, MIGRATIONS)
}

fn apply_steps(connection: &mut Connection, migrations: &[&str]) -> Result<()> {
    // Read the version under the write lock, so concurrent startups cannot race.
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: i64 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let supported = migrations.len() as i64;
    if version < 0 {
        return Err(DatabaseError::InvalidSchemaVersion(version));
    }
    if version > supported {
        return Err(DatabaseError::NewerSchema {
            found: version,
            supported,
        });
    }
    for (index, sql) in migrations.iter().enumerate().skip(version as usize) {
        transaction.execute_batch(sql)?;
        transaction.pragma_update(None, "user_version", (index + 1) as i64)?;
    }
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_migration_rolls_back_schema_and_version() {
        let mut connection = Connection::open_in_memory().unwrap();
        assert!(
            apply_steps(
                &mut connection,
                &["CREATE TABLE example (id INTEGER);", "INVALID SQL;"]
            )
            .is_err()
        );
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 0);
        let tables: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = 'example'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 0);
    }
    #[test]
    fn negative_schema_version_is_rejected_without_changes() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE keep_data (value TEXT); INSERT INTO keep_data VALUES ('preserve'); PRAGMA user_version = -1;").unwrap();
        assert!(matches!(
            apply(&mut connection),
            Err(DatabaseError::InvalidSchemaVersion(-1))
        ));
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, -1);
        let value: String = connection
            .query_row("SELECT value FROM keep_data", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "preserve");
    }
}
