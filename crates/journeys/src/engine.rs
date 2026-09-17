use std::sync::Arc;

use chrono::Utc;
use mb_audiences::evaluate_condition;
use mb_channels::{ChannelAdapter, ChannelRepo, RenderedMessage};
use mb_profile::ProfileRepo;
use mb_templates::TemplateRepo;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::JourneyError;
use crate::model::{JourneyDefinition, JourneyRun, NodeKind, RunEventKind, RunStatus, StepOutcome};
use crate::split::choose_branch;

pub struct Engine {
    pub pool: PgPool,
    pub profiles: Arc<dyn ProfileRepo>,
    pub templates: Arc<dyn TemplateRepo>,
    pub channels: Arc<dyn ChannelRepo>,
    pub channel_adapter: Arc<dyn ChannelAdapter>,
}

impl Engine {
    /// Executes the run's current node exactly once. `Condition`/`Split`
    /// resolve immediately and report `Progressed`, so the caller can loop
    /// until it sees `Waiting`/`Completed` to run a whole "tick" through
    /// several synchronous nodes in one poll.
    pub async fn advance_run(
        &self,
        run: &mut JourneyRun,
        journey: &JourneyDefinition,
    ) -> Result<StepOutcome, JourneyError> {
        let node = journey
            .nodes
            .iter()
            .find(|n| n.id == run.current_node)
            .ok_or_else(|| JourneyError::UnknownNode(run.current_node.clone()))?
            .clone();

        match node.kind {
            NodeKind::Wait {
                duration_seconds,
                next,
            } => match run.wake_at {
                None => {
                    let wake_at = Utc::now() + chrono::Duration::seconds(duration_seconds);
                    self.set_waiting(run.id, wake_at).await?;
                    run.wake_at = Some(wake_at);
                    run.status = RunStatus::Waiting;
                    Ok(StepOutcome::Waiting)
                }
                Some(wake_at) if wake_at > Utc::now() => Ok(StepOutcome::Waiting),
                Some(_) => {
                    self.transition(run, &next).await?;
                    self.log_event(run.id, &node.id, RunEventKind::Waited, None)
                        .await?;
                    Ok(StepOutcome::Progressed)
                }
            },

            NodeKind::Condition {
                condition,
                if_true,
                if_false,
            } => {
                let mixins = self.profile_mixins(run.profile_id).await?;
                let result = evaluate_condition(
                    &self.pool,
                    &condition,
                    &mixins,
                    run.tenant_id,
                    run.profile_id,
                )
                .await?;
                let next = if result { &if_true } else { &if_false };
                self.transition(run, next).await?;
                self.log_event(
                    run.id,
                    &node.id,
                    RunEventKind::Branched,
                    Some(result.to_string()),
                )
                .await?;
                Ok(StepOutcome::Progressed)
            }

            NodeKind::Split { branches } => {
                let next = choose_branch(&branches, run.profile_id, &node.id);
                self.transition(run, &next).await?;
                self.log_event(
                    run.id,
                    &node.id,
                    RunEventKind::Branched,
                    Some(format!("-> {next}")),
                )
                .await?;
                Ok(StepOutcome::Progressed)
            }

            NodeKind::Action {
                channel_id,
                template_id,
                next,
            } => {
                self.execute_action(run, &node.id, channel_id, template_id)
                    .await?;
                self.transition(run, &next).await?;
                Ok(StepOutcome::Progressed)
            }

            NodeKind::End => {
                self.complete(run.id).await?;
                run.status = RunStatus::Completed;
                self.log_event(run.id, &node.id, RunEventKind::Completed, None)
                    .await?;
                Ok(StepOutcome::Completed)
            }
        }
    }

    async fn profile_mixins(&self, profile_id: Uuid) -> Result<serde_json::Value, JourneyError> {
        Ok(self
            .profiles
            .get(profile_id)
            .await?
            .map(|p| p.mixins)
            .unwrap_or_else(|| serde_json::json!({})))
    }

