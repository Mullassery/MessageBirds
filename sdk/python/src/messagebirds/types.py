"""
Typed dict mirrors of the MessageBirds API's JSON shapes.

Kept in lockstep with `sdk/js/src/types.ts`, which is itself kept in
lockstep with the Rust types it names in each docstring. Python is
dynamically typed, so nothing here is enforced at runtime — these exist
for editor/mypy support when building your own requests against
`MessageBirdsClient.request`, the same role `apiFetch<T>` plays for
`ui/` against the TS types.
"""

from __future__ import annotations

from typing import Any, Literal, TypedDict


class IdentityRef(TypedDict, total=False):
    """Mirrors `mb_core::IdentityRef`."""

    namespace: str
    value: str
    primary: bool
    source: str
    confidence: float


class EventSource(TypedDict):
    """Mirrors `mb_core::EventSource`."""

    type: str
    name: str


class SchemaRef(TypedDict):
    """Mirrors `mb_core::SchemaRef`."""

    name: str
    version: str


class EventEnvelope(TypedDict, total=False):
    """
    Mirrors `mb_core::EventEnvelope`. `context["profile_updates"]`, if
    present, is a map of mixin key (`"core/person@1.0"`) to the field
    values this event contributes to that mixin — see docs/OCDS.md.
    """

    event_id: str
    event_type: str
    timestamp: str
    source: EventSource
    identity: list[IdentityRef]
    context: dict[str, Any]
    data: dict[str, Any]
    schema: SchemaRef
    consent: Any
    governance: Any
    correlation_id: str
    trace_id: str
    tenant_id: str


class FieldProvenance(TypedDict):
    """Mirrors `mb_profile::FieldProvenance`."""

    profile_id: str
    mixin_key: str
    field_path: str
    value: Any
    source: str
    event_id: str
    applied_policy: str
    updated_at: str


class ProfileView(TypedDict):
    """Response shape of `GET /profiles/{id}` and `GET /profiles/by-identity`."""

    id: str
    tenant_id: str
    mixins: dict[str, dict[str, Any]]
    created_at: str
    updated_at: str
    provenance: list[FieldProvenance]


class IdentityNode(TypedDict):
    """Mirrors `mb_identity::IdentityNode`."""

    id: str
    tenant_id: str
    namespace: str
    value_hash: str
    profile_id: str
    confidence: float
    source: str
    first_seen_at: str
    last_seen_at: str


IdentityAuditKind = Literal["linked", "merged", "split"]


class IdentityAuditEntry(TypedDict):
    """
    Mirrors `mb_identity::IdentityAuditEntry`. `namespace`/`value_hash`
    are `None` for `"merged"` entries, which record a whole-profile
    merge rather than one identity claim.
    """

    id: str
    tenant_id: str
    kind: IdentityAuditKind
    namespace: str | None
    value_hash: str | None
    profile_id: str
    previous_profile_id: str | None
    source: str
    created_at: str


class IdentityView(TypedDict):
    """Response shape of `GET /profiles/{id}/identity`."""

    nodes: list[IdentityNode]
    audit: list[IdentityAuditEntry]


class MergeSuggestion(TypedDict):
    """
    One entry in `GET /profiles/{id}/merge-suggestions` — a
    confidence-scored candidate for a manual merge. Never applied
    automatically.
    """

    profile_id: str
    score: float


class EventSummary(TypedDict):
    """One row of `GET /profiles/{id}/events`."""

    id: str
    event_type: str
    schema_name: str
    schema_version: str
    identity: list[IdentityRef]
    data: dict[str, Any]
    received_at: str
    occurred_at: str


class EventDetail(TypedDict):
    """Response of `GET /events/{id}` — a single event, in full."""

    id: str
    tenant_id: str
    event_type: str
    schema_name: str
    schema_version: str
    identity: list[IdentityRef]
    context: dict[str, Any]
    data: dict[str, Any]
    profile_id: str
    received_at: str
    occurred_at: str


FieldType = Literal["string", "number", "boolean", "object", "array"]


class FieldDef(TypedDict, total=False):
    """Mirrors `mb_schema_registry::FieldDef`."""

    type: FieldType
    required: bool
    enum: list[Any] | None
    labels: list[str]


SchemaStatus = Literal["active", "deprecated"]


class SchemaDefinition(TypedDict):
    """Mirrors `mb_schema_registry::Schema` — response of `GET /schemas/{name}/{version}`."""

    id: str
    name: str
    version: str
    fields: dict[str, FieldDef]
    status: SchemaStatus
    created_at: str


MixinStatus = Literal["active", "deprecated"]


class MixinDef(TypedDict):
    """Mirrors `mb_mixins::MixinDef` — response of `GET /mixins/{namespace}/{name}/{version}`."""

    id: str
    namespace: str
    name: str
    version: str
    fields: dict[str, FieldDef]
    status: MixinStatus
    created_at: str


AttributeOp = Literal[
    "equals", "not_equals", "exists", "not_exists", "greater_than", "less_than", "contains"
]

# Mirrors `mb_audiences::Condition`. Externally tagged on the wire
# (`{"attribute": {...}}`, not `{"type": "attribute", ...}`) — plain
# dicts, not a TypedDict union, since Python has no discriminated-union
# construct that maps cleanly onto externally-tagged serde output.
Condition = dict[str, Any]

