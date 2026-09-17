use chrono::{DateTime, Utc};
use mb_audiences::Condition;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What starts a run. `AudienceEntered` is wired from
/// `evaluate_and_sync_membership`'s `Entered` changes in the worker
/// pipeline; `Event` would need its own wiring in the ingestion path
/// (not implemented this phase — only the audience trigger is live).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Trigger {
    AudienceEntered { audience_id: Uuid },
    Event { event_type: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitBranch {
    pub next: String,
    pub weight: u32,
}

/// One journey step. Not recursive — nodes reference each other by string
/// `id`, a flat graph rather than a nested tree, which sidesteps the exact
/// serde `Content`-buffering compile failure `Condition` hit in Phase 3
/// (see `mb-audiences`) by construction. `Condition` reuses
/// `mb_audiences::Condition` directly — a journey can branch on the same
/// attribute/event logic an audience is built from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NodeKind {
    Wait {
        duration_seconds: i64,
        next: String,
    },
    Condition {
        condition: Condition,
        if_true: String,
        if_false: String,
    },
    Action {
        channel_id: Uuid,
        template_id: Uuid,
        next: String,
    },
    /// Deterministic weighted branching (hash of profile id + node id, so
    /// a given profile always lands in the same branch) — the only
    /// experimentation primitive this phase. No statistical-significance
    /// tracking or bandits (Section 30; real additional feature, not
    /// implemented).
    Split {
        branches: Vec<SplitBranch>,
    },
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(flatten)]
    pub kind: NodeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum JourneyStatus {
    Active,
    Archived,
}

/// A versioned journey graph. Authored as JSON (this `nodes` list), not a
/// drag-and-drop canvas — matching Section 26's own principle that the
/// canvas must never be the source of truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyDefinition {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub version: i32,
    pub trigger: Trigger,
    pub nodes: Vec<Node>,
    pub entry_node: String,
    pub status: JourneyStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct JourneyRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub version: i32,
    pub trigger: serde_json::Value,
    pub nodes: serde_json::Value,
    pub entry_node: String,
    pub status: JourneyStatus,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<JourneyRow> for JourneyDefinition {
    type Error = serde_json::Error;

    fn try_from(row: JourneyRow) -> Result<Self, Self::Error> {
        Ok(JourneyDefinition {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            version: row.version,
            trigger: serde_json::from_value(row.trigger)?,
            nodes: serde_json::from_value(row.nodes)?,
            entry_node: row.entry_node,
            status: row.status,
            created_at: row.created_at,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Running,
    Waiting,
    Completed,
    Failed,
}

/// Durable execution state for one profile's pass through a journey.
/// Persisted after every node transition — a crash mid-run resumes from
/// exactly `current_node`/`wake_at`, not from the start and not lost.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct JourneyRun {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub journey_id: Uuid,
    pub profile_id: Uuid,
    pub current_node: String,
    pub status: RunStatus,
    pub wake_at: Option<DateTime<Utc>>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RunEventKind {
    Entered,
    Waited,
    Branched,
    ActionSent,
    ActionSuppressed,
    ActionFailed,
    Completed,
}

/// Append-only step log — the "why did this happen" trail for a run.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct JourneyRunEvent {
    pub id: Uuid,
    pub run_id: Uuid,
    pub node_id: String,
    pub kind: RunEventKind,
    pub detail: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// `(tenant, max_messages in window_days[, channel_id])` — a send that
/// would exceed this is suppressed (the run still advances), not the
/// whole run blocked. `channel_id = None` applies across every channel.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContactPolicy {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub max_messages: i32,
    pub window_days: i32,
    pub channel_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    Sent,
    Failed,
    Suppressed,
}

/// One `Action` node's send attempt. `UNIQUE(run_id, node_id)` at the
/// database level makes a retry after a crash idempotent — see
/// `engine::execute_action`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageSent {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub run_id: Uuid,
    pub node_id: String,
    pub channel_id: Uuid,
    pub profile_id: Uuid,
    pub subject: Option<String>,
    pub body: String,
    pub status: MessageStatus,
    pub detail: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StepOutcome {
    Progressed,
    Waiting,
    Completed,
}
