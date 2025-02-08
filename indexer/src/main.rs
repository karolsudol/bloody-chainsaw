use anyhow::Result;
use ethers::{
    contract::Contract,
    core::{types::*, utils::keccak256},
    providers::{Provider, Ws, Middleware, StreamExt},
};
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc, str::FromStr};

// ERC-4626 specific events
#[derive(Debug, Serialize, Deserialize)]
struct VaultEvent {
    event_type: String,
    block_number: u64,
    transaction_hash: String,
    sender: Address,
    receiver: Option<Address>,
    owner: Option<Address>,
    assets: U256,
    shares: U256,
    timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct VaultState {
    total_assets: U256,
    total_supply: U256,
    block_number: u64,
    timestamp: u64,
}

struct VaultIndexer {
    provider: Arc<Provider<Ws>>,
    contract: Contract<Provider<Ws>>,
    vault_address: Address,
}

impl VaultIndexer {
    async fn new(ws_url: &str, vault_address: &str, abi: &str) -> Result<Self> {
        let provider = Provider::<Ws>::connect(ws_url).await?;
        let provider = Arc::new(provider);
        
        let vault_address = Address::from_str(vault_address)?;
        let abi = serde_json::from_str(abi)?;
        let contract = Contract::new(vault_address, abi, provider.clone());

        Ok(Self {
            provider,
            contract,
            vault_address,
        })
    }

    async fn get_vault_state(&self, block_number: u64) -> Result<VaultState> {
        let block = self.provider.get_block(block_number).await?.unwrap();
        
        // Call ERC-4626 view functions
        let total_assets: U256 = self.contract
            .method::<_, U256>("totalAssets", ())?
            .call()
            .await?;

        let total_supply: U256 = self.contract
            .method::<_, U256>("totalSupply", ())?
            .call()
            .await?;

        Ok(VaultState {
            total_assets,
            total_supply,
            block_number,
            timestamp: block.timestamp.as_u64(),
        })
    }

    fn parse_vault_event(&self, log: &Log, timestamp: u64) -> Result<Option<VaultEvent>> {
        // Define event signatures
        let deposit_sig = "Deposit(address,address,uint256,uint256)";
        let withdraw_sig = "Withdraw(address,address,address,uint256,uint256)";

        let topics = log.topics.iter()
            .map(|t| t.as_bytes().to_vec())
            .collect::<Vec<_>>();

        if topics.is_empty() {
            return Ok(None);
        }

        match topics[0].as_slice() {
            sig if sig == keccak256(deposit_sig).as_ref() => {
                let sender = Address::from_slice(&topics[1][12..]);
                let owner = Address::from_slice(&topics[2][12..]);
                let data = log.data.as_ref();
                let assets = U256::from_big_endian(&data[..32]);
                let shares = U256::from_big_endian(&data[32..]);

                Ok(Some(VaultEvent {
                    event_type: "Deposit".to_string(),
                    block_number: log.block_number.unwrap().as_u64(),
                    transaction_hash: format!("{:?}", log.transaction_hash.unwrap()),
                    sender,
                    receiver: Some(owner),
                    owner: Some(owner),
                    assets,
                    shares,
                    timestamp,
                }))
            },
            sig if sig == keccak256(withdraw_sig).as_bytes() => {
                let sender = Address::from_slice(&topics[1][12..]);
                let receiver = Address::from_slice(&topics[2][12..]);
                let owner = Address::from_slice(&topics[3][12..]);
                let data = log.data.as_ref();
                let assets = U256::from_big_endian(&data[..32]);
                let shares = U256::from_big_endian(&data[32..]);

                Ok(Some(VaultEvent {
                    event_type: "Withdraw".to_string(), 
                    block_number: log.block_number.unwrap().as_u64(),
                    transaction_hash: format!("{:?}", log.transaction_hash.unwrap()),
                    sender,
                    receiver: Some(receiver),
                    owner: Some(owner),
                    assets,
                    shares,
                    timestamp,
                }))
            },
            _ => Ok(None),
        }
    }

    async fn run(&self) -> Result<()> {
        // Subscribe to new blocks
        let mut block_stream = self.provider.subscribe_blocks().await?;
        
        // Subscribe to logs for the vault contract
        let filter = Filter::new()
            .address(self.vault_address)
            .from_block(BlockNumber::Latest);
        let mut event_stream = self.provider.subscribe_logs(&filter).await?;

        log::info!("Starting real-time event monitoring...");

        loop {
            tokio::select! {
                // Handle new blocks
                Some(block) = block_stream.next() => {
                    log::info!("New block: {}", block.number.unwrap_or_default());
                    
                    // Get updated vault state on each new block
                    if let Ok(state) = self.get_vault_state(block.number.unwrap().as_u64()).await {
                        log::info!("Vault state updated: {:?}", state);
                        self.save_state(&state).await?;
                    }
                }
                
                // Handle new logs
                Some(log) = event_stream.next() => {
                    let block = self.provider
                        .get_block(log.block_number.unwrap())
                        .await?
                        .unwrap();
                    
                    if let Some(event) = self.parse_vault_event(&log, block.timestamp.as_u64())? {
                        log::info!("New vault event: {:?}", event);
                        self.save_event(&event).await?;
                    }
                }
            }
        }
    }

    async fn save_event(&self, event: &VaultEvent) -> Result<()> {
        let json = serde_json::to_string_pretty(event)?;
        let timestamp = chrono::Utc::now().timestamp();
        std::fs::write(
            format!("data/event_{}.json", timestamp),
            json,
        )?;
        Ok(())
    }

    async fn save_state(&self, state: &VaultState) -> Result<()> {
        let json = serde_json::to_string_pretty(state)?;
        let timestamp = chrono::Utc::now().timestamp();
        std::fs::write(
            format!("data/state_{}.json", timestamp),
            json,
        )?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    // Use WSS URL instead of HTTP
    let ws_url = env::var("WSS_URL").expect("WSS_URL must be set");
    let vault_address = env::var("VAULT_ADDRESS").expect("VAULT_ADDRESS must be set");
    let abi = include_str!("../../abi/vault.json");

    let indexer = VaultIndexer::new(&ws_url, &vault_address, abi).await?;
    indexer.run().await?;

    Ok(())
}
