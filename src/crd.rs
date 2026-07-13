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
