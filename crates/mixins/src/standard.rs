use mb_schema_registry::FieldDef;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Shape of a `standard-mixins/*.yaml` file, before it becomes a stored
/// `MixinDef` (which additionally carries an id/status/created_at once
/// it's persisted).
#[derive(Debug, Clone, Deserialize)]
pub struct StandardMixinSpec {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub fields: BTreeMap<String, FieldDef>,
}

const IDENTITY_YAML: &str = include_str!("../../../standard-mixins/identity@1.0.yaml");
const PERSON_YAML: &str = include_str!("../../../standard-mixins/person@1.0.yaml");
const CONTACT_YAML: &str = include_str!("../../../standard-mixins/contact@1.0.yaml");
const DEVICE_YAML: &str = include_str!("../../../standard-mixins/device@1.0.yaml");

/// The standard mixin library shipped with this milestone: identity,
/// person, contact, device. The rest of Section 6's library (commerce,
/// engagement, loyalty, consent, ...) arrives in later phases as the
/// primitives that use them (audiences, governance, engagement) get built.
pub fn standard_library() -> Vec<StandardMixinSpec> {
    [IDENTITY_YAML, PERSON_YAML, CONTACT_YAML, DEVICE_YAML]
        .iter()
        .map(|yaml| {
            serde_yaml::from_str(yaml)
                .expect("embedded standard mixin YAML must parse — this is a build-time invariant")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_library_loads_all_four_mixins() {
        let mixins = standard_library();
        let names: Vec<_> = mixins.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["identity", "person", "contact", "device"]);
        assert!(mixins.iter().all(|m| m.namespace == "core"));
    }
}
