use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::engine::Engine;
use crate::error::JourneyError;
use crate::model::{ContactPolicy, JourneyDefinition, JourneyRow, JourneyRun, JourneyRunEvent};

const JOURNEY_COLUMNS: &str =
    "id, tenant_id, name, version, trigger, nodes, entry_node, status, created_at";
const RUN_COLUMNS: &str =
    "id, tenant_id, journey_id, profile_id, current_node, status, wake_at, started_at, updated_at";

#[async_trait]
pub trait JourneyRepo: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn register(
        &self,
        tenant_id: Uuid,
        name: &str,
        trigger: crate::model::Trigger,
        nodes: Vec<crate::model::Node>,
        entry_node: &str,
    ) -> Result<JourneyDefinition, JourneyError>;

    async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<JourneyDefinition, JourneyError>;

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<JourneyDefinition>, JourneyError>;

    /// Active journeys whose trigger is `AudienceEntered { audience_id }`
    /// — what the worker pipeline looks up after a membership change.
    async fn find_by_audience_trigger(
        &self,
        tenant_id: Uuid,
        audience_id: Uuid,
    ) -> Result<Vec<JourneyDefinition>, JourneyError>;

    async fn get_runs(&self, journey_id: Uuid) -> Result<Vec<JourneyRun>, JourneyError>;

    async fn get_profile_runs(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
    ) -> Result<Vec<JourneyRun>, JourneyError>;

    async fn get_run_events(&self, run_id: Uuid) -> Result<Vec<JourneyRunEvent>, JourneyError>;

    /// Starts a run at the journey's `entry_node` and drives it forward
    /// immediately (through any synchronous `Condition`/`Split` nodes)
    /// until it hits a `Wait`, an `Action`, or `End`.
    async fn start_run(
        &self,
        journey_id: Uuid,
        profile_id: Uuid,
    ) -> Result<JourneyRun, JourneyError>;

    /// The poller's entry point: advances every run that's due (waiting
    /// with `wake_at <= now()`, or newly started and still `running`).
    async fn advance_due_runs(&self) -> Result<usize, JourneyError>;
}

#[async_trait]
pub trait ContactPolicyRepo: Send + Sync {
    // Named distinctly from `JourneyRepo::register`/`list` — both traits
    // are implemented by the same `PgJourneyRepo`, and same-named methods
    // across two in-scope traits on one type would be ambiguous to call.
    async fn register_policy(
        &self,
        tenant_id: Uuid,
        max_messages: i32,
        window_days: i32,
        channel_id: Option<Uuid>,
    ) -> Result<ContactPolicy, JourneyError>;

    async fn list_policies(&self, tenant_id: Uuid) -> Result<Vec<ContactPolicy>, JourneyError>;
}

pub struct PgJourneyRepo {
    pool: PgPool,
    engine: Engine,
}

impl PgJourneyRepo {
    pub fn new(pool: PgPool, engine: Engine) -> Self {
        Self { pool, engine }
    }

    async fn run_to_completion_or_pause(
        &self,
        mut run: JourneyRun,
        journey: &JourneyDefinition,
    ) -> Result<JourneyRun, JourneyError> {
        loop {
            let outcome = self.engine.advance_run(&mut run, journey).await?;
            if outcome != crate::model::StepOutcome::Progressed {
                break;
            }
        }
        Ok(run)
    }
}

#[async_trait]
impl JourneyRepo for PgJourneyRepo {
    async fn register(
        &self,
        tenant_id: Uuid,
        name: &str,
        trigger: crate::model::Trigger,
        nodes: Vec<crate::model::Node>,
        entry_node: &str,
    ) -> Result<JourneyDefinition, JourneyError> {
        let next_version: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM journeys WHERE tenant_id = $1 AND name = $2",
        )
        .bind(tenant_id)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        let trigger_json = serde_json::to_value(&trigger)?;
        let nodes_json = serde_json::to_value(&nodes)?;

