use serde::{Deserialize, Serialize};

use super::layer::Layer;

/// A stack of branches, as reported by `gh stack view --json`.
///
/// `gh stack view --json` has no field naming the stack itself (only
/// `trunk`, `currentBranch`, and `branches`), so `name` is left for callers
/// to set from context (e.g. the stack number reported by `gh stack submit`)
/// rather than being sourced from this JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stack {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub trunk: String,
    pub current_branch: String,
    #[serde(rename = "branches")]
    pub layers: Vec<Layer>,
}

impl Stack {
    /// Parses `gh stack view --json` output, filling in each layer's
    /// [`Layer::position`] from its index in `branches`.
    pub fn from_json(raw: &str) -> serde_json::Result<Self> {
        let mut stack: Stack = serde_json::from_str(raw)?;

        for (index, layer) in stack.layers.iter_mut().enumerate() {
            layer.position = index;
        }

        Ok(stack)
    }
}