    /// Render, check the contact/frequency policy, and send — idempotent
    /// under retry via `messages_sent`'s `UNIQUE(run_id, node_id)`: the
    /// reservation insert is what makes a resumed run not double-send, at
    /// the cost of "at most once" for the underlying webhook call once
    /// reserved (better than risking a duplicate message to a customer).
    async fn execute_action(
        &self,
        run: &JourneyRun,
        node_id: &str,
        channel_id: Uuid,
        template_id: Uuid,
    ) -> Result<(), JourneyError> {
        let profile = self
            .profiles
            .get(run.profile_id)
            .await?
            .ok_or(JourneyError::ProfileNotFound(run.profile_id))?;
        let template = self.templates.get(run.tenant_id, template_id).await?;
        let rendered = mb_templates::render(&template, &profile.mixins);
        let channel = self.channels.get(run.tenant_id, channel_id).await?;

        if self
            .is_suppressed(run.tenant_id, run.profile_id, channel_id)
            .await?
        {
            sqlx::query(
                r#"
                INSERT INTO messages_sent (id, tenant_id, run_id, node_id, channel_id, profile_id, subject, body, status, detail, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'suppressed', 'contact policy limit reached', now())
                ON CONFLICT (run_id, node_id) DO NOTHING
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(run.tenant_id)
            .bind(run.id)
            .bind(node_id)
            .bind(channel_id)
            .bind(run.profile_id)
            .bind(&rendered.subject)
            .bind(&rendered.body)
            .execute(&self.pool)
            .await?;
            self.log_event(run.id, node_id, RunEventKind::ActionSuppressed, None)
                .await?;
            return Ok(());
        }

        let reserved: Option<Uuid> = sqlx::query_scalar(
            r#"
            INSERT INTO messages_sent (id, tenant_id, run_id, node_id, channel_id, profile_id, subject, body, status, detail, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'sent', NULL, now())
            ON CONFLICT (run_id, node_id) DO NOTHING
            RETURNING id
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(run.tenant_id)
        .bind(run.id)
        .bind(node_id)
        .bind(channel_id)
        .bind(run.profile_id)
        .bind(&rendered.subject)
        .bind(&rendered.body)
        .fetch_optional(&self.pool)
        .await?;

        let Some(message_id) = reserved else {
            // Already attempted before a crash — don't resend.
            return Ok(());
        };

        let message = RenderedMessage {
            subject: rendered.subject,
            body: rendered.body,
        };

        match self.channel_adapter.send(&channel.config, &message).await {
            Ok(()) => {
                self.log_event(run.id, node_id, RunEventKind::ActionSent, None)
                    .await?;
            }
            Err(e) => {
                sqlx::query(
                    "UPDATE messages_sent SET status = 'failed', detail = $1 WHERE id = $2",
                )
                .bind(e.to_string())
                .bind(message_id)
                .execute(&self.pool)
                .await?;
                self.log_event(
                    run.id,
                    node_id,
                    RunEventKind::ActionFailed,
                    Some(e.to_string()),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn is_suppressed(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
        channel_id: Uuid,
    ) -> Result<bool, JourneyError> {
        let policies: Vec<(i32, i32, Option<Uuid>)> = sqlx::query_as(
            "SELECT max_messages, window_days, channel_id FROM contact_policies WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        for (max_messages, window_days, policy_channel) in policies {
            if let Some(pc) = policy_channel {
                if pc != channel_id {
                    continue;
                }
            }
            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT count(*) FROM messages_sent
                WHERE tenant_id = $1 AND profile_id = $2 AND status = 'sent'
                  AND created_at >= now() - ($3 || ' days')::interval
                  AND ($4::uuid IS NULL OR channel_id = $4)
                "#,
            )
            .bind(tenant_id)
            .bind(profile_id)
            .bind(window_days.to_string())
            .bind(policy_channel)
            .fetch_one(&self.pool)
            .await?;
            if count >= max_messages as i64 {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn set_waiting(
        &self,
        run_id: Uuid,
        wake_at: chrono::DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE journey_runs SET wake_at = $1, status = 'waiting', updated_at = now() WHERE id = $2")
            .bind(wake_at)
            .bind(run_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn transition(&self, run: &mut JourneyRun, next_node: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE journey_runs SET current_node = $1, wake_at = NULL, status = 'running', updated_at = now() WHERE id = $2",
        )
        .bind(next_node)
        .bind(run.id)
        .execute(&self.pool)
        .await?;
        run.current_node = next_node.to_string();
        run.wake_at = None;
        run.status = RunStatus::Running;
        Ok(())
    }

    async fn complete(&self, run_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE journey_runs SET status = 'completed', updated_at = now() WHERE id = $1",
        )
        .bind(run_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn log_event(
        &self,
        run_id: Uuid,
        node_id: &str,
        kind: RunEventKind,
        detail: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO journey_run_events (id, run_id, node_id, kind, detail, created_at) VALUES ($1, $2, $3, $4, $5, now())",
        )
        .bind(Uuid::new_v4())
        .bind(run_id)
        .bind(node_id)
        .bind(kind)
        .bind(detail)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
