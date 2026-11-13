use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::db::search::{SearchEngine, SearchFilters};
use crate::db::store::Store;
use crate::embedding::EmbeddingProvider;
use crate::types::{MatchSource, SearchResult};

const SEARCH_LIMIT: usize = 200;
const FETCH_MULTIPLIER: usize = 3;
// shortcut: cap contextual descendants at 1,000, use paged child loading for larger trees.
const CHILD_LIMIT: usize = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchPhase {
    Text,
    Hybrid,
}

pub(crate) struct SearchRequest {
    pub(crate) id: u64,
    pub(crate) query: String,
    pub(crate) filters: SearchFilters,
    pub(crate) semantic_ready: bool,
}

pub(crate) struct SearchResponse {
    pub(crate) id: u64,
    pub(crate) query: String,
    pub(crate) phase: SearchPhase,
    pub(crate) result: Result<Vec<SearchResult>, String>,
}

pub(crate) struct SearchWorker {
    request_tx: Sender<SearchRequest>,
    response_rx: Receiver<SearchResponse>,
}

impl SearchWorker {
    pub(crate) fn spawn() -> Self {
        let (request_tx, request_rx) = mpsc::channel();
        let (response_tx, response_rx) = mpsc::channel();

        thread::spawn(move || run_worker(request_rx, response_tx));

        Self { request_tx, response_rx }
    }

    pub(crate) fn search(&self, request: SearchRequest) -> bool {
        self.request_tx.send(request).is_ok()
    }

    pub(crate) fn try_recv(&self) -> Option<SearchResponse> {
        self.response_rx.try_recv().ok()
    }
}

fn run_worker(request_rx: Receiver<SearchRequest>, response_tx: Sender<SearchResponse>) {
    crate::db::schema::register_sqlite_vec();

    let store = match Store::open() {
        Ok(store) => store,
        Err(err) => {
            while let Ok(request) = request_rx.recv() {
                send_error(
                    &response_tx,
                    request,
                    SearchPhase::Text,
                    format!("Search error: {err}"),
                );
            }
            return;
        }
    };

    let engine = SearchEngine::new(&store.conn);
    let mut provider = None;
    let mut embedding_unavailable = false;

    while let Ok(mut request) = request_rx.recv() {
        while let Ok(next) = request_rx.try_recv() {
            request = next;
        }

        run_request(
            &store,
            &engine,
            &mut provider,
            &mut embedding_unavailable,
            request,
            &response_tx,
        );
    }
}

fn run_request(
    store: &Store,
    engine: &SearchEngine,
    provider: &mut Option<EmbeddingProvider>,
    embedding_unavailable: &mut bool,
    request: SearchRequest,
    response_tx: &Sender<SearchResponse>,
) {
    if request.query.trim().is_empty() {
        send_result(response_tx, &request, SearchPhase::Text, recent_sessions(store, &request));
        return;
    }

    let text_result = engine
        .hybrid_search(&request.query, None, &request.filters, SEARCH_LIMIT, FETCH_MULTIPLIER)
        .and_then(|results| include_children(store, results));
    let text_ok = text_result.is_ok();
    send_result(response_tx, &request, SearchPhase::Text, text_result);

    if !text_ok || !request.semantic_ready || *embedding_unavailable {
        return;
    }

    if provider.is_none() {
        match EmbeddingProvider::new(false) {
            Ok(next_provider) => {
                *provider = Some(next_provider);
            }
            Err(err) => {
                *embedding_unavailable = true;
                send_error(
                    response_tx,
                    request,
                    SearchPhase::Hybrid,
                    format!("Semantic unavailable - using text search only: {err}"),
                );
                return;
            }
        }
    }

    let embedding = provider
        .as_ref()
        .and_then(|p| p.embed_query(&[request.query.as_str()]).ok())
        .and_then(|mut e| if e.is_empty() { None } else { Some(e.swap_remove(0)) });

    let Some(embedding) = embedding else {
        send_error(
            response_tx,
            request,
            SearchPhase::Hybrid,
            "Semantic unavailable - using text search only".to_string(),
        );
        return;
    };

    let hybrid_result = engine
        .hybrid_search(
            &request.query,
            Some(embedding.as_slice()),
            &request.filters,
            SEARCH_LIMIT,
            FETCH_MULTIPLIER,
        )
        .and_then(|results| include_children(store, results));
    send_result(response_tx, &request, SearchPhase::Hybrid, hybrid_result);
}

fn recent_sessions(store: &Store, request: &SearchRequest) -> anyhow::Result<Vec<SearchResult>> {
    let recent = store.list_recent_sessions_for_search_scope(
        request.filters.sources.as_deref(),
        request.filters.time_range,
        &request.filters.scope,
        None,
        SEARCH_LIMIT,
    )?;
    let results = recent
        .into_iter()
        .map(|session| SearchResult { session, match_source: MatchSource::Fts, snippet: None })
        .collect();
    include_children(store, results)
}

fn include_children(
    store: &Store,
    mut results: Vec<SearchResult>,
) -> anyhow::Result<Vec<SearchResult>> {
    let max_results = results.len().saturating_add(CHILD_LIMIT);
    let mut ids: std::collections::HashSet<_> =
        results.iter().map(|result| result.session.id.clone()).collect();
    let mut index = 0;
    while index < results.len() && results.len() < max_results {
        for session in store.child_subagents_limited(&results[index].session.id, max_results)? {
            if results.len() == max_results {
                break;
            }
            if ids.insert(session.id.clone()) {
                results.push(SearchResult {
                    session,
                    match_source: MatchSource::Fts,
                    snippet: None,
                });
            }
        }
        index += 1;
    }
    Ok(results)
}

