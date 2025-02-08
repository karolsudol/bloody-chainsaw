# bloody-chainsaw
Real Time data pipeline for ERC-4626: Tokenized Vault Contracts

## Indexer

The indexer is a Rust-based service that monitors ERC-4626 vault contracts in real-time. It tracks:

- Deposit and Withdraw events
- Vault state changes (total assets and total supply)

### Features
- Real-time monitoring of vault events using WebSocket connection
- Tracks key ERC-4626 events:
  - Deposits: When assets are deposited in exchange for shares
  - Withdrawals: When shares are burned to receive assets
- Monitors vault state changes on every new block:
  - Total Assets
  - Total Supply
- Saves events and state changes as JSON files

### Requirements
- Rust toolchain
- WebSocket endpoint for an Ethereum node (WSS URL)
- ERC-4626 vault contract address
- Vault contract ABI (stored in `abi/vault.json`)

### Environment Variables
Create a `.env` file with:
```
WSS_URL=wss://mainnet.infura.io/ws/v3/YOUR_INFURA_PROJECT_ID
VAULT_ADDRESS=0x1234567890123456789012345678901234567890
```

### Running the Indexer
1. Build the project:
```
cd indexer
cargo  build --release
```

2. Run the indexer:
```
cargo run --release
```

### Data Files
Events and state changes are saved as JSON files in the `data` directory:
- `events/` - Contains all events
- `state/` - Contains the latest state
