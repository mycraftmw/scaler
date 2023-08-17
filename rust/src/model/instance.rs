use anyhow::{Context, Result};
use std::collections::HashMap;
use tokio::time::Instant;

use once_cell::sync::OnceCell;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::RwLock;

use crate::pb::serverless_simulator::{Meta, Slot};

#[derive(Debug, Clone)]
pub struct Instance {
    pub instance_id: String,
    pub slot: Slot,
    pub create_time_ms: u64,
    pub init_duration_ms: u64,
    pub last_idle_time: Instant,
    pub meta: Meta,
}

static IDLE_INSTANCE: OnceCell<RwLock<HashMap<String, RwLock<Receiver<Instance>>>>> =
    OnceCell::new();
static INSTANCE_FEED: OnceCell<RwLock<HashMap<String, Sender<Instance>>>> = OnceCell::new();
static INSTANCE_MAP: OnceCell<RwLock<HashMap<String, Instance>>> = OnceCell::new();

fn idle_instance() -> &'static RwLock<HashMap<String, RwLock<Receiver<Instance>>>> {
    IDLE_INSTANCE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn instance_feed() -> &'static RwLock<HashMap<String, Sender<Instance>>> {
    INSTANCE_FEED.get_or_init(|| RwLock::new(HashMap::new()))
}

fn instance_map() -> &'static RwLock<HashMap<String, Instance>> {
    INSTANCE_MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

/// try to add idle instance, will return err if buffer is full
pub async fn try_add_idle_instance(instance: Instance) -> Result<()> {
    instance_feed()
        .read()
        .await
        .get(&instance.meta.key)
        .context("should have sender channel")?
        .try_send(instance)?;
    Ok(())
}

/// await to add idle instance
pub async fn add_idle_instance(instance: Instance) -> Result<()> {
    instance_feed()
        .read()
        .await
        .get(&instance.meta.key)
        .context("should have sender channel")?
        .send(instance)
        .await?;
    Ok(())
}

/// update instance channels
pub async fn insert_instance_channel(meta_key: String) {
    let mut guard = idle_instance().write().await;
    if !guard.contains_key(&meta_key) {
        let (tx, rx) = tokio::sync::mpsc::channel(5);
        guard.insert(meta_key.clone(), RwLock::new(rx));
        instance_feed().write().await.insert(meta_key, tx);
    }
}

/// await to get available instance
pub async fn get_idle_instance(meta_key: &str) -> Result<Instance> {
    idle_instance()
        .read()
        .await
        .get(meta_key)
        .context("should have receiver")?
        .write()
        .await
        .recv()
        .await
        .context("sender is closed")
}

/// try to get available instance, immediately return if not any
pub async fn try_get_idle_instance(meta_key: &str) -> Option<Instance> {
    if let Some(receiver) = idle_instance().read().await.get(meta_key) {
        return receiver.write().await.try_recv().ok();
    }
    insert_instance_channel(meta_key.to_string()).await;
    None
}

/// update instance map for assigned instance
pub async fn update_instance_map(instance: Instance) {
    instance_map()
        .write()
        .await
        .insert(instance.instance_id.clone(), instance);
}

pub async fn remove_from_instance_map(instance_id: String) -> Result<Instance> {
    instance_map()
        .write()
        .await
        .remove(&instance_id)
        .context("instance not found")
}
