use mb_schema_registry::{validate_fields, FieldViolation};

use crate::model::MixinDef;

/// Validates a profile's field data for one mixin against that mixin's
/// field definitions. Used when composing a profile from several mixins,
/// and when registering/updating a profile's mixin data directly.
pub fn validate_mixin_fields(mixin: &MixinDef, data: &serde_json::Value) -> Vec<FieldViolation> {
    validate_fields(&mixin.fields, data)
}
