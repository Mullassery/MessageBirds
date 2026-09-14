use serde::{Deserialize, Serialize};

/// Reference to a mixin, e.g. `core/identity@1.0` or a custom, namespace-owned
/// mixin like `acme/fitness_membership@1.0`. The namespace prevents custom
/// mixins from colliding with (or silently shadowing) the standard library.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MixinRef {
    pub namespace: String,
    pub name: String,
    pub version: String,
}

pub const STANDARD_MIXIN_NAMESPACE: &str = "core";

impl MixinRef {
    pub fn standard(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            namespace: STANDARD_MIXIN_NAMESPACE.to_string(),
            name: name.into(),
            version: version.into(),
        }
    }

    /// Parses `namespace/name@version`, defaulting the namespace to `core`
    /// when omitted (`name@version`).
    pub fn parse(s: &str) -> Result<Self, String> {
        let (ns_and_name, version) = s.split_once('@').ok_or_else(|| {
            format!("mixin ref '{s}' is missing a version (expected name@version)")
        })?;
        let (namespace, name) = match ns_and_name.split_once('/') {
            Some((ns, name)) => (ns.to_string(), name.to_string()),
            None => (
                STANDARD_MIXIN_NAMESPACE.to_string(),
                ns_and_name.to_string(),
            ),
        };
        if name.is_empty() || version.is_empty() {
            return Err(format!("mixin ref '{s}' has an empty name or version"));
        }
        Ok(Self {
            namespace,
            name,
            version: version.to_string(),
        })
    }
}

impl std::fmt::Display for MixinRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}@{}", self.namespace, self.name, self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_mixin_without_namespace() {
        let m = MixinRef::parse("identity@1.0").unwrap();
        assert_eq!(m.namespace, "core");
        assert_eq!(m.name, "identity");
        assert_eq!(m.version, "1.0");
    }

    #[test]
    fn parses_custom_namespaced_mixin() {
        let m = MixinRef::parse("acme/fitness_membership@1.0").unwrap();
        assert_eq!(m.namespace, "acme");
        assert_eq!(m.name, "fitness_membership");
        assert_eq!(m.version, "1.0");
    }

    #[test]
    fn rejects_missing_version() {
        assert!(MixinRef::parse("identity").is_err());
    }
}
