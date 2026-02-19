# Pump.fun Smart Contract Clone

A production-grade Solana smart contract implementation of Pump.fun's bonding curve token launch mechanism, built with Anchor 0.30+ and SPL Token-2022.

## 📋 Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Bonding Curve Mechanics](#bonding-curve-mechanics)
- [Architecture](#architecture)
- [Installation](#installation)
- [Building & Testing](#building--testing)
- [Deployment](#deployment)
- [Contact](#contact)

## 🎯 Overview

This project implements a complete bonding curve mechanism for launching memecoins on Solana. Users can create tokens with metadata, trade them instantly via a bonding curve (buys push price up, sells pull price down), and automatically graduate to a DEX liquidity pool when the curve reaches completion.

### Key Characteristics

- **Token Supply**: 1,000,000,000 tokens with 6 decimals
- **Bonding Curve Tokens**: ~793 million tokens available for trading
- **Reserved Tokens**: ~207 million tokens reserved for DEX liquidity pool
- **Token Standard**: SPL Token-2022 with Metadata Pointer and Token Metadata extensions
- **Graduation Target**: ~500 SOL virtual market cap (~$75k at $150/SOL)

## ✨ Features

### Core Functionality

1. **Token Creation**
   - Create Token-2022 mints with metadata (name, symbol, URI)
   - Initialize bonding curve with virtual reserves
   - Pay creation fee (0.02 SOL)

2. **Instant Trading**
   - Buy tokens with SOL (mints tokens, increases price)
   - Sell tokens for SOL (burns tokens, decreases price)
   - Constant product bonding curve formula
   - Slippage protection

3. **Protocol Fees**
   - 0.5% fee on all buys and sells
   - Fees sent to treasury PDA
   - Creation fee for new tokens

4. **Automatic Graduation**
   - Curve completes when virtual SOL reserve reaches ~500 SOL
   - Anyone can call `complete` instruction
   - Prepares for DEX pool creation (stub implementation)

### Security Features

- ✅ Anchor framework best practices
- ✅ PDA-based account management
- ✅ Slippage checks on all trades
- ✅ Reentrancy protection via Anchor's account model
- ✅ Authority checks on all privileged operations
- ✅ Custom error types for clear failure cases
- ✅ Math overflow protection with checked arithmetic

## 📊 Bonding Curve Mechanics

### Constant Product Formula

The bonding curve uses a constant product formula (similar to Uniswap's x * y = k):

```
k = virtual_sol_reserve * virtual_token_reserve
```

### Price Calculation

**When Buying (SOL → Tokens):**
1. User sends `sol_in` SOL
2. New virtual SOL reserve: `virtual_sol_reserve + sol_in`
3. New virtual token reserve: `k / (virtual_sol_reserve + sol_in)`
4. Tokens out: `virtual_token_reserve - new_virtual_token_reserve`
5. Protocol fee (0.5%) deducted from tokens out

**When Selling (Tokens → SOL):**
1. User burns `tokens_in` tokens
2. New virtual token reserve: `virtual_token_reserve + tokens_in`
3. New virtual SOL reserve: `k / (virtual_token_reserve + tokens_in)`
4. SOL out: `virtual_sol_reserve - new_virtual_sol_reserve`
5. Protocol fee (0.5%) deducted from SOL out

### Mathematical Formulas

#### Buy Calculation
```
k = virtual_sol_reserve * virtual_token_reserve
new_sol_reserve = virtual_sol_reserve + sol_in
new_token_reserve = k / new_sol_reserve
tokens_out = virtual_token_reserve - new_token_reserve
tokens_out_after_fee = tokens_out * (1 - protocol_fee_bps / 10000)
```

#### Sell Calculation
```
k = virtual_sol_reserve * virtual_token_reserve
new_token_reserve = virtual_token_reserve + tokens_in
new_sol_reserve = k / new_token_reserve
sol_out = virtual_sol_reserve - new_sol_reserve
sol_out_after_fee = sol_out * (1 - protocol_fee_bps / 10000)
```

### Price Progression

- **Starting Price**: Very low (determined by initial virtual reserves: 30 SOL / 793M tokens)
- **Price Increases**: Each buy increases the virtual SOL reserve, decreasing available tokens
- **Price Decreases**: Each sell decreases the virtual SOL reserve, increasing available tokens
- **Completion**: When virtual SOL reserve reaches ~500 SOL, curve is complete

### Virtual vs Real Reserves

- **Virtual Reserves**: Used for price calculation (x * y = k formula)
- **Real SOL Reserve**: Accumulates actual SOL from buys (used for DEX liquidity)
- **Virtual reserves** start at 30 SOL and 793M tokens
- **Real SOL** accumulates from each buy and is used when graduating to DEX

## 🏗️ Architecture

### Program Structure

```
pump-fun-clone/
├── programs/
│   └── pump-fun-clone/
│       └── src/
│           ├── lib.rs              # Program entry point
│           ├── state.rs            # Account structs (GlobalConfig, BondingCurve)
│           ├── errors.rs           # Custom error types
│           ├── constants.rs         # Constants and bonding curve math
│           └── instructions/
│               ├── mod.rs          # Instruction module exports
│               ├── initialize.rs    # Initialize global config
│               ├── create.rs        # Create token + bonding curve
│               ├── buy.rs           # Buy tokens from curve
│               ├── sell.rs          # Sell tokens to curve
│               └── complete.rs      # Complete/graduate curve
└── tests/
    └── pump-fun-clone.ts           # Integration tests
```

### Account Structure

#### GlobalConfig
- **PDA**: `[b"global_config"]`
- **Fields**:
  - `authority`: Protocol authority
  - `treasury`: Treasury PDA address
  - `protocol_fee_bps`: Fee in basis points (50 = 0.5%)
  - `creation_fee`: Token creation fee in lamports
  - `total_tokens_created`: Counter
  - `treasury_bump`: Bump seed for treasury PDA

#### BondingCurve
- **PDA**: `[b"bonding_curve", mint]`
- **Fields**:
  - `mint`: Token mint address
  - `creator`: Token creator
  - `virtual_sol_reserve`: Virtual SOL reserve for pricing
  - `virtual_token_reserve`: Virtual token reserve for pricing
  - `real_sol_reserve`: Accumulated real SOL
  - `tokens_sold`: Total tokens sold
  - `completed`: Whether curve is complete
  - `created_at`: Creation timestamp
  - `completed_at`: Completion timestamp (optional)
  - `bump`: PDA bump seed

#### Treasury
- **PDA**: `[b"treasury", global_config]`
- Receives all protocol fees and creation fees
- Accumulates SOL for DEX liquidity pool creation

### Instruction Flow

```
1. Initialize
   └─> Creates GlobalConfig and Treasury PDAs

2. Create Token
   └─> Creates Token-2022 mint with metadata
   └─> Initializes BondingCurve account
   └─> Pays creation fee to treasury

3. Buy Tokens
   └─> Transfers SOL from buyer
   └─> Calculates tokens using bonding curve
   └─> Mints tokens to buyer
   └─> Updates virtual reserves
   └─> Checks for completion

4. Sell Tokens
   └─> Burns tokens from seller
   └─> Calculates SOL using bonding curve
   └─> Transfers SOL to seller
   └─> Updates virtual reserves

5. Complete Curve
   └─> Verifies completion threshold
   └─> Marks curve as complete
   └─> Emits completion event
   └─> (In production: triggers DEX pool creation)
```

## 🚀 Installation

### Prerequisites

- Rust 1.70+
- Solana CLI 1.18+
- Anchor CLI 0.30+
- Node.js 18+ and Yarn/npm

### Setup

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd Solana-Pumpfun-Smart-Contract-Clone-1
   ```

2. **Install Anchor**
   ```bash
   cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
   avm install latest
   avm use latest
   ```

3. **Install dependencies**
   ```bash
   anchor build
   ```

4. **Install test dependencies**
   ```bash
   cd tests
   yarn install  # or npm install
   ```

## 🔨 Building & Testing

### Build the Program

```bash
anchor build
```

This will:
- Compile the Rust program
- Generate the IDL (Interface Definition Language)
- Create the program binary

### Run Tests

```bash
anchor test
```

Or run tests with verbose output:

```bash
anchor test --skip-local-validator
```

### Test on Localnet

1. **Start local validator**
   ```bash
   solana-test-validator
   ```

2. **Deploy program**
   ```bash
   anchor deploy
   ```

3. **Run tests**
   ```bash
   anchor test --skip-local-validator
   ```

### Test Coverage

The test suite includes:
- ✅ Global config initialization
- ✅ Token creation with bonding curve
- ✅ Multiple buy operations (price increase verification)
- ✅ Sell operations (price decrease verification)
- ✅ Curve completion
- ✅ Slippage protection
- ✅ Fee calculations

## 🌐 Deployment

### Devnet Deployment

1. **Set Solana CLI to devnet**
   ```bash
   solana config set --url devnet
   ```

2. **Airdrop SOL (if needed)**
   ```bash
   solana airdrop 2 <your-wallet-address>
   ```

3. **Update program ID**
   - Generate new keypair: `solana-keygen new -o target/deploy/pump_fun_clone-keypair.json`
   - Update `declare_id!` in `lib.rs`
   - Update `Anchor.toml` with new program ID

4. **Build and deploy**
   ```bash
   anchor build
   anchor deploy
   ```

### Mainnet Deployment

⚠️ **WARNING**: Only deploy to mainnet after thorough auditing and testing.

1. **Set Solana CLI to mainnet**
   ```bash
   solana config set --url mainnet-beta
   ```

2. **Build for mainnet**
   ```bash
   anchor build
   ```

3. **Deploy (requires sufficient SOL)**
   ```bash
   anchor deploy
   ```

4. **Verify deployment**
   ```bash
   solana program show <program-id>
   ```

### Post-Deployment

1. **Initialize global config**
   - Call `initialize` instruction with protocol authority
   - This sets up the GlobalConfig and Treasury PDAs

2. **Verify initialization**
   - Check GlobalConfig account exists
   - Verify treasury PDA is funded

## 📧 Contact
- Telegram: https://t.me/codiiman
- Twitter: https://x.com/codiiman_

