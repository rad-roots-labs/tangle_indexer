#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use rusqlite::params;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tracing::{info, warn};

use crate::{
    audit,
    domain::{
        indexer::{
            kind::IndexerEventKind,
            models::{
                EventCommentIndexes, EventFollowIndexes, EventIndexes, EventJobFeedbackIndexes,
                EventJobRequestIndexes, EventJobResultIndexes, EventListingIndexes,
                EventPostIndexes, EventProfileIndexes, EventReactionIndexes, WriteEventIndexes,
            },
        },
        resolvers::profile::ProfileResolver,
    },
    relay::{event::RelayIndexerEvent, record::RelayEventRecord},
    utils::{
        db::IndexerDb,
        sqlite::{sqlite_conn, sqlite_stmt},
    },
    Settings,
};
use radroots_events::kinds::{KIND_JOB_REQUEST_MIN, KIND_JOB_RESULT_MIN};

const TREE_RAW: &str = "hashes";
const TREE_EVENTS_PROFILE: &str = "profile_events";
const TREE_EVENTS_POST: &str = "post_events";
const TREE_EVENTS_FOLLOW: &str = "follow_events";
const TREE_EVENTS_LISTING: &str = "listing_events";
const TREE_EVENTS_REACTION: &str = "reaction_events";
const TREE_EVENTS_COMMENT: &str = "comment_events";
const TREE_EVENTS_JOB_REQUEST: &str = "job_request_events";
const TREE_EVENTS_JOB_RESULT: &str = "job_result_events";
const TREE_EVENTS_JOB_FEEDBACK: &str = "job_feedback_events";
const TREE_STATS: &str = "stats";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CursorMode {
    RowId,
    CreatedAt,
}

#[derive(Debug, Default)]
struct CursorState {
    last_created_at: u32,
    last_event_hash: String,
    last_rowid: u64,
}

impl CursorState {
    fn load(db_idx: &IndexerDb) -> Result<Self> {
        let last_created_at = db_idx
            .get_raw(TREE_STATS, "last_created_at")?
            .and_then(|ivec| parse_u32(ivec.as_ref(), "last_created_at"))
            .unwrap_or(0);
        let last_event_hash = db_idx
            .get_raw(TREE_STATS, "last_event_hash")?
            .and_then(|ivec| parse_string(ivec.as_ref(), "last_event_hash"))
            .unwrap_or_default();
        let last_rowid = db_idx
            .get_raw(TREE_STATS, "last_rowid")?
            .and_then(|ivec| parse_u64(ivec.as_ref(), "last_rowid"))
            .unwrap_or(0);

        Ok(Self {
            last_created_at,
            last_event_hash,
            last_rowid,
        })
    }
}

fn parse_u32(raw: &[u8], label: &str) -> Option<u32> {
    if raw.len() != 4 {
        warn!(len = raw.len(), label, "Ignoring invalid cursor value");
        return None;
    }
    let arr: [u8; 4] = raw.try_into().ok()?;
    Some(u32::from_be_bytes(arr))
}

fn parse_u64(raw: &[u8], label: &str) -> Option<u64> {
    if raw.len() != 8 {
        warn!(len = raw.len(), label, "Ignoring invalid cursor value");
        return None;
    }
    let arr: [u8; 8] = raw.try_into().ok()?;
    Some(u64::from_be_bytes(arr))
}

fn parse_string(raw: &[u8], label: &str) -> Option<String> {
    match std::str::from_utf8(raw) {
        Ok(value) => Some(value.to_string()),
        Err(err) => {
            warn!(error = %err, label, "Ignoring invalid cursor value");
            None
        }
    }
}

struct EventBatch {
    events_by_kind: HashMap<IndexerEventKind, Vec<RelayIndexerEvent>>,
    next_created: Option<(u32, String)>,
    next_rowid: Option<u64>,
    record_count: usize,
}

