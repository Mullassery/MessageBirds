use mb_core::EventEnvelope;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn persist_event(
    pool: &PgPool,
    envelope: &EventEnvelope,
    profile_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO events (id, tenant_id, event_type, schema_name, schema_version, identity, context, data, profile_id, received_at, occurred_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now(), $10)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(envelope.event_id)
    .bind(envelope.tenant_id.0)
    .bind(&envelope.event_type)
    .bind(&envelope.schema.name)
    .bind(&envelope.schema.version)
    .bind(serde_json::to_value(&envelope.identity).expect("identity claims always serialize"))
    .bind(&envelope.context)
    .bind(&envelope.data)
    .bind(profile_id)
    .bind(envelope.timestamp)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn persist_dead_letter(
    pool: &PgPool,
    envelope: &EventEnvelope,
    reason: &str,
) -> Result<(), sqlx::Error> {
    let raw_payload = serde_json::to_value(envelope).expect("envelope always serializes");
    sqlx::query(
        r#"
        INSERT INTO dead_letter_events (id, tenant_id, event_id, schema_name, schema_version, source, reason, raw_payload, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(envelope.tenant_id.0)
    .bind(envelope.event_id)
    .bind(&envelope.schema.name)
    .bind(&envelope.schema.version)
    .bind(&envelope.source.name)
    .bind(reason)
    .bind(raw_payload)
    .execute(pool)
    .await?;
    Ok(())
}
