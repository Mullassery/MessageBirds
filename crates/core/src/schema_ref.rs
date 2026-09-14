use serde::{Deserialize, Serialize};

/// Reference to a registered event schema, e.g. `commerce.product_view@1.0`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SchemaRef {
    pub name: String,
    pub version: String,
}

impl std::fmt::Display for SchemaRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}