impl EventBatch {
    fn from_records(records: Vec<RelayEventRecord>) -> Result<Self> {
        let record_count = records.len();
        let next_created = records
            .last()
            .map(|rec| (rec.created_at, rec.event_hash.clone()));
        let next_rowid = records.last().and_then(|rec| rec.rowid);
        let mut events_by_kind: HashMap<IndexerEventKind, Vec<RelayIndexerEvent>> =
            HashMap::with_capacity(IndexerEventKind::GROUPS.len());

        for rec in records {
            let iev = RelayIndexerEvent::try_from(rec)?;
            audit::log_indexer_event(&iev);
            events_by_kind.entry(iev.kind.group()).or_default().push(iev);
        }

        Ok(Self {
            events_by_kind,
            next_created,
            next_rowid,
            record_count,
        })
    }
}

struct RelayQueries {
    created: String,
    rowid: String,
}

fn build_relay_queries() -> RelayQueries {
    let relay_kind_filter = IndexerEventKind::relay_kind_filter_sql();
    let created = format!(
        "SELECT hex(event_hash), hex(author), created_at, kind, content FROM event WHERE ({}) AND (created_at > ? OR (created_at = ? AND hex(event_hash) > ?)) ORDER BY created_at ASC, hex(event_hash) ASC",
        relay_kind_filter
    );
    let rowid = format!(
        "SELECT rowid, hex(event_hash), hex(author), created_at, kind, content FROM event WHERE ({}) AND rowid > ? ORDER BY rowid ASC",
        relay_kind_filter
    );
    RelayQueries { created, rowid }
}

fn resolve_cursor_mode(relay_db: &rusqlite::Connection, rowid_query: &str) -> CursorMode {
    match sqlite_stmt(relay_db, rowid_query) {
        Ok(_) => CursorMode::RowId,
        Err(err) => {
            warn!(
                error = %err,
                "Rowid cursor unavailable, falling back to created_at cursor"
            );
            CursorMode::CreatedAt
        }
    }
}

fn load_records(
    relay_db: &rusqlite::Connection,
    mode: CursorMode,
    queries: &RelayQueries,
    cursor: &CursorState,
) -> Result<Vec<RelayEventRecord>> {
    match mode {
        CursorMode::RowId => {
            let mut stmt = sqlite_stmt(relay_db, &queries.rowid)
                .context("Could not prepare rowid event query")?;
            let rows = stmt.query_map(
                params![cursor.last_rowid],
                RelayEventRecord::from_row_with_rowid,
            )?;
            rows.collect::<Result<Vec<_>, _>>()
                .context("collecting RelayEventRecord rows")
        }
        CursorMode::CreatedAt => {
            let mut stmt = sqlite_stmt(relay_db, &queries.created)
                .context("Could not prepare created_at event query")?;
            let rows = stmt.query_map(
                params![
                    cursor.last_created_at,
                    cursor.last_created_at,
                    &cursor.last_event_hash
                ],
                RelayEventRecord::from_row,
            )?;
            rows.collect::<Result<Vec<_>, _>>()
                .context("collecting RelayEventRecord rows")
        }
    }
}

fn update_cursor(
    db_idx: &IndexerDb,
    cursor: &mut CursorState,
    mode: CursorMode,
    batch: &mut EventBatch,
) -> Result<bool> {
    let mut cursor_updated = false;
    match mode {
        CursorMode::CreatedAt => {
            if let Some((created_at, event_hash)) = batch.next_created.take() {
                cursor.last_created_at = created_at;
                cursor.last_event_hash = event_hash;
                db_idx.insert_raw(
                    TREE_STATS,
                    "last_created_at",
                    &cursor.last_created_at.to_be_bytes(),
                )?;
                db_idx.insert_raw(
                    TREE_STATS,
                    "last_event_hash",
                    cursor.last_event_hash.as_bytes(),
                )?;
                cursor_updated = true;
            }
        }
        CursorMode::RowId => {
            if let Some(rowid) = batch.next_rowid.take() {
                cursor.last_rowid = rowid;
                db_idx.insert_raw(TREE_STATS, "last_rowid", &cursor.last_rowid.to_be_bytes())?;
                cursor_updated = true;
            }
        }
    }
    Ok(cursor_updated)
}

