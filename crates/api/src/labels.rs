use std::collections::HashMap;

use mb_core::MixinRef;
use mb_mixins::{MixinError, MixinRepo};
use mb_profile::Profile;

use crate::error::ApiError;
use crate::state::AppState;

/// For every mixin field actually populated on a profile, the governance
/// labels its current mixin definition declares for that field —
/// `"core/contact@1.0.email" -> ["PII", "DIRECT_IDENTIFIER"]`. Only fields
/// with a value on this specific profile are considered — not every field
/// the mixin could ever have — since that's what an activation payload
/// (`profile.mixins`) actually contains.
pub async fn profile_field_labels(
    state: &AppState,
    profile: &Profile,
) -> Result<HashMap<String, Vec<String>>, ApiError> {
    let mut out = HashMap::new();
    let Some(mixins_obj) = profile.mixins.as_object() else {
        return Ok(out);
    };

    for (mixin_key, fields_value) in mixins_obj {
        let Ok(mixin_ref) = MixinRef::parse(mixin_key) else {
            continue;
        };
        let mixin_def = match state
            .mixins
            .get(&mixin_ref.namespace, &mixin_ref.name, &mixin_ref.version)
            .await
        {
            Ok(def) => def,
            Err(MixinError::NotFound { .. }) => continue,
            Err(e) => return Err(e.into()),
        };
        let Some(fields_obj) = fields_value.as_object() else {
            continue;
        };
        for field_key in fields_obj.keys() {
            if let Some(field_def) = mixin_def.fields.get(field_key) {
                out.insert(format!("{mixin_key}.{field_key}"), field_def.labels.clone());
            }
        }
    }

    Ok(out)
}

/// Deduplicated union of every label across a field-label map — what
/// `PolicyRepo::evaluate_labels` needs for a single profile's activation
/// check.
pub fn flatten_labels(map: &HashMap<String, Vec<String>>) -> Vec<String> {
    let mut set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for labels in map.values() {
        set.extend(labels.iter().cloned());
    }
    set.into_iter().collect()
}