fn send_result(
    response_tx: &Sender<SearchResponse>,
    request: &SearchRequest,
    phase: SearchPhase,
    result: anyhow::Result<Vec<SearchResult>>,
) {
    let _ = response_tx.send(SearchResponse {
        id: request.id,
        query: request.query.clone(),
        phase,
        result: result.map_err(|err| format!("Search error: {err}")),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::search::TimeRange;
    use crate::project_scope::ProjectScope;
    use crate::types::Session;

    #[test]
    fn text_response_includes_unmatched_child_as_context() {
        use crate::types::{Message, Role};
        crate::db::schema::register_sqlite_vec();
        let store = Store::open_in_memory().unwrap();
        let session = |id: &str| Session {
            id: id.into(),
            source: "codex".into(),
            source_id: id.into(),
            title: id.into(),
            directory: None,
            repo_remote: None,
            repo_slug: None,
            repo_name: None,
            started_at: 0,
            updated_at: None,
            message_count: 1,
            entrypoint: None,
            custom_title: None,
            summary: None,
            duration_minutes: None,
            source_file_path: None,
            is_import: false,
            locations: Vec::new(),
            alternative_versions: 0,
        };
        for id in ["parent", "child"] {
            store.insert_session(&session(id)).unwrap();
        }
        store.conn.execute("INSERT INTO session_parent_links (session_id, relation, parent_source, parent_source_id) VALUES ('child', 'spawn', 'codex', 'parent')", []).unwrap();
        store
            .insert_messages(&[
                Message {
                    session_id: "parent".into(),
                    role: Role::User,
                    content: "uniquewombat".into(),
                    timestamp: None,
                    seq: 0,
                },
                Message {
                    session_id: "child".into(),
                    role: Role::User,
                    content: "other content".into(),
                    timestamp: None,
                    seq: 0,
                },
            ])
            .unwrap();
        let request = SearchRequest {
            id: 2,
            query: "uniquewombat".into(),
            semantic_ready: false,
            filters: SearchFilters {
                sources: Some(vec!["codex".into()]),
                time_range: TimeRange::All,
                scope: ProjectScope::Global,
                thread_role: None,
                excluded_session_id: None,
            },
        };
        let engine = SearchEngine::new(&store.conn);
        let matched = engine
            .hybrid_search(&request.query, None, &request.filters, SEARCH_LIMIT, FETCH_MULTIPLIER)
            .unwrap();
        assert_eq!(matched.len(), 1);
        let snippet = matched[0].snippet.clone();
        let (tx, rx) = mpsc::channel();
        run_request(&store, &engine, &mut None, &mut false, request, &tx);
        let response = rx.recv().unwrap();
        assert_eq!(response.id, 2);
        let results = response.result.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].session.id, "parent");
        assert_eq!(results[0].snippet, snippet);
        assert_eq!(results[1].session.id, "child");
        assert!(results[1].snippet.is_none());
    }

    #[test]
    fn recent_response_includes_hidden_spawn_child() {
        crate::db::schema::register_sqlite_vec();
        let store = Store::open_in_memory().unwrap();
        let session = |id: &str| Session {
            id: id.into(),
            source: "codex".into(),
            source_id: id.into(),
            title: id.into(),
            directory: None,
            repo_remote: None,
            repo_slug: None,
            repo_name: None,
            started_at: 0,
            updated_at: None,
            message_count: 1,
            entrypoint: None,
            custom_title: None,
            summary: None,
            duration_minutes: None,
            source_file_path: None,
            is_import: false,
            locations: Vec::new(),
            alternative_versions: 0,
        };
        for id in ["parent", "child", "grandchild", "orphan"] {
            store.insert_session(&session(id)).unwrap();
        }
        store
            .conn
            .execute("UPDATE sessions SET thread_role = 'subagent' WHERE id != 'parent'", [])
            .unwrap();
        for (child, parent) in [("child", "parent"), ("grandchild", "child"), ("orphan", "missing")]
        {
            store.conn.execute(
                "INSERT INTO session_parent_links (session_id, relation, parent_source, parent_source_id) VALUES (?1, 'spawn', 'codex', ?2)",
                rusqlite::params![child, parent],
            ).unwrap();
        }
        let request = SearchRequest {
            id: 1,
            query: String::new(),
            semantic_ready: false,
            filters: SearchFilters {
                sources: None,
                time_range: TimeRange::All,
                scope: ProjectScope::Global,
                thread_role: None,
                excluded_session_id: None,
            },
        };
        let recent = store
            .list_recent_sessions_for_search_scope(
                None,
                TimeRange::All,
                &ProjectScope::Global,
                None,
                SEARCH_LIMIT,
            )
            .unwrap();
        assert!(recent.iter().any(|s| s.id == "orphan"));
        assert!(!recent.iter().any(|s| s.id == "child"));
        let (tx, rx) = mpsc::channel();
        run_request(&store, &SearchEngine::new(&store.conn), &mut None, &mut false, request, &tx);
        let response = rx.recv().unwrap();
        assert_eq!(response.id, 1);
        assert!(response.query.is_empty());
        let results = response.result.unwrap();
        assert_eq!(results.len(), 4);
        for id in ["parent", "child", "grandchild", "orphan"] {
            assert_eq!(results.iter().filter(|result| result.session.id == id).count(), 1);
        }
    }
}

fn send_error(
    response_tx: &Sender<SearchResponse>,
    request: SearchRequest,
    phase: SearchPhase,
    error: String,
) {
    let _ = response_tx.send(SearchResponse {
        id: request.id,
        query: request.query,
        phase,
        result: Err(error),
    });
}
