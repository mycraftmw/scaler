use once_cell::sync::OnceCell;
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::pb::serverless_simulator::Meta;

static META_DATA: OnceCell<RwLock<HashMap<String, Meta>>> = OnceCell::new();

pub fn get_meta_data() -> &'static RwLock<HashMap<String, Meta>> {
    META_DATA.get_or_init(|| RwLock::new(HashMap::new()))
}

pub async fn insert_meta_data(meta: Meta) {
    get_meta_data().write().await.insert(meta.key.clone(), meta);
}