#[derive(Default)]
struct ChangeFlags {
    profiles: bool,
    posts: bool,
    follows: bool,
    listings: bool,
    reactions: bool,
    comments: bool,
    job_requests: bool,
    job_results: bool,
    job_feedback: bool,
}

impl ChangeFlags {
    fn needs_profiles(&self) -> bool {
        self.profiles
            || self.listings
            || self.reactions
            || self.comments
            || self.posts
            || self.follows
            || self.job_requests
            || self.job_results
            || self.job_feedback
    }
}

fn insert_event(
    db_idx: &IndexerDb,
    tree: &str,
    raw_tree: &str,
    ev: &RelayIndexerEvent,
) -> Result<bool> {
    let id = &ev.id;
    let hash = &ev.hash;
    let skip = if let Some(old) = db_idx.get_raw(raw_tree, id)? {
        old.as_ref() == hash.as_bytes()
    } else {
        false
    };
    if skip {
        return Ok(false);
    }
    db_idx.insert(tree, id, ev)?;
    db_idx.insert_raw(raw_tree, id, hash.as_bytes())?;
    Ok(true)
}

fn insert_events(
    db_idx: &IndexerDb,
    tree: &str,
    raw_tree: &str,
    events: &[RelayIndexerEvent],
) -> Result<bool> {
    let mut any_new = false;
    for ev in events {
        if insert_event(db_idx, tree, raw_tree, ev)? {
            any_new = true;
        }
    }
    Ok(any_new)
}

fn write_indexes<T: WriteEventIndexes>(
    settings: &Settings,
    label: Option<&str>,
    indexes: T,
) -> Result<()> {
    let mut updated = Vec::new();
    indexes.write(settings, &mut updated)?;
    match label {
        Some(label) => info!(
            written = updated.len(),
            "Written {} {} index files",
            updated.len(),
            label
        ),
        None => info!(written = updated.len(), "Written {} index files", updated.len()),
    }
    Ok(())
}

