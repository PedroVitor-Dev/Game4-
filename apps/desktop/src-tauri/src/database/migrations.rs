use super::{DatabaseError, Result};
use rusqlite::{Connection, TransactionBehavior};

const MIGRATIONS: &[&str] = &[include_str!("migrations/0001_initial.sql")];

pub(super) fn apply(connection: &mut Connection) -> Result<()> {
    apply_steps(connection, MIGRATIONS)
}

fn apply_steps(connection: &mut Connection, migrations: &[&str]) -> Result<()> {
    // Read the version under the write lock, so concurrent startups cannot race.
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: i64 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let supported = migrations.len() as i64;
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
}
