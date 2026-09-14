mod events;
mod health;
mod mixins;
mod namespaces;
mod profiles;
mod schemas;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/events", post(events::post_event))
        .route(
            "/profiles/by-identity",
            get(profiles::get_profile_by_identity),
        )
        .route("/profiles/:id", get(profiles::get_profile))
        .route("/profiles/:id/events", get(profiles::get_profile_events))
        .route(
            "/profiles/:id/identity",
            get(profiles::get_profile_identity),
        )
        .route(
            "/profiles/:id/merge-suggestions",
            get(profiles::get_merge_suggestions),
        )
        .route("/profiles/:id/merge", post(profiles::merge_profile))
        .route("/profiles/:id/split", post(profiles::split_profile))
        .route(
            "/schemas",
            get(schemas::list_schemas).post(schemas::create_schema),
        )
        .route("/schemas/:name/:version", get(schemas::get_schema))
        .route(
            "/mixins",
            get(mixins::list_mixins).post(mixins::create_mixin),
        )
        .route("/namespaces", get(namespaces::list_namespaces))
        .with_state(state)
}
