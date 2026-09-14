use serde::{Deserialize, Serialize};

/// A single identity claim attached to an event, in some namespace
/// (`email`, `anonymous_id`, `device_id`, ...). Namespaces are registered
/// via `mb-namespaces`, never hardcoded here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdentityRef {
    pub namespace: String,
    pub value: String,
    #[serde(default)]
    pub primary: bool,
    /// Where this identity claim came from (connector name, SDK, etc.).
    pub source: String,
    /// 1.0 for deterministic identifiers; lower for probabilistic matches.
    #[serde(default = "IdentityRef::default_confidence")]
    pub confidence: f64,
}

impl IdentityRef {
    fn default_confidence() -> f64 {
        1.0
    }
}
