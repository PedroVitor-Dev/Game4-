//! Manual, read-only checking of registered physical copies of item content.
use crate::{contracts::*, database::Database, library};
use rusqlite::{OptionalExtension, params};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, IpcError>;
const MAX_CHECKS: usize = 500_000;
fn db_error(_: rusqlite::Error) -> IpcError {
    IpcError::new(ErrorCode::DatabaseUnavailable)
}
fn lock(db: &Database) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>> {
    db.connection
        .lock()
        .map_err(|_| IpcError::new(ErrorCode::DatabaseUnavailable))
}
fn ensure_item(c: &rusqlite::Connection, id: i64) -> Result<()> {
    if !c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM collection_items WHERE id=?1)",
            [id],
            |r| r.get::<_, bool>(0),
        )
        .map_err(db_error)?
    {
        return Err(IpcError::new(ErrorCode::NotFound));
    }
    Ok(())
}
// Call under ImportState.gate, also used by import and backup admission.
pub fn accept(db: &Database, request: &StartVerifyRequest) -> Result<(TaskAccepted, bool)> {
    let id = crate::manifests::item_id(&request.item_id)?;
    let uuid = uuid::Uuid::parse_str(&request.request_id)
        .map_err(|_| library::invalid(ErrorField::RequestId))?;
    if uuid.get_version_num() != 4
        || uuid.get_variant() != uuid::Variant::RFC4122
        || uuid.to_string() != request.request_id
    {
        return Err(library::invalid(ErrorField::RequestId));
    }
    let marker = format!("verify:{id}");
    let c = lock(db)?;
    let previous: Option<(i64, String, String, Option<String>)> = c
        .query_row(
            "SELECT id,status,kind,selection_token FROM tasks WHERE request_id=?1",
            [&request.request_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(db_error)?;
    if let Some((task, status, kind, payload)) = previous {
        if kind != "verify" || payload.as_deref() != Some(&marker) {
            return Err(library::invalid(ErrorField::RequestId));
        }
        let status = serde_json::from_value(serde_json::Value::String(status))
            .map_err(|_| IpcError::new(ErrorCode::InvalidResponse))?;
        return Ok((
            TaskAccepted {
                task_id: task.to_string(),
                status,
            },
            false,
        ));
    }
    ensure_item(&c, id)?;
    c.execute("INSERT INTO tasks(kind,request_id,selection_token,updated_at) VALUES('verify',?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",params![request.request_id,marker]).map_err(db_error)?;
    Ok((
        TaskAccepted {
            task_id: c.last_insert_rowid().to_string(),
            status: TaskStatus::Queued,
        },
        true,
    ))
}
struct Target {
    id: i64,
    file: i64,
    path: String,
    size: i64,
    hash: String,
}
fn targets(db: &Database, item: i64) -> Result<Vec<Target>> {
    let c = lock(db)?;
    ensure_item(&c, item)?;
    let mut q=c.prepare("SELECT l.id,l.file_id,l.path,f.size_bytes,f.sha256 FROM file_locations l JOIN files f ON f.id=l.file_id WHERE l.file_id IN(SELECT file_id FROM collection_item_files WHERE item_id=?1) ORDER BY l.id LIMIT 500001").map_err(db_error)?;
    let rows = q
        .query_map([item], |r| {
            Ok(Target {
                id: r.get(0)?,
                file: r.get(1)?,
                path: r.get(2)?,
                size: r.get(3)?,
                hash: r.get(4)?,
            })
        })
        .map_err(db_error)?;
    let rows = rows
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db_error)?;
    if rows.len() > MAX_CHECKS {
        return Err(IpcError::new(ErrorCode::VerificationLimitExceeded));
    }
    Ok(rows)
}
pub fn run(db: &Database, task: &str, item: &str) -> Result<()> {
    run_observed(db, task, item, &mut || {})
}
fn run_observed(db: &Database, task: &str, item: &str, observer: &mut impl FnMut()) -> Result<()> {
    let mut processed = 0;
    let mut total = None;
    let result = (|| {
        library::update(db, task, "running", 0, None, None)?;
        let targets = targets(db, crate::manifests::item_id(item)?)?;
        total = Some(targets.len());
        library::update(db, task, "running", 0, total, None)?;
        let mut buffer = vec![0u8; 1024 * 1024];
        let mut observations = Vec::with_capacity(targets.len());
        let mut progress = Instant::now();
        for target in &targets {
            let path = PathBuf::from(&target.path);
            let status = match library::hash_file_with_buffer(&path, &mut buffer, observer) {
                Ok((size, hash, _)) if size == target.size && hash == target.hash => "verified",
                Ok(_) => "changed",
                Err(e) if matches!(e.code, ErrorCode::SourceChanged) => "changed",
                Err(_) => match std::fs::symlink_metadata(&path) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => "missing",
                    _ => "inaccessible",
                },
            };
            // Record time of observation, rather than the later batch commit.
            let checked = lock(db)?
                .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |r| {
                    r.get::<_, String>(0)
                })
                .map_err(db_error)?;
            observations.push((status, checked));
            processed += 1;
            if progress.elapsed() >= Duration::from_millis(200) {
                library::update(db, task, "running", processed, total, None)?;
                progress = Instant::now();
            }
        }
        // All observations and task completion become visible together. A failed
        // or interrupted run leaves previous health observations unchanged.
        let mut c = lock(db)?;
        let tx = c.transaction().map_err(db_error)?;
        for (target, (status, checked)) in targets.iter().zip(observations) {
            let n=tx.execute("UPDATE file_locations SET verification_status=?1,last_verified_at=?2 WHERE id=?3 AND file_id=?4 AND path=?5",params![status,checked,target.id,target.file,target.path]).map_err(db_error)?;
            if n != 1 {
                return Err(IpcError::new(ErrorCode::SourceChanged));
            }
        }
        tx.execute("UPDATE tasks SET status='succeeded',processed_count=?2,total_count=?2,error_code=NULL,revision=revision+1,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),finished_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![task,processed as i64]).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(())
    })();
    if let Err(ref e) = result {
        library::update(db, task, "failed", processed, total, Some(e.code))?;
    }
    result
}
pub fn summary(db: &Database, item: &str) -> Result<LibraryIntegrity> {
    let id = crate::manifests::item_id(item)?;
    let c = lock(db)?;
    ensure_item(&c, id)?;
    let mut summary = LibraryIntegrity {
        verified: 0,
        changed: 0,
        missing: 0,
        inaccessible: 0,
        unverified: 0,
        unlocated: 0,
        last_checked_at: None,
    };
    let mut q=c.prepare("SELECT verification_status,count(*),max(last_verified_at) FROM file_locations WHERE file_id IN(SELECT file_id FROM collection_item_files WHERE item_id=?1) GROUP BY verification_status").map_err(db_error)?;
    for row in q
        .query_map([id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, u32>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(db_error)?
    {
        let (status, count, time) = row.map_err(db_error)?;
        match status.as_str() {
            "verified" => summary.verified = count,
            "changed" => summary.changed = count,
            "missing" => summary.missing = count,
            "inaccessible" => summary.inaccessible = count,
            _ => summary.unverified = count,
        }
        summary.last_checked_at = summary.last_checked_at.max(time);
    }
    summary.unlocated=c.query_row("SELECT count(DISTINCT file_id) FROM collection_item_files cf WHERE item_id=?1 AND NOT EXISTS(SELECT 1 FROM file_locations l WHERE l.file_id=cf.file_id)",[id],|r|r.get(0)).map_err(db_error)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;
    fn setup() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("db.sqlite3")).unwrap();
        (dir, db)
    }
    fn import(db: &Database, paths: Vec<PathBuf>) {
        let req = StartImportRequest {
            request_id: Uuid::new_v4().to_string(),
            selection_token: Uuid::new_v4().to_string(),
        };
        let task = library::create_task(db, &req).unwrap();
        library::run_import(db, &task.task_id, paths).unwrap();
    }
    fn request(item: &str) -> StartVerifyRequest {
        StartVerifyRequest {
            request_id: Uuid::new_v4().to_string(),
            item_id: item.into(),
        }
    }
    fn check(db: &Database, item: &str) -> String {
        let (task, fresh) = accept(db, &request(item)).unwrap();
        assert!(fresh);
        run(db, &task.task_id, item).unwrap();
        task.task_id
    }
    #[test]
    fn checks_all_distinct_registered_copies_and_preserves_identity() {
        let (dir, db) = setup();
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        let empty = dir.path().join("empty.bin");
        fs::write(&a, b"abc").unwrap();
        fs::write(&b, b"abc").unwrap();
        fs::write(&empty, b"").unwrap();
        import(&db, vec![a.clone(), b.clone(), empty]);
        fs::write(&a, b"xyz").unwrap();
        let task = check(&db, "1");
        let health = summary(&db, "1").unwrap();
        assert_eq!((health.verified, health.changed), (1, 1));
        assert!(health.last_checked_at.is_some());
        assert_eq!(library::task_get(&db, &task).unwrap().processed_count, "2");
        fs::remove_file(&b).unwrap();
        check(&db, "1");
        assert_eq!(summary(&db, "1").unwrap().missing, 1);
        fs::write(&a, b"longer bytes").unwrap();
        check(&db, "1");
        assert_eq!(summary(&db, "1").unwrap().changed, 1);
        check(&db, "3");
        assert_eq!(summary(&db, "3").unwrap().verified, 1);
        let hash: String = lock(&db)
            .unwrap()
            .query_row("SELECT sha256 FROM files WHERE size_bytes=3", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            hash,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        drop(db);
        let reopened = Database::open(&dir.path().join("db.sqlite3")).unwrap();
        assert_eq!(summary(&reopened, "1").unwrap().missing, 1);
    }
    #[test]
    fn replay_and_cross_command_request_conflicts_are_rejected() {
        let (dir, db) = setup();
        let a = dir.path().join("a");
        fs::write(&a, b"abc").unwrap();
        import(&db, vec![a]);
        let req = request("1");
        let (task, fresh) = accept(&db, &req).unwrap();
        assert!(fresh);
        let (replay, fresh) = accept(&db, &req).unwrap();
        assert!(!fresh);
        assert_eq!(task.task_id, replay.task_id);
        assert!(
            accept(
                &db,
                &StartVerifyRequest {
                    item_id: "2".into(),
                    ..req.clone()
                }
            )
            .is_err()
        );
        assert!(
            library::existing(
                &db,
                &StartImportRequest {
                    request_id: req.request_id,
                    selection_token: "verify:1".into()
                }
            )
            .is_err()
        );
        let imp = StartImportRequest {
            request_id: Uuid::new_v4().to_string(),
            selection_token: Uuid::new_v4().to_string(),
        };
        library::create_task(&db, &imp).unwrap();
        assert!(
            accept(
                &db,
                &StartVerifyRequest {
                    request_id: imp.request_id,
                    item_id: "1".into()
                }
            )
            .is_err()
        );
        assert!(accept(&db, &request("01")).is_err());
        assert!(accept(&db, &request("999")).is_err());
        assert!(
            accept(
                &db,
                &StartVerifyRequest {
                    request_id: "invalid".into(),
                    item_id: "1".into()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn directory_replacement_is_inaccessible_and_unlocated_is_explicit() {
        let (dir, db) = setup();
        let a = dir.path().join("a");
        fs::write(&a, b"abc").unwrap();
        import(&db, vec![a.clone()]);
        fs::remove_file(&a).unwrap();
        fs::create_dir(&a).unwrap();
        check(&db, "1");
        assert_eq!(summary(&db, "1").unwrap().inaccessible, 1);
        lock(&db)
            .unwrap()
            .execute("DELETE FROM file_locations", [])
            .unwrap();
        check(&db, "1");
        let s = summary(&db, "1").unwrap();
        assert_eq!((s.unlocated, s.verified), (1, 0));
    }
    #[test]
    fn reads_remain_available_during_hash_and_observations_commit_atomically() {
        let (dir, db) = setup();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::write(&a, vec![1u8; 2 * 1024 * 1024]).unwrap();
        fs::write(&b, vec![1u8; 2 * 1024 * 1024]).unwrap();
        import(&db, vec![a, b]);
        let (task, _) = accept(&db, &request("1")).unwrap();
        let mut calls = 0;
        run_observed(&db, &task.task_id, "1", &mut || {
            calls += 1;
            assert!(matches!(
                library::task_get(&db, &task.task_id).unwrap().status,
                TaskStatus::Running
            ));
            assert_eq!(summary(&db, "1").unwrap().verified, 2); // Import observation still intact.
            assert!(db.info().is_ok());
        })
        .unwrap();
        assert!(calls >= 2);
        // Abort the final transaction after the first update. Neither locations
        // nor success can leak from a partially applied batch.
        lock(&db).unwrap().execute_batch("UPDATE file_locations SET verification_status='unverified',last_verified_at=NULL; CREATE TRIGGER reject_check BEFORE UPDATE OF verification_status ON file_locations WHEN NEW.id=2 BEGIN SELECT RAISE(ABORT,'test rollback'); END;").unwrap();
        let (task, _) = accept(&db, &request("2")).unwrap();
        assert!(run(&db, &task.task_id, "2").is_err());
        assert!(matches!(
            library::task_get(&db, &task.task_id).unwrap().status,
            TaskStatus::Failed
        ));
        assert_eq!(summary(&db, "2").unwrap().unverified, 2);
    }
    #[cfg(windows)]
    #[test]
    fn locked_file_is_inaccessible() {
        use std::os::windows::fs::OpenOptionsExt;
        let (dir, db) = setup();
        let a = dir.path().join("a");
        fs::write(&a, b"abc").unwrap();
        import(&db, vec![a.clone()]);
        let _exclusive = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&a)
            .unwrap();
        check(&db, "1");
        assert_eq!(summary(&db, "1").unwrap().inaccessible, 1);
    }
}
