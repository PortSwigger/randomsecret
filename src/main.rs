// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: The randomsecret contributors

mod crd;
mod generate;
mod reconcile;

use std::sync::Arc;

use futures::StreamExt;
use k8s_openapi::api::core::v1::Secret;
use kube::runtime::controller::Controller;
use kube::runtime::watcher;
use kube::{Api, Client, CustomResourceExt};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::crd::RandomSecret;
use crate::reconcile::{Context, error_policy, reconcile};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("crd") {
        print!("{}", serde_yaml::to_string(&RandomSecret::crd())?);
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let client = Client::try_default().await?;
    let random_secrets = Api::<RandomSecret>::all(client.clone());
    let secrets = Api::<Secret>::all(client.clone());

    info!("starting randomsecret operator");
    Controller::new(random_secrets, watcher::Config::default())
        .owns(secrets, watcher::Config::default())
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(Context { client }))
        .for_each(|result| async move {
            match result {
                Ok(object) => info!("reconciled {object:?}"),
                Err(error) => warn!("reconcile failed: {error}"),
            }
        })
        .await;
    info!("controller terminated");
    Ok(())
}