pub async fn run(settings: Settings) -> Result<()> {
    let db_idx = IndexerDb::open(&format!("{}/indexer_db", settings.indexer.data_dir))?;
    let mut cursor = CursorState::load(&db_idx)?;
    let relay_queries = build_relay_queries();

    let mut profiles = ProfileResolver::default();
    let mut profiles_loaded = false;
    let mut cursor_mode: Option<CursorMode> = None;

    loop {
        let iteration_start = Instant::now();
        let relay_db = sqlite_conn(&settings.relay.database_path).with_context(|| {
            format!(
                "Could not open relay DB at {}",
                settings.relay.database_path
            )
        })?;
        if cursor_mode.is_none() {
            cursor_mode = Some(resolve_cursor_mode(&relay_db, &relay_queries.rowid));
        }

        let mode = cursor_mode.unwrap_or(CursorMode::CreatedAt);
        let records = load_records(&relay_db, mode, &relay_queries, &cursor)?;

        let mut batch = EventBatch::from_records(records)?;
        info!(record_count = batch.record_count, "Loaded relay records");

        let mut changes = ChangeFlags::default();
        let mut raw_listing_events: Option<Vec<RelayIndexerEvent>> = None;

        if let Some(profile_events) = batch.events_by_kind.remove(&IndexerEventKind::Profile) {
            if insert_events(&db_idx, TREE_EVENTS_PROFILE, TREE_RAW, &profile_events)? {
                let raw_profile_events: Vec<RelayIndexerEvent> =
                    db_idx.get_all(TREE_EVENTS_PROFILE)?;
                let indexed_profile_events = EventProfileIndexes::build(&raw_profile_events)?;
                write_indexes(&settings, None, indexed_profile_events)?;

                profiles = ProfileResolver::from_metadata(&raw_profile_events);
                profiles_loaded = true;
                audit::set_profile_resolver(profiles.clone());
                changes.profiles = true;
            }
        }

        if let Some(post_events) = batch.events_by_kind.remove(&IndexerEventKind::Post) {
            changes.posts = insert_events(&db_idx, TREE_EVENTS_POST, TREE_RAW, &post_events)?;
        }

        if let Some(follow_events) = batch.events_by_kind.remove(&IndexerEventKind::Follow) {
            changes.follows = insert_events(&db_idx, TREE_EVENTS_FOLLOW, TREE_RAW, &follow_events)?;
        }

        if let Some(listing_events) = batch.events_by_kind.remove(&IndexerEventKind::Listing) {
            if insert_events(&db_idx, TREE_EVENTS_LISTING, TREE_RAW, &listing_events)? {
                raw_listing_events = Some(db_idx.get_all(TREE_EVENTS_LISTING)?);
                changes.listings = true;
            }
        }

        if let Some(reaction_events) = batch.events_by_kind.remove(&IndexerEventKind::Reaction) {
            changes.reactions =
                insert_events(&db_idx, TREE_EVENTS_REACTION, TREE_RAW, &reaction_events)?;
        }

        if let Some(comment_events) = batch.events_by_kind.remove(&IndexerEventKind::Comment) {
            changes.comments =
                insert_events(&db_idx, TREE_EVENTS_COMMENT, TREE_RAW, &comment_events)?;
        }

        if let Some(job_request_events) =
            batch.events_by_kind.remove(&IndexerEventKind::JobRequest(KIND_JOB_REQUEST_MIN))
        {
            changes.job_requests = insert_events(
                &db_idx,
                TREE_EVENTS_JOB_REQUEST,
                TREE_RAW,
                &job_request_events,
            )?;
        }

        if let Some(job_result_events) =
            batch.events_by_kind.remove(&IndexerEventKind::JobResult(KIND_JOB_RESULT_MIN))
        {
            changes.job_results = insert_events(
                &db_idx,
                TREE_EVENTS_JOB_RESULT,
                TREE_RAW,
                &job_result_events,
            )?;
        }

        if let Some(job_feedback_events) =
            batch.events_by_kind.remove(&IndexerEventKind::JobFeedback)
        {
            changes.job_feedback = insert_events(
                &db_idx,
                TREE_EVENTS_JOB_FEEDBACK,
                TREE_RAW,
                &job_feedback_events,
            )?;
        }

        if !batch.events_by_kind.is_empty() {
            let kinds: Vec<IndexerEventKind> =
                batch.events_by_kind.keys().copied().collect();
            warn!(kinds = ?kinds, "Unhandled indexer event kinds");
        }

        if changes.needs_profiles() && !profiles_loaded {
            let raw_profile_events: Vec<RelayIndexerEvent> = db_idx.get_all(TREE_EVENTS_PROFILE)?;
            profiles = ProfileResolver::from_metadata(&raw_profile_events);
            profiles_loaded = true;
            audit::set_profile_resolver(profiles.clone());
        }

        if changes.reactions {
            let raw_reaction_events: Vec<RelayIndexerEvent> =
                db_idx.get_all(TREE_EVENTS_REACTION)?;
            let reaction_indexes =
                EventReactionIndexes::build_with_profiles(&raw_reaction_events, &profiles)?;
            write_indexes(&settings, Some("reaction"), reaction_indexes)?;
        }

        if changes.comments {
            let raw_comment_events: Vec<RelayIndexerEvent> = db_idx.get_all(TREE_EVENTS_COMMENT)?;
            let comment_indexes =
                EventCommentIndexes::build_with_profiles(&raw_comment_events, &profiles)?;
            write_indexes(&settings, Some("comment"), comment_indexes)?;
        }

        if changes.posts {
            let raw_post_events: Vec<RelayIndexerEvent> = db_idx.get_all(TREE_EVENTS_POST)?;
            let post_indexes = EventPostIndexes::build_with_profiles(&raw_post_events, &profiles)?;
            write_indexes(&settings, Some("post"), post_indexes)?;
        }

        if changes.follows {
            let raw_follow_events: Vec<RelayIndexerEvent> = db_idx.get_all(TREE_EVENTS_FOLLOW)?;
            let follow_indexes =
                EventFollowIndexes::build_with_profiles(&raw_follow_events, &profiles)?;
            write_indexes(&settings, Some("follow"), follow_indexes)?;
        }

        if changes.job_requests {
            let raw_job_request_events: Vec<RelayIndexerEvent> =
                db_idx.get_all(TREE_EVENTS_JOB_REQUEST)?;
            let job_request_indexes =
                EventJobRequestIndexes::build_with_profiles(&raw_job_request_events, &profiles)?;
            write_indexes(&settings, Some("job request"), job_request_indexes)?;
        }

        if changes.job_results {
            let raw_job_result_events: Vec<RelayIndexerEvent> =
                db_idx.get_all(TREE_EVENTS_JOB_RESULT)?;
            let job_result_indexes =
                EventJobResultIndexes::build_with_profiles(&raw_job_result_events, &profiles)?;
            write_indexes(&settings, Some("job result"), job_result_indexes)?;
        }

        if changes.job_feedback {
            let raw_job_feedback_events: Vec<RelayIndexerEvent> =
                db_idx.get_all(TREE_EVENTS_JOB_FEEDBACK)?;
            let job_feedback_indexes =
                EventJobFeedbackIndexes::build_with_profiles(&raw_job_feedback_events, &profiles)?;
            write_indexes(&settings, Some("job feedback"), job_feedback_indexes)?;
        }

        if changes.listings || changes.profiles {
            let raw_listing_events = match raw_listing_events.take() {
                Some(events) => events,
                None => db_idx.get_all(TREE_EVENTS_LISTING)?,
            };
            let listing_indexes =
                EventListingIndexes::build_with_profiles(&raw_listing_events, &profiles)?;
            write_indexes(&settings, Some("listing"), listing_indexes)?;
        }

        let cursor_updated = update_cursor(&db_idx, &mut cursor, mode, &mut batch)?;
        if cursor_updated {
            db_idx.flush()?;
        }

        let elapsed = iteration_start.elapsed();
        let interval = Duration::from_secs(settings.indexer.flush_interval);
        let delay = interval.saturating_sub(elapsed);
        info!(
            elapsed_ms = elapsed.as_millis(),
            sleeping_ms = delay.as_millis(),
            "Iteration complete"
        );
        tokio::time::sleep(delay).await;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_relay_queries, parse_string, parse_u32, parse_u64, resolve_cursor_mode, update_cursor,
        CursorMode, CursorState, EventBatch,
    };
    use crate::domain::indexer::kind::IndexerEventKind;
    use crate::relay::event::RelayRawEvent;
    use crate::relay::record::RelayEventRecord;
    use crate::utils::db::IndexerDb;
    use radroots_events::kinds::KIND_JOB_REQUEST_MIN;
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn make_record(rowid: u64, event_hash: &str, author: &str, created_at: u32, kind: u32) -> RelayEventRecord {
        let raw = RelayRawEvent {
            id: event_hash.to_string(),
            pubkey: author.to_string(),
            created_at,
            kind,
            tags: Vec::new(),
            content: "hello".to_string(),
            sig: "sig".to_string(),
        };
        let content = serde_json::to_string(&raw).expect("json");
        RelayEventRecord {
            rowid: Some(rowid),
            event_hash: event_hash.to_string(),
            author: author.to_string(),
            created_at,
            kind: IndexerEventKind::try_from(kind as u64).expect("kind"),
            content,
        }
    }

    #[test]
    fn parse_helpers_reject_invalid_lengths() {
        assert!(parse_u32(&[0u8; 3], "u32").is_none());
        assert!(parse_u64(&[0u8; 7], "u64").is_none());
    }

    #[test]
    fn parse_string_rejects_invalid_utf8() {
        assert!(parse_string(&[0xff, 0xfe], "str").is_none());
    }

    #[test]
    fn event_batch_groups_job_request_kinds() {
        let author = "a".repeat(64);
        let rec = make_record(
            1,
            "1".repeat(64).as_str(),
            &author,
            10,
            KIND_JOB_REQUEST_MIN + 1,
        );
        let batch = EventBatch::from_records(vec![rec]).expect("batch");
        assert!(batch
            .events_by_kind
            .contains_key(&IndexerEventKind::JobRequest(KIND_JOB_REQUEST_MIN)));
    }

    #[test]
    fn resolve_cursor_mode_falls_back_without_rowid() {
        let conn = rusqlite::Connection::open_in_memory().expect("conn");
        conn.execute(
            "CREATE TABLE event (event_hash BLOB PRIMARY KEY, author BLOB, created_at INTEGER, kind INTEGER, content TEXT) WITHOUT ROWID",
            [],
        )
        .expect("create table");
        let queries = build_relay_queries();
        let mode = resolve_cursor_mode(&conn, &queries.rowid);
        assert_eq!(mode, CursorMode::CreatedAt);
    }

    #[test]
    fn resolve_cursor_mode_uses_rowid_when_available() {
        let conn = rusqlite::Connection::open_in_memory().expect("conn");
        conn.execute(
            "CREATE TABLE event (event_hash BLOB, author BLOB, created_at INTEGER, kind INTEGER, content TEXT)",
            [],
        )
        .expect("create table");
        let queries = build_relay_queries();
        let mode = resolve_cursor_mode(&conn, &queries.rowid);
        assert_eq!(mode, CursorMode::RowId);
    }

    #[test]
    fn update_cursor_writes_created_at_state() {
        let dir = tempdir().expect("tempdir");
        let db_idx = IndexerDb::open(dir.path().join("db").to_str().expect("path"))
            .expect("open db");
        let mut cursor = CursorState::default();
        let mut batch = EventBatch {
            events_by_kind: HashMap::new(),
            next_created: Some((42, "hash".to_string())),
            next_rowid: None,
            record_count: 0,
        };

        let updated = update_cursor(&db_idx, &mut cursor, CursorMode::CreatedAt, &mut batch)
            .expect("update cursor");
        assert!(updated);
        assert_eq!(cursor.last_created_at, 42);
        assert_eq!(cursor.last_event_hash, "hash");
        db_idx.flush().expect("flush");
        let stored = db_idx
            .get_raw("stats", "last_created_at")
            .expect("get raw")
            .expect("value");
        assert_eq!(stored.as_ref(), &42u32.to_be_bytes());
    }

    #[test]
    fn update_cursor_writes_rowid_state() {
        let dir = tempdir().expect("tempdir");
        let db_idx = IndexerDb::open(dir.path().join("db").to_str().expect("path"))
            .expect("open db");
        let mut cursor = CursorState::default();
        let mut batch = EventBatch {
            events_by_kind: HashMap::new(),
            next_created: None,
            next_rowid: Some(7),
            record_count: 0,
        };

        let updated = update_cursor(&db_idx, &mut cursor, CursorMode::RowId, &mut batch)
            .expect("update cursor");
        assert!(updated);
        assert_eq!(cursor.last_rowid, 7);
        db_idx.flush().expect("flush");
        let stored = db_idx
            .get_raw("stats", "last_rowid")
            .expect("get raw")
            .expect("value");
        assert_eq!(stored.as_ref(), &7u64.to_be_bytes());
    }
}
