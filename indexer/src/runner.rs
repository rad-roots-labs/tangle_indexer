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

#[derive(Clone, Copy, Debug)]
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

    let relay_kind_filter = IndexerEventKind::relay_kind_filter_sql();
    let relay_query_created = format!(
        "SELECT hex(event_hash), hex(author), created_at, kind, content FROM event WHERE ({}) AND (created_at > ? OR (created_at = ? AND hex(event_hash) > ?)) ORDER BY created_at ASC, hex(event_hash) ASC",
        relay_kind_filter
    );
    let relay_query_rowid = format!(
        "SELECT rowid, hex(event_hash), hex(author), created_at, kind, content FROM event WHERE ({}) AND rowid > ? ORDER BY rowid ASC",
        relay_kind_filter
    );

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
            cursor_mode = match sqlite_stmt(&relay_db, &relay_query_rowid) {
                Ok(_) => Some(CursorMode::RowId),
                Err(err) => {
                    warn!(
                        error = %err,
                        "Rowid cursor unavailable, falling back to created_at cursor"
                    );
                    Some(CursorMode::CreatedAt)
                }
            };
        }

        let mode = cursor_mode.unwrap_or(CursorMode::CreatedAt);
        let records: Vec<RelayEventRecord> = match mode {
            CursorMode::RowId => {
                let mut stmt = sqlite_stmt(&relay_db, &relay_query_rowid)
                    .context("Could not prepare rowid event query")?;
                let rows =
                    stmt.query_map(params![cursor.last_rowid], RelayEventRecord::from_row_with_rowid)?;
                rows.collect::<Result<Vec<_>, _>>()
                    .context("collecting RelayEventRecord rows")?
            }
            CursorMode::CreatedAt => {
                let mut stmt = sqlite_stmt(&relay_db, &relay_query_created)
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
                    .context("collecting RelayEventRecord rows")?
            }
        };

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
                    db_idx.insert_raw(
                        TREE_STATS,
                        "last_rowid",
                        &cursor.last_rowid.to_be_bytes(),
                    )?;
                    cursor_updated = true;
                }
            }
        }
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
