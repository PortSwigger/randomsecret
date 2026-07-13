// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: The randomsecret contributors

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use k8s_openapi::ByteString;
use k8s_openapi::api::core::v1::Secret;
use kube::api::{ObjectMeta, Patch, PatchParams, Resource};
use kube::runtime::controller::Action;
use kube::{Api, Client, ResourceExt};
use rand::{CryptoRng, Rng};
use serde_json::json;
use tracing::{info, warn};

use crate::crd::{RandomSecret, RandomSecretSpec};
use crate::generate::{DEFAULT_LENGTH, generate};

const FIELD_MANAGER: &str = "randomsecret-operator";

pub struct Context {
    pub client: Client,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("kube api error: {0}")]
    Kube(#[from] kube::Error),
    #[error("RandomSecret is missing {0}")]
    MissingObjectKey(&'static str),
}

/// Compute the Secret data for `spec`, keeping values from `existing` where
/// present, generating values for keys that are missing and dropping keys that
/// are no longer in the spec.
fn desired_data<R: Rng + CryptoRng>(
    spec: &RandomSecretSpec,
    existing: &BTreeMap<String, ByteString>,
    rng: &mut R,
) -> BTreeMap<String, ByteString> {
    spec.secrets
        .iter()
        .map(|entry| {
            let value = existing.get(&entry.name).cloned().unwrap_or_else(|| {
                let length = entry.length.map_or(DEFAULT_LENGTH, |l| l as usize);
                ByteString(generate(length, rng).into_bytes())
            });
            (entry.name.clone(), value)
        })
        .collect()
}

pub async fn reconcile(rs: Arc<RandomSecret>, ctx: Arc<Context>) -> Result<Action, Error> {
    let namespace = rs
        .namespace()
        .ok_or(Error::MissingObjectKey(".metadata.namespace"))?;
    let name = rs.name_any();
    let secrets = Api::<Secret>::namespaced(ctx.client.clone(), &namespace);

    let existing = secrets
        .get_opt(&name)
        .await?
        .and_then(|secret| secret.data)
        .unwrap_or_default();

    let data = desired_data(&rs.spec, &existing, &mut rand::rng());
    if data != existing {
        let owner_ref = rs
            .controller_owner_ref(&())
            .ok_or(Error::MissingObjectKey(".metadata.uid"))?;
        let secret = Secret {
            metadata: ObjectMeta {
                name: Some(name.clone()),
                namespace: Some(namespace.clone()),
                owner_references: Some(vec![owner_ref]),
                ..ObjectMeta::default()
            },
            data: Some(data),
            ..Secret::default()
        };
        secrets
            .patch(
                &name,
                &PatchParams::apply(FIELD_MANAGER),
                &Patch::Apply(&secret),
            )
            .await?;
        info!("updated Secret {namespace}/{name}");
    }

    let generation = rs.meta().generation;
    let observed = rs
        .status
        .as_ref()
        .and_then(|status| status.observed_generation);
    if observed != generation {
        let random_secrets = Api::<RandomSecret>::namespaced(ctx.client.clone(), &namespace);
        let status = json!({ "status": { "observedGeneration": generation } });
        random_secrets
            .patch_status(&name, &PatchParams::default(), &Patch::Merge(&status))
            .await?;
        info!("updated status of RandomSecret {namespace}/{name} to generation {generation:?}");
    }

    Ok(Action::requeue(Duration::from_secs(300)))
}

pub fn error_policy(_object: Arc<RandomSecret>, error: &Error, _ctx: Arc<Context>) -> Action {
    warn!("reconcile failed: {error}");
    Action::requeue(Duration::from_secs(5))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crd::SecretEntry;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn spec(entries: &[(&str, Option<u32>)]) -> RandomSecretSpec {
        RandomSecretSpec {
            secrets: entries
                .iter()
                .map(|(name, length)| SecretEntry {
                    name: name.to_string(),
                    length: *length,
                })
                .collect(),
        }
    }

    #[test]
    fn generates_missing_keys_with_configured_lengths() {
        let mut rng = StdRng::seed_from_u64(0);
        let spec = spec(&[("MY_SECRET_NAME", Some(40)), ("WITH_DEFAULT", None)]);
        let data = desired_data(&spec, &BTreeMap::new(), &mut rng);
        assert_eq!(data.len(), 2);
        assert_eq!(data["MY_SECRET_NAME"].0.len(), 40);
        assert_eq!(data["WITH_DEFAULT"].0.len(), DEFAULT_LENGTH);
    }

    #[test]
    fn keeps_existing_values() {
        let mut rng = StdRng::seed_from_u64(1);
        let existing = BTreeMap::from([("KEEP_ME".to_string(), ByteString(b"unchanged".to_vec()))]);
        let data = desired_data(&spec(&[("KEEP_ME", None)]), &existing, &mut rng);
        assert_eq!(data, existing);
    }

    #[test]
    fn drops_keys_no_longer_in_spec() {
        let mut rng = StdRng::seed_from_u64(2);
        let existing = BTreeMap::from([
            ("KEEP_ME".to_string(), ByteString(b"unchanged".to_vec())),
            ("DROP_ME".to_string(), ByteString(b"old".to_vec())),
        ]);
        let data = desired_data(&spec(&[("KEEP_ME", None)]), &existing, &mut rng);
        assert_eq!(data.len(), 1);
        assert_eq!(data["KEEP_ME"], existing["KEEP_ME"]);
    }
}