AudienceStatus = Literal["active", "archived"]


class AudienceDefinition(TypedDict):
    """Mirrors `mb_audiences::AudienceDefinition`."""

    id: str
    tenant_id: str
    name: str
    version: int
    conditions: Condition
    status: AudienceStatus
    created_at: str


MembershipKind = Literal["entered", "exited"]


class Membership(TypedDict):
    """Mirrors `mb_audiences::Membership` — current membership state."""

    audience_id: str
    profile_id: str
    entered_at: str
    exited_at: str | None


class Destination(TypedDict):
    """Mirrors `mb_connectors::Destination`."""

    id: str
    tenant_id: str
    kind: str
    name: str
    config: dict[str, Any]
    supported_actions: list[str]
    created_at: str


# Mirrors `mb_governance::Reason` — why an activation was denied.
# Internally tagged on `"kind"` (safe here since `Reason` isn't
# recursive) — again a plain dict rather than a TypedDict union.
Reason = dict[str, Any]


class BlockedProfile(TypedDict):
    """One profile skipped during activation, and why."""

    profile_id: str
    reasons: list[Reason]


class ActivationSummary(TypedDict):
    """Response of `POST /audiences/{id}/activate`."""

    sent: int
    failed: int
    blocked: list[BlockedProfile]


class DeadLetterEventView(TypedDict):
    """One row of `GET /dead-letter-events`."""

    id: str
    event_id: str
    schema_name: str
    schema_version: str
    source: str
    reason: str
    raw_payload: Any
    created_at: str


Effect = Literal["allow", "deny"]


class Policy(TypedDict):
    """Mirrors `mb_governance::Policy`."""

    id: str
    tenant_id: str
    label: str
    action: str
    effect: Effect
    priority: int
    created_at: str


class ConsentEvent(TypedDict):
    """Mirrors `mb_governance::ConsentEvent` — one append-only consent record."""

    id: str
    tenant_id: str
    profile_id: str
    purpose: str
    granted: bool
    source: str
    jurisdiction: str | None
    consent_version: str | None
    expires_at: str | None
    created_at: str


class ConsentState(TypedDict):
    """Mirrors `mb_governance::ConsentState` — derived current state for one purpose."""

    purpose: str
    granted: bool
    source: str
    updated_at: str


class ConsentView(TypedDict):
    """Response of `GET /profiles/{id}/consent`."""

    current: list[ConsentState]
    history: list[ConsentEvent]


class ConsentSummary(TypedDict):
    granted: int
    missing: int
    total: int


class PolicySimulateResponse(TypedDict):
    """Response of `POST /policy-simulate` (Section 49)."""

    allowed_fields: list[str]
    blocked_fields: list[str]
    applicable_policies: list[Reason]
    consent_summary: ConsentSummary | None
    final_decision: Literal["allow", "deny", "partial"]


class Channel(TypedDict):
    """Mirrors `mb_channels::Channel`. Only `kind: "webhook"` has a real transport."""

    id: str
    tenant_id: str
    kind: str
    name: str
    config: Any
    created_at: str


class MessageTemplate(TypedDict):
    """Mirrors `mb_templates::MessageTemplate`."""

    id: str
    tenant_id: str
    channel_kind: str
    name: str
    version: int
    subject: str | None
    body: str
    created_at: str


class RenderedTemplate(TypedDict):
    """Mirrors `mb_templates::RenderedTemplate`."""

    subject: str | None
    body: str
    missing_variables: list[str]


# Mirrors `mb_journeys::Trigger`. Internally tagged on `"kind"`, like `NodeKind`.
Trigger = dict[str, Any]


class SplitBranch(TypedDict):
    """Mirrors `mb_journeys::SplitBranch`."""

    next: str
    weight: int


# Mirrors `mb_journeys::NodeKind` flattened with `id` into `mb_journeys::Node`
# — internally tagged on `"kind"` (not recursive, unlike `Condition`).
Node = dict[str, Any]

JourneyStatus = Literal["active", "archived"]


class JourneyDefinition(TypedDict):
    """Mirrors `mb_journeys::JourneyDefinition`. Graph is a flat node list, not nested."""

    id: str
    tenant_id: str
    name: str
    version: int
    trigger: Trigger
    nodes: list[Node]
    entry_node: str
    status: JourneyStatus
    created_at: str


RunStatus = Literal["running", "waiting", "completed", "failed"]


class JourneyRun(TypedDict):
    """Mirrors `mb_journeys::JourneyRun`."""

    id: str
    tenant_id: str
    journey_id: str
    profile_id: str
    current_node: str
    status: RunStatus
    wake_at: str | None
    started_at: str
    updated_at: str


RunEventKind = Literal[
    "entered",
    "waited",
    "branched",
    "action_sent",
    "action_suppressed",
    "action_failed",
    "completed",
]


class JourneyRunEvent(TypedDict):
    """Mirrors `mb_journeys::JourneyRunEvent` — one append-only step log entry."""

    id: str
    run_id: str
    node_id: str
    kind: RunEventKind
    detail: str | None
    created_at: str


class ContactPolicy(TypedDict):
    """Mirrors `mb_journeys::ContactPolicy`. `channel_id: None` applies across all channels."""

    id: str
    tenant_id: str
    max_messages: int
    window_days: int
    channel_id: str | None
    created_at: str
