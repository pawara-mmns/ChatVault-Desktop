use super::{models::*, protocol, search};
use crate::storage::{imports::models::DateOrder, migrations, service::StorageService};
use rusqlite::{Connection, params};
use std::{fs, time::Instant};
use tauri::http::Request;

fn service(temp: &tempfile::TempDir) -> StorageService {
    StorageService::open(&temp.path().join("app"), &[]).unwrap()
}
fn import(s: &mut StorageService, temp: &tempfile::TempDir, name: &str, bytes: &[u8]) -> String {
    let path = temp.path().join(name);
    fs::write(&path, bytes).unwrap();
    let selection = s.select_source(&path).unwrap();
    s.analyze_import(&selection.token, DateOrder::DayFirst)
        .unwrap();
    s.confirm_import(&selection.token, name).unwrap().id
}
fn seed(s: &mut StorageService, count: usize) -> String {
    let archive = uuid::Uuid::new_v4().to_string();
    let conversation = uuid::Uuid::new_v4().to_string();
    let tx = s.connection.transaction().unwrap();
    tx.execute("INSERT INTO archives(id,title,source_type,storage_path) VALUES (?1,'Synthetic performance archive','chat-export',?2)",params![archive,format!("archives/{archive}")]).unwrap();
    tx.execute("INSERT INTO conversations(id,archive_id,title) VALUES (?1,?2,'Synthetic performance chat')",params![conversation,archive]).unwrap();
    {
        let mut statement=tx.prepare("INSERT INTO messages(id,conversation_id,sender,message_text,sequence_number,classification,original_timestamp,local_timestamp) VALUES (?1,?2,?3,?4,?5,'message','31/12/2026, 09:30','2026-12-31T09:30:00')").unwrap();
        for n in 0..count {
            statement
                .execute(params![
                    format!("{conversation}-{n}"),
                    conversation,
                    if n % 2 == 0 {
                        "Synthetic Alice"
                    } else {
                        "Synthetic BÖB"
                    },
                    format!(
                        "Fixture {n}: ආයුබෝවන් Grüße 👋\nsecond line literal [brackets] OR \"quote\""
                    ),
                    n as i64
                ])
                .unwrap();
        }
    }
    search::index_conversation(&tx, &conversation).unwrap();
    tx.commit().unwrap();
    conversation
}
#[test]
fn conversations_and_cursor_pages_preserve_imported_order() {
    let t = tempfile::tempdir().unwrap();
    let mut s = service(&t);
    let c = seed(&mut s, 137);
    let list = s.list_conversations("PERFORMANCE", None, 0, 50).unwrap();
    assert_eq!(list.total, 1);
    assert_eq!(list.items[0].message_count, 137);
    assert!(list.items[0].last_preview.as_ref().unwrap().contains("136"));
    let last = s.read_messages(&c, None, Direction::Latest, 50).unwrap();
    assert_eq!(last.items.len(), 50);
    assert_eq!(last.items[0].sequence, 87);
    assert!(last.has_older);
    assert!(!last.has_newer);
    let older = s
        .read_messages(&c, Some(&last.items[0].id), Direction::Older, 50)
        .unwrap();
    assert_eq!(older.items[0].sequence, 37);
    assert!(older.has_newer);
    let newer = s
        .read_messages(
            &c,
            Some(&older.items.last().unwrap().id),
            Direction::Newer,
            50,
        )
        .unwrap();
    assert_eq!(newer.items[0].id, last.items[0].id);
    assert!(
        s.read_messages(&c, Some("foreign-id"), Direction::Older, 50)
            .is_err()
    );
    assert!(s.conversation("missing").is_err());
}
#[test]
fn context_includes_correct_anchor_and_neighbours() {
    let t = tempfile::tempdir().unwrap();
    let mut s = service(&t);
    let c = seed(&mut s, 80);
    for n in [0, 30, 79] {
        let id = format!("{c}-{n}");
        let context = s.message_context(&c, &id).unwrap();
        assert_eq!(context.anchor_id.as_deref(), Some(id.as_str()));
        assert_eq!(context.items.iter().filter(|m| m.id == id).count(), 1);
        assert!(
            context
                .items
                .windows(2)
                .all(|w| w[0].sequence + 1 == w[1].sequence)
        );
        assert!(context.items.len() <= 51);
    }
    assert!(s.message_context(&c, "other").is_err());
}
#[test]
fn literal_unicode_search_and_sender_index_are_consistent() {
    let t = tempfile::tempdir().unwrap();
    let mut s = service(&t);
    let c = seed(&mut s, 6);
    for query in [
        "ආයුබෝවන්",
        "GRÜßE",
        "BÖB",
        "[brackets]",
        "OR \"quote\"",
        "second line",
        "Fixture 4",
    ] {
        let page = s.search_messages(query, None, None).unwrap();
        assert!(!page.items.is_empty(), "{query}");
        assert_eq!(page.mode, "trigram");
    }
    assert!(
        s.search_messages("no-such-text", None, None)
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        s.search_messages("\" OR NOT NEAR(*)", None, None)
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        s.search_messages("fixture", Some(&c), None)
            .unwrap()
            .items
            .len(),
        6
    );
    assert!(s.search_messages("", None, None).is_err());
    assert!(s.search_messages("valid", None, Some("../../")).is_err());
}
#[test]
fn short_sinhala_search_continues_after_a_bounded_scan() {
    let t = tempfile::tempdir().unwrap();
    let mut s = service(&t);
    let c = seed(&mut s, 2200);
    // Update isolated synthetic fixture and its search index together; no production edit API.
    s.connection
        .execute(
            "UPDATE messages SET message_text='rare ක' WHERE id=?1",
            [format!("{c}-2199")],
        )
        .unwrap();
    let first = s.search_messages("ක", None, None).unwrap();
    assert!(first.items.is_empty());
    assert_eq!(first.scanned, 2000);
    assert!(first.next_cursor.is_some());
    let second = s
        .search_messages("ක", None, first.next_cursor.as_deref())
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert!(second.next_cursor.is_none());
    assert_eq!(second.items[0].message_id, format!("{c}-2199"));
}
#[test]
fn migration_backfills_index_preserves_legacy_rows_and_is_repeatable() {
    let mut db = Connection::open_in_memory().unwrap();
    migrations::apply(&mut db, &migrations::MIGRATIONS[..2]).unwrap();
    db.execute_batch("INSERT INTO archives(id,title,source_type,storage_path) VALUES ('a','old','chat-export','archives/a'); INSERT INTO conversations(id,archive_id,title) VALUES ('c','a','old'); INSERT INTO messages(id,conversation_id,message_text) VALUES ('m1','c','ආයුබෝවන්'),('m2','c','Grüße');").unwrap();
    migrations::migrate(&mut db).unwrap();
    migrations::migrate(&mut db).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM message_search WHERE message_search MATCH ?1",
            ["\"ආයුබෝවන්\""],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT sequence_number FROM messages WHERE id='m2'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}
