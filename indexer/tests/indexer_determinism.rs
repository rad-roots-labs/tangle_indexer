use radroots_radroots_indexer::config::{Indexer, Listings, Relay, Settings};
use radroots_radroots_indexer::domain::indexer::kind::IndexerEventKind;
use radroots_radroots_indexer::domain::indexer::models::{
    EventCommentIndexes, EventFollowIndexes, EventIndexes, EventJobFeedbackIndexes,
    EventJobRequestIndexes, EventJobResultIndexes, EventListingIndexes, EventPostIndexes,
    EventProfileIndexes, EventReactionIndexes, WriteEventIndexes,
};
use radroots_radroots_indexer::domain::resolvers::profile::ProfileResolver;
use radroots_radroots_indexer::relay::event::RelayIndexerEvent;
use radroots_events::kinds::{KIND_JOB_REQUEST_MIN, KIND_JOB_RESULT_MIN};
use std::path::Path;
use tempfile::tempdir;

fn settings_for(root: &Path) -> Settings {
    Settings {
        indexer: Indexer {
            data_dir: root.join("data").to_string_lossy().to_string(),
            logs_dir: root.join("logs").to_string_lossy().to_string(),
            flush_interval: 1,
        },
        relay: Relay {
            url: String::new(),
            database_path: String::new(),
        },
        listings: Listings {
            country_shard_size: 0,
            profile_shard_size: 0,
        },
    }
}

fn write_ids<T: WriteEventIndexes>(
    indexes: &T,
    settings: &Settings,
    kind: IndexerEventKind,
) -> Vec<String> {
    let mut updated = Vec::new();
    indexes.write(settings, &mut updated).expect("write indexes");
    let path = kind
        .base_path(&settings.indexer.data_dir)
        .expect("base path")
        .join("events.json");
    let raw = std::fs::read_to_string(path).expect("read events.json");
    serde_json::from_str(&raw).expect("parse events.json")
}

fn expected_ids(events: &[RelayIndexerEvent]) -> Vec<String> {
    let mut refs: Vec<&RelayIndexerEvent> = events.iter().collect();
    refs.sort_unstable_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.cmp(&b.id)));
    refs.into_iter().map(|e| e.id.clone()).collect()
}

fn assert_deterministic<T, F>(
    kind: IndexerEventKind,
    events: Vec<RelayIndexerEvent>,
    build: F,
) where
    T: WriteEventIndexes,
    F: Fn(&[RelayIndexerEvent]) -> T,
{
    let mut reversed = events.clone();
    reversed.reverse();
    let expected = expected_ids(&events);

    let dir_a = tempdir().expect("tempdir");
    let settings_a = settings_for(dir_a.path());
    let dir_b = tempdir().expect("tempdir");
    let settings_b = settings_for(dir_b.path());

    let indexes_a = build(&events);
    let indexes_b = build(&reversed);

    let ids_a = write_ids(&indexes_a, &settings_a, kind);
    let ids_b = write_ids(&indexes_b, &settings_b, kind);

    assert_eq!(ids_a, expected);
    assert_eq!(ids_b, expected);
}

fn make_event(
    id: &str,
    author: &str,
    created_at: u32,
    kind: IndexerEventKind,
    tags: Vec<Vec<String>>,
    content: &str,
) -> RelayIndexerEvent {
    RelayIndexerEvent {
        id: id.to_string(),
        author: author.to_string(),
        created_at,
        pubkey: author.to_string(),
        kind,
        tags,
        content: content.to_string(),
        hash: id.to_string(),
        sig: "sig".to_string(),
    }
}

fn profile_event(id: &str, author: &str, created_at: u32, nip05: &str) -> RelayIndexerEvent {
    let content = serde_json::json!({"name": "user", "nip05": nip05}).to_string();
    make_event(
        id,
        author,
        created_at,
        IndexerEventKind::Profile,
        Vec::new(),
        &content,
    )
}

fn listing_tags(d_tag: &str) -> Vec<Vec<String>> {
    vec![
        vec!["d".to_string(), d_tag.to_string()],
        vec!["key".to_string(), "key".to_string()],
        vec!["title".to_string(), "title".to_string()],
        vec!["category".to_string(), "category".to_string()],
    ]
}

