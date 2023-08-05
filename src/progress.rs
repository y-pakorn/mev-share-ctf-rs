use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::Arc,
};

use ethers_core::types::{H160, U64};
use ethers_signers::Signer;
use tokio::sync::RwLock;

use crate::constants::WALLET;

#[derive(Debug, Clone)]
pub struct Progress {
    pub inner: Arc<RwLock<HashMap<H160, HashSet<H160>>>>,
    pub processing: Arc<RwLock<HashSet<H160>>>,
    pub latest_block: Arc<RwLock<U64>>,
}

unsafe impl Send for Progress {}
unsafe impl Sync for Progress {}

impl Progress {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            processing: Arc::new(RwLock::new(HashSet::new())),
            latest_block: Arc::new(RwLock::new(U64::zero())),
        }
    }

    pub fn read() -> Self {
        Self {
            inner: Arc::new(RwLock::new(read_ctf_progress().unwrap())),
            processing: Arc::new(RwLock::new(HashSet::new())),
            latest_block: Arc::new(RwLock::new(U64::zero())),
        }
    }

    pub async fn get_latest_block(&self) -> U64 {
        *self.latest_block.read().await
    }

    pub async fn set_latest_block(&self, block: U64) {
        *self.latest_block.write().await = block;
    }

    pub async fn save(&self) {
        write_ctf_progress(&self.inner.read().await.clone());
    }

    pub async fn get_is_processing(&self, address: H160) -> bool {
        self.processing.read().await.get(&address).is_some()
    }

    pub async fn set_is_processing(&self, address: H160, is_processing: bool) {
        if is_processing {
            self.processing.write().await.insert(address);
        } else {
            self.processing.write().await.remove(&address);
        }
    }

    pub async fn get_progress_for_address(&self, contract: H160) -> bool {
        self.inner
            .read()
            .await
            .get(&WALLET.address())
            .cloned()
            .unwrap_or_default()
            .get(&contract)
            .is_some()
    }

    pub async fn add_progress_for_address(&self, contract: H160) {
        self.inner
            .write()
            .await
            .entry(WALLET.address())
            .or_default()
            .insert(contract);
        self.save().await;
    }
}

pub fn read_ctf_progress() -> Result<HashMap<H160, HashSet<H160>>, Box<dyn Error>> {
    let data = std::fs::read_to_string("ctf_progress.json").unwrap_or_else(|_| "{}".to_string());
    let progress = serde_json::from_str::<HashMap<H160, HashSet<H160>>>(&data)?;
    Ok(progress)
}

pub fn write_ctf_progress(progress: &HashMap<H160, HashSet<H160>>) {
    let data = serde_json::to_string(progress).unwrap();
    std::fs::write("ctf_progress.json", data).unwrap();
}
