// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: The randomsecret contributors

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "noa.re",
    version = "v1alpha1",
    kind = "RandomSecret",
    namespaced,
    status = "RandomSecretStatus"
)]
pub struct RandomSecretSpec {
    /// The entries of the Secret that this RandomSecret manages. The Secret is
    /// created with the same name and namespace as this resource.
    pub secrets: Vec<SecretEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RandomSecretStatus {
    /// The value of `.metadata.generation` that was most recently reconciled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_generation: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
pub struct SecretEntry {
    /// The key under which the generated value is stored in the Secret.
    pub name: String,
    /// Length in characters of the generated value. When omitted, the value is
    /// made long enough to contain at least 256 bits of entropy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1))]
    pub length: Option<u32>,
}

// Workaround for https://github.com/kube-rs/kube/issues/1680 where kube-derive
// emits these as empty arrays even when unset; the API server drops them on
// apply, causing Argo CD to report perpetual drift. An allow-list rather than
// "prune all empties" because an empty collection isn't always equivalent to an
// absent one in a CRD.
const PRUNABLE_EMPTY_ARRAYS: &[&str] = &["categories", "shortNames", "additionalPrinterColumns"];

/// Remove the [`PRUNABLE_EMPTY_ARRAYS`] fields wherever they appear as an empty
/// array. Unlisted fields and empty objects (e.g. `subresources.status`) are
/// left untouched.
pub fn prune_empty(value: &mut serde_json::Value) {
    use serde_json::Value;
    match value {
        Value::Array(items) => items.iter_mut().for_each(prune_empty),
        Value::Object(map) => {
            map.values_mut().for_each(prune_empty);
            map.retain(|key, v| {
                let is_prunable_empty = PRUNABLE_EMPTY_ARRAYS.contains(&key.as_str())
                    && matches!(v, Value::Array(a) if a.is_empty());
                !is_prunable_empty
            });
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kube::CustomResourceExt;

    #[test]
    fn prune_empty_drops_empty_optional_fields() {
        let mut crd = serde_json::to_value(RandomSecret::crd()).unwrap();
        prune_empty(&mut crd);

        let names = &crd["spec"]["names"];
        assert!(names.get("categories").is_none());
        assert!(names.get("shortNames").is_none());

        let version = &crd["spec"]["versions"][0];
        assert!(version.get("additionalPrinterColumns").is_none());
    }

    #[test]
    fn prune_empty_preserves_meaningful_empty_objects() {
        let mut crd = serde_json::to_value(RandomSecret::crd()).unwrap();
        prune_empty(&mut crd);

        // An empty `status` object is what enables the status subresource; it
        // must survive pruning even though it is empty.
        assert!(crd["spec"]["versions"][0]["subresources"]["status"].is_object());
    }

    #[test]
    fn prune_empty_keeps_empty_arrays_not_on_the_allow_list() {
        let mut value = serde_json::json!({
            "categories": [],           // on the allow-list -> pruned
            "required": [],             // not on the allow-list -> kept
            "nested": { "shortNames": [] },
        });
        prune_empty(&mut value);

        assert!(value.get("categories").is_none());
        assert_eq!(value["required"], serde_json::json!([]));
        assert!(value["nested"].get("shortNames").is_none());
    }
}
