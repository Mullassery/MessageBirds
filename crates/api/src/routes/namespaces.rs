use axum::extract::State;
use axum::Json;

use mb_namespaces::{Namespace, NamespaceRepo};

use crate::error::ApiError;
use crate::state::AppState;

pub async fn list_namespaces(
    State(state): State<AppState>,
) -> Result<Json<Vec<Namespace>>, ApiError> {
    Ok(Json(state.namespaces.list().await?))
}
