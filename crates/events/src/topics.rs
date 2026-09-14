/// Single topic for this milestone: every tenant's raw, validated-shape
/// events land here, partitioned by tenant + primary identity so a given
/// customer's events stay in order. Per-schema or per-tenant topics are a
/// later scaling concern, not needed to prove the foundation.
pub const RAW_EVENTS_TOPIC: &str = "events.raw";
