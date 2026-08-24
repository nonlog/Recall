use crate::session_delete::{DeleteMode, DeleteResult};
use crate::types::Session;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeleteOrigin {
    Search,
    Viewing,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingDelete {
    pub(crate) session: Session,
    pub(crate) origin: DeleteOrigin,
    pub(crate) mode: DeleteMode,
}

#[derive(Debug)]
pub(crate) struct DeleteWorkerResponse {
    pub(crate) session_id: String,
    pub(crate) origin: DeleteOrigin,
    pub(crate) result: Result<DeleteResult, String>,
}
