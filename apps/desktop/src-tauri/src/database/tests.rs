use super::*;
use rusqlite::params;

fn temporary_database() -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("nested/database/game4.sqlite3");
    (directory, path)
}

#[test]
fn creates_database_and_preserves_records_after_reopening() {
    let (_directory, path) = temporary_database();
    {
        let database = Database::open(&path).unwrap();
        assert_eq!(database.info().unwrap().schema_version, 3);
        database
            .connection
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO collection_items (title) VALUES (?1)",
                ["Preservation fixture"],
            )
            .unwrap();
    }
    let database = Database::open(&path).unwrap();
    let info = database.info().unwrap();
    assert_eq!(info.item_count, 1);
    assert_eq!(info.file_count, 0);
    assert_eq!(info.task_count, 0);
    let connection = database.connection.lock().unwrap();
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
}

#[test]
fn enforces_content_identity_and_independent_copy_health() {
    let (_directory, path) = temporary_database();
    let database = Database::open(&path).unwrap();
    let connection = database.connection.lock().unwrap();
    let hash = "a".repeat(64);
    connection
        .execute(
            "INSERT INTO files (sha256, size_bytes) VALUES (?1, 100)",
            [&hash],
        )
        .unwrap();
    assert!(
        connection
            .execute(
                "INSERT INTO files (sha256, size_bytes) VALUES (?1, 100)",
                [&hash]
            )
            .is_err()
    );
    assert!(
        connection
            .execute(
                "INSERT INTO files (sha256, size_bytes) VALUES ('invalid', 100)",
                []
            )
            .is_err()
    );
    connection.execute("INSERT INTO file_locations (file_id, path, path_key, verification_status) VALUES (1, ?1, ?2, 'verified')", params!["C:/a.bin", "c:/a.bin"]).unwrap();
    connection.execute("INSERT INTO file_locations (file_id, path, path_key, verification_status) VALUES (1, ?1, ?2, 'missing')", params!["D:/a.bin", "d:/a.bin"]).unwrap();
    assert!(
        connection
            .execute(
                "INSERT INTO file_locations (file_id, path, path_key) VALUES (99, 'x', 'x')",
                []
            )
            .is_err()
    );
    assert!(
        connection
            .execute("DELETE FROM files WHERE id = 1", [])
            .is_err()
    );
    assert!(connection.execute("INSERT INTO file_locations (file_id, path, path_key) VALUES (1, 'C:/A.bin', 'c:/a.bin')", []).is_err());
    let copies: i64 = connection
        .query_row(
            "SELECT count(*) FROM file_locations WHERE file_id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(copies, 2);
}

#[test]
fn validates_tasks_and_collection_relationships() {
    let (_directory, path) = temporary_database();
    let database = Database::open(&path).unwrap();
    let connection = database.connection.lock().unwrap();
    assert!(
        connection
            .execute(
                "INSERT INTO tasks (kind, status) VALUES ('import', 'unknown')",
                []
            )
            .is_err()
    );
    assert!(
        connection
            .execute(
                "INSERT INTO tasks (kind, processed_count, total_count) VALUES ('verify', 2, 1)",
                []
            )
            .is_err()
    );
    connection
        .execute("INSERT INTO tasks (kind) VALUES ('import')", [])
        .unwrap();
    connection
        .execute(
            "INSERT INTO files (sha256, size_bytes) VALUES (?1, 0)",
            ["b".repeat(64)],
        )
        .unwrap();
    connection
        .execute("INSERT INTO collection_items (title) VALUES ('Test')", [])
        .unwrap();
    connection
        .execute(
            "INSERT INTO collection_item_files VALUES (1, 1, 'game.bin')",
            [],
        )
        .unwrap();
    connection
        .execute("DELETE FROM collection_items WHERE id = 1", [])
        .unwrap();
    let links: i64 = connection
        .query_row("SELECT count(*) FROM collection_item_files", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(links, 0);
    let content: i64 = connection
        .query_row("SELECT count(*) FROM files", [], |row| row.get(0))
        .unwrap();
    assert_eq!(content, 1);
}

#[test]
fn rejects_future_schema_without_downgrading_or_resetting_data() {
    let (_directory, path) = temporary_database();
    {
        let database = Database::open(&path).unwrap();
        let connection = database.connection.lock().unwrap();
        connection
            .execute(
                "INSERT INTO collection_items (title) VALUES ('Keep me')",
                [],
            )
            .unwrap();
        connection.pragma_update(None, "user_version", 99).unwrap();
    }
    assert!(matches!(
        Database::open(&path),
        Err(DatabaseError::NewerSchema {
            found: 99,
            supported: 3
        })
    ));
    let connection = Connection::open(&path).unwrap();
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 99);
    let title: String = connection
        .query_row("SELECT title FROM collection_items", [], |row| row.get(0))
        .unwrap();
    assert_eq!(title, "Keep me");
}

#[test]
fn upgrades_v1_without_losing_catalog_and_recovers_running_tasks() {
    let (_d, path) = temporary_database();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let c = Connection::open(&path).unwrap();
    c.execute_batch(include_str!("migrations/0001_initial.sql"))
        .unwrap();
    c.pragma_update(None, "user_version", 1).unwrap();
    c.execute(
        "INSERT INTO collection_items (title) VALUES ('Existing collection')",
        [],
    )
    .unwrap();
    c.execute(
        "INSERT INTO tasks (kind,status) VALUES ('import','running')",
        [],
    )
    .unwrap();
    drop(c);
    let db = Database::open(&path).unwrap();
    assert_eq!(db.info().unwrap().schema_version, 3);
    assert_eq!(db.info().unwrap().item_count, 1);
    let snapshot = crate::library::task_get(&db, "1").unwrap();
    assert!(matches!(
        snapshot.status,
        crate::contracts::TaskStatus::Interrupted
    ));
    assert_eq!(snapshot.revision, "1");
    assert!(snapshot.error.is_some());
}
