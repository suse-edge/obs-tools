use std::sync::Arc;

use lapin::{
    message::Delivery,
    options::{
        BasicAckOptions, BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
    Connection, ConnectionProperties, Result,
};
use obs_client::client::OBSClient;
use serde::Deserialize;
use tokio_stream::StreamExt;
use tracing::{debug, error, span, Level};

use crate::dispatcher::Trigger;

#[derive(Debug, Clone, Deserialize)]
struct RepoPublishedMessage {
    project: String,
    repo: String,
    buildid: String,
}

pub async fn create_client(
    uri: &str,
    projects: Vec<String>,
    trigger: Arc<dyn Trigger>,
    obs_client: Arc<OBSClient>,
) -> Result<tokio::task::JoinHandle<()>> {
    let options = ConnectionProperties::default()
        // Use tokio executor and reactor.
        // At the moment the reactor is only available for unix.
        .with_executor(tokio_executor_trait::Tokio::current())
        .with_reactor(tokio_reactor_trait::Tokio);

    let connection = Connection::connect(uri, options).await.unwrap();
    let channel = connection.create_channel().await.unwrap();

    channel
        .exchange_declare(
            "pubsub",
            lapin::ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                passive: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    let queue = channel
        .queue_declare(
            "",
            QueueDeclareOptions {
                auto_delete: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_bind(
            queue.name().as_str(),
            "pubsub",
            "*.obs.repo.published",
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let mut consumer = channel
        .basic_consume(
            queue.name().as_str(),
            "tag_foo",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    Ok(tokio::spawn(async move {
        while let Some(delivery) = consumer.next().await {
            let _ = span!(tracing::Level::TRACE, "Handling amqp message", ?delivery).entered();

            let delivery = match delivery {
                // Carries the delivery alongside its channel
                Ok(delivery) => delivery,
                Err(error) => {
                    debug!("Failed to consume queue message {}", error);
                    return;
                }
            };

            match handle_message(&delivery, &projects, trigger.clone(), obs_client.clone()).await {
                Ok(_) => delivery
                    .ack(BasicAckOptions::default())
                    .await
                    .expect("Failed to ack message"),
                Err(err) => {
                    error!(?err, "Failed to handle message");
                }
            }
        }
    }))
}

async fn handle_message(
    message: &Delivery,
    projects: &[String],
    trigger: Arc<dyn Trigger>,
    obs_client: Arc<OBSClient>,
) -> anyhow::Result<()> {
    let message: RepoPublishedMessage = serde_yaml::from_slice(&message.data)?;
    span!(
        Level::DEBUG,
        "Handling message",
        project = &message.project,
        trigger_repo = message.repo,
        trigger_buildid = message.buildid
    );
    if !projects.contains(&message.project) {
        debug!(
            project = &message.project,
            "Message is not for a watched project"
        );
        return Ok(());
    }

    let project = obs_client::api::project::Project::from_name(obs_client, &message.project);
    let summary = project.summary().await?;
    if summary.is_all_packages_ok() && summary.is_all_published() {
        return trigger.call(&message.project).await;
    }

    debug!(
        project=&message.project,
        trigger_repo=message.repo,
        trigger_buildid=message.buildid,
        summary=%summary,
        "Received message for project, but all repos are not ready",
    );

    Ok(())
}
