use crate::session_delete::{DeleteMode, DeleteResult};
use crate::types::Session;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeleteOrigin {
    Search,
    Viewing,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingDelete {
    pub(crate) sessions: Vec<Session>,
    pub(crate) origin: DeleteOrigin,
    pub(crate) mode: DeleteMode,
}

#[derive(Debug)]
pub(crate) struct DeleteSuccess {
    pub(crate) session_id: String,
    pub(crate) result: DeleteResult,
}

#[derive(Debug)]
pub(crate) struct DeleteFailure {
    pub(crate) session_id: String,
    pub(crate) message: String,
}

#[derive(Debug)]
pub(crate) struct DeleteWorkerResponse {
    pub(crate) origin: DeleteOrigin,
    pub(crate) successes: Vec<DeleteSuccess>,
    pub(crate) failures: Vec<DeleteFailure>,
}