fn comment_tags(root_id: &str, root_author: &str) -> Vec<Vec<String>> {
    vec![
        vec!["E".to_string(), root_id.to_string()],
        vec!["K".to_string(), "1".to_string()],
        vec!["P".to_string(), root_author.to_string()],
    ]
}

fn reaction_tags(root_id: &str, root_author: &str) -> Vec<Vec<String>> {
    vec![
        vec!["e".to_string(), root_id.to_string()],
        vec!["k".to_string(), "1".to_string()],
        vec!["p".to_string(), root_author.to_string()],
    ]
}

#[test]
fn events_json_deterministic_profile() {
    let author = "a".repeat(64);
    let events = vec![
        profile_event("1".repeat(64).as_str(), &author, 10, "a@radroots.market"),
        profile_event("2".repeat(64).as_str(), &author, 20, "b@radroots.market"),
    ];
    assert_deterministic(IndexerEventKind::Profile, events, |raw| {
        EventProfileIndexes::build(raw).expect("build profile")
    });
}

#[test]
fn events_json_deterministic_listing() {
    let author = "b".repeat(64);
    let events = vec![
        make_event(
            "1".repeat(64).as_str(),
            &author,
            10,
            IndexerEventKind::Listing,
            listing_tags("d1"),
            "",
        ),
        make_event(
            "2".repeat(64).as_str(),
            &author,
            20,
            IndexerEventKind::Listing,
            listing_tags("d2"),
            "",
        ),
    ];
    let profiles = ProfileResolver::default();
    assert_deterministic(IndexerEventKind::Listing, events, |raw| {
        EventListingIndexes::build_with_profiles(raw, &profiles).expect("build listing")
    });
}

#[test]
fn events_json_deterministic_comment() {
    let author = "c".repeat(64);
    let root_author = "d".repeat(64);
    let events = vec![
        make_event(
            "1".repeat(64).as_str(),
            &author,
            10,
            IndexerEventKind::Comment,
            comment_tags("root1", &root_author),
            "hello",
        ),
        make_event(
            "2".repeat(64).as_str(),
            &author,
            20,
            IndexerEventKind::Comment,
            comment_tags("root2", &root_author),
            "hi",
        ),
    ];
    let profiles = ProfileResolver::default();
    assert_deterministic(IndexerEventKind::Comment, events, |raw| {
        EventCommentIndexes::build_with_profiles(raw, &profiles).expect("build comment")
    });
}

#[test]
fn events_json_deterministic_reaction() {
    let author = "e".repeat(64);
    let root_author = "f".repeat(64);
    let events = vec![
        make_event(
            "1".repeat(64).as_str(),
            &author,
            10,
            IndexerEventKind::Reaction,
            reaction_tags("root1", &root_author),
            "+",
        ),
        make_event(
            "2".repeat(64).as_str(),
            &author,
            20,
            IndexerEventKind::Reaction,
            reaction_tags("root2", &root_author),
            "+",
        ),
    ];
    let profiles = ProfileResolver::default();
    assert_deterministic(IndexerEventKind::Reaction, events, |raw| {
        EventReactionIndexes::build_with_profiles(raw, &profiles).expect("build reaction")
    });
}

#[test]
fn events_json_deterministic_post() {
    let author = "1".repeat(64);
    let events = vec![
        make_event(
            "a".repeat(64).as_str(),
            &author,
            10,
            IndexerEventKind::Post,
            Vec::new(),
            "hello",
        ),
        make_event(
            "b".repeat(64).as_str(),
            &author,
            20,
            IndexerEventKind::Post,
            Vec::new(),
            "hi",
        ),
    ];
    let profiles = ProfileResolver::default();
    assert_deterministic(IndexerEventKind::Post, events, |raw| {
        EventPostIndexes::build_with_profiles(raw, &profiles).expect("build post")
    });
}

