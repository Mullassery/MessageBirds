mod audiences;
mod channels;
mod dead_letter;
mod destinations;
mod events;
mod governance;
mod health;
mod journeys;
mod mixins;
mod namespaces;
mod profiles;
mod schemas;
mod templates;

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
            "/profiles/:id/journeys",
            get(journeys::get_profile_journeys),
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
            "/audiences/:id/activate-via-reverse-etl",
            post(destinations::activate_audience_via_reverse_etl),
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
        .route(
            "/channels",
            get(channels::list_channels).post(channels::create_channel),
        )
        .route(
            "/templates",
            get(templates::list_templates).post(templates::create_template),
        )
        .route(
            "/journeys",
            get(journeys::list_journeys).post(journeys::create_journey),
        )
        .route("/journeys/:id", get(journeys::get_journey))
        .route("/journeys/:id/runs", get(journeys::get_journey_runs))
        .route("/journeys/:id/start", post(journeys::start_journey))
        .route("/journey-runs/:id/events", get(journeys::get_run_events))
        .route(
            "/contact-policies",
            get(journeys::list_contact_policies).post(journeys::create_contact_policy),
        )
        .with_state(state)
}