        let row = sqlx::query_as::<_, JourneyRow>(&format!(
            r#"
            INSERT INTO journeys (id, tenant_id, name, version, trigger, nodes, entry_node, status, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'active', now())
            RETURNING {JOURNEY_COLUMNS}
            "#
        ))
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(name)
        .bind(next_version)
        .bind(trigger_json)
        .bind(nodes_json)
        .bind(entry_node)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_into()?)
    }

    async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<JourneyDefinition, JourneyError> {
        let row = sqlx::query_as::<_, JourneyRow>(&format!(
            "SELECT {JOURNEY_COLUMNS} FROM journeys WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(JourneyError::NotFound(id))?;
        Ok(row.try_into()?)
    }

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<JourneyDefinition>, JourneyError> {
        let rows = sqlx::query_as::<_, JourneyRow>(&format!(
            "SELECT {JOURNEY_COLUMNS} FROM journeys WHERE tenant_id = $1 ORDER BY name, created_at"
        ))
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(|r| Ok(r.try_into()?)).collect()
    }

    async fn find_by_audience_trigger(
        &self,
        tenant_id: Uuid,
        audience_id: Uuid,
    ) -> Result<Vec<JourneyDefinition>, JourneyError> {
        let rows = sqlx::query_as::<_, JourneyRow>(&format!(
            "SELECT {JOURNEY_COLUMNS} FROM journeys WHERE tenant_id = $1 AND status = 'active'"
        ))
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let mut matched = Vec::new();
        for row in rows {
            let journey: JourneyDefinition = row.try_into()?;
            if let crate::model::Trigger::AudienceEntered { audience_id: aid } = journey.trigger {
                if aid == audience_id {
                    matched.push(journey);
                }
            }
        }
        Ok(matched)
    }

    async fn get_runs(&self, journey_id: Uuid) -> Result<Vec<JourneyRun>, JourneyError> {
        let rows = sqlx::query_as::<_, JourneyRun>(&format!(
            "SELECT {RUN_COLUMNS} FROM journey_runs WHERE journey_id = $1 ORDER BY started_at DESC"
        ))
        .bind(journey_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_profile_runs(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
    ) -> Result<Vec<JourneyRun>, JourneyError> {
        let rows = sqlx::query_as::<_, JourneyRun>(&format!(
            "SELECT {RUN_COLUMNS} FROM journey_runs WHERE tenant_id = $1 AND profile_id = $2 ORDER BY started_at DESC"
        ))
        .bind(tenant_id)
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_run_events(&self, run_id: Uuid) -> Result<Vec<JourneyRunEvent>, JourneyError> {
        let rows = sqlx::query_as::<_, JourneyRunEvent>(
            "SELECT id, run_id, node_id, kind, detail, created_at FROM journey_run_events WHERE run_id = $1 ORDER BY created_at",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn start_run(
        &self,
        journey_id: Uuid,
        profile_id: Uuid,
    ) -> Result<JourneyRun, JourneyError> {
        // tenant_id is implied by the journey; fetch it without requiring
        // the caller to know it up front.
        let journey_row = sqlx::query_as::<_, JourneyRow>(&format!(
            "SELECT {JOURNEY_COLUMNS} FROM journeys WHERE id = $1"
        ))
        .bind(journey_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(JourneyError::NotFound(journey_id))?;
        let journey: JourneyDefinition = journey_row.try_into()?;

        let run = sqlx::query_as::<_, JourneyRun>(&format!(
            r#"
            INSERT INTO journey_runs (id, tenant_id, journey_id, profile_id, current_node, status, wake_at, started_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, 'running', NULL, now(), now())
            RETURNING {RUN_COLUMNS}
            "#
        ))
        .bind(Uuid::new_v4())
        .bind(journey.tenant_id)
        .bind(journey_id)
        .bind(profile_id)
        .bind(&journey.entry_node)
        .fetch_one(&self.pool)
        .await?;

        self.engine
            .log_event(
                run.id,
                &journey.entry_node,
                crate::model::RunEventKind::Entered,
                None,
            )
            .await?;

        self.run_to_completion_or_pause(run, &journey).await
    }

    async fn advance_due_runs(&self) -> Result<usize, JourneyError> {
        let due: Vec<JourneyRun> = sqlx::query_as::<_, JourneyRun>(&format!(
            r#"
            SELECT {RUN_COLUMNS} FROM journey_runs
            WHERE status = 'waiting' AND wake_at <= now()
            "#
        ))
        .fetch_all(&self.pool)
        .await?;

        let mut advanced = 0;
        for run in due {
            let journey_row = sqlx::query_as::<_, JourneyRow>(&format!(
                "SELECT {JOURNEY_COLUMNS} FROM journeys WHERE id = $1"
            ))
            .bind(run.journey_id)
            .fetch_optional(&self.pool)
            .await?;
            let Some(journey_row) = journey_row else {
                continue;
            };
            let journey: JourneyDefinition = journey_row.try_into()?;

            self.run_to_completion_or_pause(run, &journey).await?;
            advanced += 1;
        }
        Ok(advanced)
    }
}

#[async_trait]
impl ContactPolicyRepo for PgJourneyRepo {
    async fn register_policy(
        &self,
        tenant_id: Uuid,
        max_messages: i32,
        window_days: i32,
        channel_id: Option<Uuid>,
    ) -> Result<ContactPolicy, JourneyError> {
        let row = sqlx::query_as::<_, ContactPolicy>(
            r#"
            INSERT INTO contact_policies (id, tenant_id, max_messages, window_days, channel_id, created_at)
            VALUES ($1, $2, $3, $4, $5, now())
            RETURNING id, tenant_id, max_messages, window_days, channel_id, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(max_messages)
        .bind(window_days)
        .bind(channel_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_policies(&self, tenant_id: Uuid) -> Result<Vec<ContactPolicy>, JourneyError> {
        let rows = sqlx::query_as::<_, ContactPolicy>(
            "SELECT id, tenant_id, max_messages, window_days, channel_id, created_at FROM contact_policies WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
