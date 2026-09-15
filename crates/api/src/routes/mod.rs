mod audiences;
mod dead_letter;
mod destinations;
mod events;
mod governance;
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
        .route("/events/:id", get(events::get_event))
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
            "/profiles/:id/audiences",
            get(profiles::get_profile_audiences),
        )
        .route(
            "/profiles/:id/consent",
            get(governance::get_profile_consent),
        )
        .route(
            "/schemas",
            get(schemas::list_schemas).post(schemas::create_schema),
        )
        .route("/schemas/:name/:version", get(schemas::get_schema))
        .route(
            "/mixins",
            get(mixins::list_mixins).post(mixins::create_mixin),
        )
        .route("/mixins/:namespace/:name/:version", get(mixins::get_mixin))
        .route("/namespaces", get(namespaces::list_namespaces))
        .route(
            "/audiences",
            get(audiences::list_audiences).post(audiences::create_audience),
        )
        .route("/audiences/:id", get(audiences::get_audience))
        .route(
            "/audiences/:id/members",
            get(audiences::get_audience_members),
        )
        .route(
            "/audiences/:id/activate",
            post(destinations::activate_audience),
        )
        .route(
            "/destinations",
            get(destinations::list_destinations).post(destinations::create_destination),
        )
        .route(
            "/dead-letter-events",
            get(dead_letter::list_dead_letter_events),
        )
        .route(
            "/policies",
            get(governance::list_policies).post(governance::create_policy),
        )
        .route("/consent", post(governance::record_consent))
        .route("/policy-simulate", post(governance::policy_simulate))
        .with_state(state)
}