#[test]
fn imports_index_transactionally_and_remain_searchable_after_restart() {
    let t = tempfile::tempdir().unwrap();
    let mut s = service(&t);
    let archive = import(
        &mut s,
        &t,
        "chat.txt",
        "31/12/2026, 09:30 - Alice: saved Sinhala ආයුබෝවන්\ncontinuation".as_bytes(),
    );
    assert_eq!(
        s.search_messages("ආයුබෝවන්", None, None).unwrap().items.len(),
        1
    );
    drop(s);
    let s = service(&t);
    let list = s.list_conversations("", Some(&archive), 0, 1).unwrap();
    assert_eq!(list.items.len(), 1);
    assert!(
        s.read_messages(&list.items[0].id, None, Direction::Latest, 50)
            .unwrap()
            .items[0]
            .text
            .contains('\n')
    );
    assert_eq!(
        s.search_messages("Sinhala", None, None)
            .unwrap()
            .items
            .len(),
        1
    );
}
fn imported_image(s: &mut StorageService, t: &tempfile::TempDir) -> String {
    import(
        s,
        t,
        "image.zip",
        include_bytes!("../../../../src/test/fixtures/SyntheticMedia.zip"),
    );
    s.connection
        .query_row("SELECT id FROM archive_media LIMIT 1", [], |r| r.get(0))
        .unwrap()
}
#[test]
fn media_requires_database_ownership_token_and_valid_local_path() {
    let t = tempfile::tempdir().unwrap();
    let mut s = service(&t);
    let media = imported_image(&mut s, &t);
    assert!(s.media_metadata("../../Windows").is_err());
    assert!(s.media_metadata("unknown-id").is_err());
    let access = s.authorize_media(&media).unwrap();
    assert_eq!(access.metadata.mime, "image/png");
    assert_eq!(access.metadata.width, Some(1));
    let request = |token: &str, origin: &str| {
        Request::builder()
            .uri(format!("http://chatvault-media.localhost/{token}"))
            .header("Origin", origin)
            .body(Vec::new())
            .unwrap()
    };
    let response = s.media_response(request(&access.token, "http://tauri.localhost"));
    assert_eq!(response.status(), 200);
    assert_eq!(response.body().len(), 68);
    assert_eq!(response.headers()["X-Content-Type-Options"], "nosniff");
    assert_eq!(
        s.media_response(request(&access.token, "https://untrusted.example"))
            .status(),
        403
    );
    assert_eq!(
        s.media_response(request("not-a-token", "http://tauri.localhost"))
            .status(),
        403
    );
    s.revoke_media(&access.token).unwrap();
    assert_eq!(
        s.media_response(request(&access.token, "http://tauri.localhost"))
            .status(),
        404
    );
    let path = s.resolve_media(&media).unwrap().path;
    fs::remove_file(path).unwrap();
    assert_eq!(s.media_metadata(&media).unwrap_err().code, "media_missing");
}
#[test]
fn range_requests_are_bounded_and_invalid_ranges_fail() {
    assert_eq!(
        protocol::byte_range(Some("bytes=2-5"), 20).unwrap(),
        Some((2, 5))
    );
    assert_eq!(
        protocol::byte_range(Some("bytes=-3"), 20).unwrap(),
        Some((17, 19))
    );
    assert_eq!(
        protocol::byte_range(Some("bytes=0-"), 20_000_000).unwrap(),
        Some((0, 2 * 1024 * 1024 - 1))
    );
    for value in [
        "bytes=20-",
        "bytes=9-2",
        "bytes=0-1,3-4",
        "bytes=-0",
        "other",
    ] {
        assert!(protocol::byte_range(Some(value), 20).is_err());
    }
    let t = tempfile::tempdir().unwrap();
    let mut s = service(&t);
    let m = imported_image(&mut s, &t);
    let a = s.authorize_media(&m).unwrap();
    let response = s.media_response(
        Request::builder()
            .uri(format!("http://chatvault-media.localhost/{}", a.token))
            .header("Range", "bytes=0-7")
            .body(Vec::new())
            .unwrap(),
    );
    assert_eq!(response.status(), 206);
    assert_eq!(response.body(), b"\x89PNG\r\n\x1a\n");
}
#[test]
fn malicious_or_changed_media_references_are_rejected() {
    let t = tempfile::tempdir().unwrap();
    let mut s = service(&t);
    let m = imported_image(&mut s, &t);
    let path = s.resolve_media(&m).unwrap().path;
    fs::write(&path, b"not an image").unwrap();
    assert_eq!(s.media_metadata(&m).unwrap_err().code, "media_changed");
    s.connection
        .execute(
            "UPDATE archive_media SET storage_path='archives/other/media/file.png' WHERE id=?1",
            [&m],
        )
        .unwrap();
    assert_eq!(s.media_metadata(&m).unwrap_err().code, "media_denied");
}
#[test]
fn hard_link_media_escape_is_rejected() {
    let t = tempfile::tempdir().unwrap();
    let mut s = service(&t);
    let m = imported_image(&mut s, &t);
    let path = s.resolve_media(&m).unwrap().path;
    let outside = t.path().join("outside.png");
    fs::copy(&path, &outside).unwrap();
    fs::remove_file(&path).unwrap();
    fs::hard_link(&outside, &path).unwrap();
    assert!(s.media_metadata(&m).is_err());
}
#[test]
fn image_audio_metadata_ranges_and_invalid_signature_are_checked() {
    let t = tempfile::tempdir().unwrap();
    let mut s = service(&t);
    import(
        &mut s,
        &t,
        "playback.zip",
        include_bytes!("../../../../src/test/fixtures/SyntheticViewerMedia.zip"),
    );
    let c = s
        .list_conversations("", None, 0, 1)
        .unwrap()
        .items
        .remove(0);
    let media = s.conversation_media(&c.id, 0).unwrap();
    assert_eq!(media.items.len(), 3);
    let image = media
        .items
        .iter()
        .find(|m| m.filename.ends_with(".png"))
        .unwrap()
        .media_id
        .as_ref()
        .unwrap();
    let meta = s.media_metadata(image).unwrap();
    assert_eq!((meta.width, meta.height), (Some(160), Some(100)));
    let audio = media
        .items
        .iter()
        .find(|m| m.filename.ends_with(".wav"))
        .unwrap()
        .media_id
        .as_ref()
        .unwrap();
    let access = s.authorize_media(audio).unwrap();
    assert_eq!(access.metadata.mime, "audio/wav");
    let response = s.media_response(
        Request::builder()
            .uri(format!("http://chatvault-media.localhost/{}", access.token))
            .header("Range", "bytes=0-43")
            .body(Vec::new())
            .unwrap(),
    );
    assert_eq!(response.status(), 206);
    assert_eq!(&response.body()[..4], b"RIFF");
    let path = s.resolve_media(image).unwrap().path;
    let size = s.media_metadata(image).unwrap().size_bytes;
    fs::write(path, vec![0u8; size as usize]).unwrap();
    assert_eq!(
        s.media_metadata(image).unwrap_err().code,
        "media_unsupported"
    );
}
#[test]
fn a_search_index_failure_rolls_back_the_entire_import() {
    let t = tempfile::tempdir().unwrap();
    let mut s = service(&t);
    let path = t.path().join("rollback.txt");
    fs::write(&path, b"31/12/2026, 09:30 - Alice: Synthetic rollback").unwrap();
    let selection = s.select_source(&path).unwrap();
    s.analyze_import(&selection.token, DateOrder::Auto).unwrap();
    s.connection
        .execute_batch("DROP TABLE message_search")
        .unwrap();
    assert!(s.confirm_import(&selection.token, "Rollback").is_err());
    assert_eq!(
        s.connection
            .query_row("SELECT count(*) FROM archives", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        s.connection
            .query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(path.exists());
}
#[cfg(unix)]
#[test]
fn symlink_media_escape_is_rejected() {
    let t = tempfile::tempdir().unwrap();
    let mut s = service(&t);
    let m = imported_image(&mut s, &t);
    let path = s.resolve_media(&m).unwrap().path;
    let outside = t.path().join("outside.png");
    fs::rename(&path, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    assert!(s.media_metadata(&m).is_err());
}
#[test]
fn performance_12000_messages_has_bounded_pages_and_search() {
    let t = tempfile::tempdir().unwrap();
    let mut s = service(&t);
    let started = Instant::now();
    let c = seed(&mut s, 12000);
    let populate = started.elapsed();
    let started = Instant::now();
    let mut page = s.read_messages(&c, None, Direction::Latest, 60).unwrap();
    let mut seen = page.items.len();
    let mut max_page = seen;
    while page.has_older {
        page = s
            .read_messages(&c, Some(&page.items[0].id), Direction::Older, 60)
            .unwrap();
        seen += page.items.len();
        max_page = max_page.max(page.items.len());
    }
    let paging = started.elapsed();
    assert_eq!(seen, 12000);
    assert_eq!(max_page, 60);
    let started = Instant::now();
    let hit = s.search_messages("Fixture 9876:", None, None).unwrap();
    let search_time = started.elapsed();
    assert_eq!(hit.items.len(), 1);
    println!(
        "PERF synthetic 12000: seed+FTS={populate:?}; 200 cursor pages={paging:?}; exact substring={search_time:?}; max page={max_page}"
    );
}