#[test]
fn events_json_deterministic_follow() {
    let author = "2".repeat(64);
    let follow = "3".repeat(64);
    let events = vec![
        make_event(
            "a".repeat(64).as_str(),
            &author,
            10,
            IndexerEventKind::Follow,
            vec![vec!["p".to_string(), follow.clone()]],
            "",
        ),
        make_event(
            "b".repeat(64).as_str(),
            &author,
            20,
            IndexerEventKind::Follow,
            vec![vec!["p".to_string(), follow]],
            "",
        ),
    ];
    let profiles = ProfileResolver::default();
    assert_deterministic(IndexerEventKind::Follow, events, |raw| {
        EventFollowIndexes::build_with_profiles(raw, &profiles).expect("build follow")
    });
}

#[test]
fn events_json_deterministic_job_request() {
    let author = "4".repeat(64);
    let events = vec![
        make_event(
            "a".repeat(64).as_str(),
            &author,
            10,
            IndexerEventKind::JobRequest(KIND_JOB_REQUEST_MIN),
            Vec::new(),
            "",
        ),
        make_event(
            "b".repeat(64).as_str(),
            &author,
            20,
            IndexerEventKind::JobRequest(KIND_JOB_REQUEST_MIN),
            Vec::new(),
            "",
        ),
    ];
    let profiles = ProfileResolver::default();
    assert_deterministic(
        IndexerEventKind::JobRequest(KIND_JOB_REQUEST_MIN),
        events,
        |raw| EventJobRequestIndexes::build_with_profiles(raw, &profiles).expect("build job request"),
    );
}

#[test]
fn events_json_deterministic_job_result() {
    let author = "5".repeat(64);
    let events = vec![
        make_event(
            "a".repeat(64).as_str(),
            &author,
            10,
            IndexerEventKind::JobResult(KIND_JOB_RESULT_MIN),
            vec![vec!["e".to_string(), "req1".to_string()]],
            "",
        ),
        make_event(
            "b".repeat(64).as_str(),
            &author,
            20,
            IndexerEventKind::JobResult(KIND_JOB_RESULT_MIN),
            vec![vec!["e".to_string(), "req2".to_string()]],
            "",
        ),
    ];
    let profiles = ProfileResolver::default();
    assert_deterministic(
        IndexerEventKind::JobResult(KIND_JOB_RESULT_MIN),
        events,
        |raw| EventJobResultIndexes::build_with_profiles(raw, &profiles).expect("build job result"),
    );
}

#[test]
fn events_json_deterministic_job_feedback() {
    let author = "6".repeat(64);
    let events = vec![
        make_event(
            "a".repeat(64).as_str(),
            &author,
            10,
            IndexerEventKind::JobFeedback,
            vec![
                vec!["e".to_string(), "req1".to_string()],
                vec!["status".to_string(), "success".to_string()],
            ],
            "",
        ),
        make_event(
            "b".repeat(64).as_str(),
            &author,
            20,
            IndexerEventKind::JobFeedback,
            vec![
                vec!["e".to_string(), "req2".to_string()],
                vec!["status".to_string(), "success".to_string()],
            ],
            "",
        ),
    ];
    let profiles = ProfileResolver::default();
    assert_deterministic(IndexerEventKind::JobFeedback, events, |raw| {
        EventJobFeedbackIndexes::build_with_profiles(raw, &profiles).expect("build job feedback")
    });
}

#[test]
fn events_json_tiebreaks_by_id() {
    let author = "7".repeat(64);
    let events = vec![
        make_event(
            "f".repeat(64).as_str(),
            &author,
            10,
            IndexerEventKind::Post,
            Vec::new(),
            "hello",
        ),
        make_event(
            "0".repeat(64).as_str(),
            &author,
            10,
            IndexerEventKind::Post,
            Vec::new(),
            "world",
        ),
    ];
    let profiles = ProfileResolver::default();
    let dir = tempdir().expect("tempdir");
    let settings = settings_for(dir.path());
    let indexes = EventPostIndexes::build_with_profiles(&events, &profiles).expect("build post");
    let ids = write_ids(&indexes, &settings, IndexerEventKind::Post);
    assert_eq!(ids[0], "0".repeat(64));
}
