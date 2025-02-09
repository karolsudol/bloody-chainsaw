use anyhow::Result;
use ethers::{
    providers::{Middleware, Provider, StreamExt, Ws},
    types::{Filter, BlockNumber},
};
use std::sync::Arc;
use vault_indexer_core::{VaultConfig, VaultState};
use vault_indexer_storage::Storage;

pub struct VaultIndexer {
    provider: Arc<Provider<Ws>>,
    vault_address: ethers::types::Address,
    storage: Box<dyn Storage>,
}

impl VaultIndexer {
    pub async fn new(config: VaultConfig, storage: Box<dyn Storage>) -> Result<Self> {
        let provider = Provider::<Ws>::connect(&config.rpc_url).await?;
        
        Ok(Self {
            provider: Arc::new(provider),
            vault_address: config.vault_address,
            storage,
        })
    }

    pub async fn run(&self) -> Result<()> {
        // Get initial state
        let initial_state = self.get_vault_state(BlockNumber::Latest).await?;
        self.storage.save_state(&initial_state).await?;

        // Subscribe to logs for the vault contract
        let filter = Filter::new()
            .address(self.vault_address)
            .from_block(BlockNumber::Latest);
        let mut event_stream = self.provider.subscribe_logs(&filter).await?;

        while let Some(log) = event_stream.next().await {
            match log {
                Ok(log) => {
                    // Update state when events occur
                    if let Ok(state) = self.get_vault_state(log.block_number.unwrap().as_u64()).await {
                        self.storage.save_state(&state).await?;
                    }
                }
                Err(e) => log::error!("Error processing log: {:?}", e),
            }
        }

        Ok(())
    }

    async fn get_vault_state(&self, block: impl Into<BlockNumber>) -> Result<VaultState> {
        // Implementation for getting vault state
        // This would call the contract methods for totalAssets and totalSupply
        todo!("Implement get_vault_state")
    }
} 