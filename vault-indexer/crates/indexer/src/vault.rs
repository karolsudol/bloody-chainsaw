use anyhow::Result;
use ethers::{
    abi::Abi,
    contract::Contract,
    providers::{Middleware, Provider, StreamExt, Ws},
    types::{Address, BlockNumber, Filter, U256},
};
use std::sync::Arc;
use vault_indexer_core::{VaultConfig, VaultState};

pub struct VaultIndexer {
    provider: Arc<Provider<Ws>>,
    contract: Contract<Provider<Ws>>,
}

impl VaultIndexer {
    pub async fn new(config: VaultConfig) -> Result<Self> {
        let provider = Arc::new(Provider::<Ws>::connect(&config.rpc_url).await?);
        
        // Load contract ABI
        let abi: Abi = serde_json::from_str(include_str!("../../../abi/vault.json"))?;
        let contract = Contract::new(config.vault_address, abi, provider.clone());

        Ok(Self {
            provider,
            contract,
        })
    }

    pub async fn run(&self) -> Result<()> {
        // Get and print initial state
        let initial_state = self.get_vault_state(BlockNumber::Latest).await?;
        println!("Initial state: {:#?}", initial_state);

        // Subscribe to relevant events
        let filter = Filter::new()
            .address(self.contract.address())
            .event("Transfer") // Add other relevant events if needed
            .from_block(BlockNumber::Latest);
            
        let mut event_stream = self.provider.subscribe_logs(&filter).await?;

        println!("Listening for events...");
        
        while let Some(log) = event_stream.next().await {
            match log {
                Ok(log) => {
                    let block_number = log.block_number.unwrap();
                    println!("\nNew event at block {}", block_number);
                    
                    // Get and print updated state
                    if let Ok(new_state) = self.get_vault_state(block_number).await {
                        println!("Updated state: {:#?}", new_state);
                    }
                }
                Err(e) => println!("Error processing log: {:?}", e),
            }
        }

        Ok(())
    }

    async fn get_vault_state(&self, block: impl Into<BlockNumber>) -> Result<VaultState> {
        let block = block.into();
        
        // Call contract methods to get current state
        let total_assets: U256 = self.contract.method("totalAssets", ())?.call().await?;
        let total_supply: U256 = self.contract.method("totalSupply", ())?.call().await?;
        let rate: U256 = self.contract.method("rate", ())?.call().await?;
        let asset_address: Address = self.contract.method("asset", ())?.call().await?;
        let atoken_address: Address = self.contract.method("aToken", ())?.call().await?;
        let reward_tokens: Vec<Address> = self.contract.method("rewardTokens", ())?.call().await?;
        
        let block_details = self.provider.get_block(block).await?
            .expect("Block not found");
        
        Ok(VaultState {
            block_number: block_details.number.unwrap().as_u64(),
            timestamp: block_details.timestamp.as_u64(),
            total_assets,
            total_supply,
            rate,
            asset_address,
            atoken_address,
            reward_tokens,
        })
    }
} 