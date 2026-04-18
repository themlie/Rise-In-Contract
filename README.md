# Rise In - Secure Digital Content Marketplace

A smart contract running on Stellar Soroban that enables the secure sale of digital content through a hash-based content verification and escrow system.

## 🎯 Project Overview

Rise In is a blockchain solution that provides security and transparency in the sale of digital content (code, documents, data). Files are never uploaded to the blockchain; only their SHA-256 hashes are recorded. This ensures:- ✅ **Privacy**: Content is not visible on the blockchain
- ✅ **Verification**: Content integrity is guaranteed via hashing
- ✅ **Security**: Payment protection through an escrow system
- ✅ **Transparency**: All transactions are recorded on the blockchain

## 💻 User Interface (Frontend) Flow

Based on the web application interface, the seamless user journey involves:

### 1. Register Content (Seller)

Sellers select a file locally (e.g., Ders5.pdf). The app generates a SHA-256 hash locally—meaning the file never leaves the user's computer—and registers the hash on-chain with a set price (e.g., 1 XLM) and description.

<p align="center">
  <img src="screenshots/01-register-content.png" alt="Register Content" width="800"/>
</p>

After registration, the content hash is generated and stored on-chain:

<p align="center">
  <img src="screenshots/02-content-registered.png" alt="Content Registered" width="800"/>
</p>

### 2. Browse & Buy (Buyer)

Buyers look up a content hash in the marketplace, view the details (description, price, seller), and click "Buy".

<p align="center">
  <img src="screenshots/03-browse-content.png" alt="Browse Content" width="800"/>
</p>

This prompts a Freighter wallet confirmation to lock the funds in a smart contract escrow:

<p align="center">
  <img src="screenshots/04-wallet-confirmation.png" alt="Wallet Confirmation" width="400"/>
</p>

After confirmation, the escrow is created and funds are locked:

<p align="center">
  <img src="screenshots/05-escrow-created.png" alt="Escrow Created" width="800"/>
</p>

### 3. Mark as Delivered (Seller)

The seller shares the encrypted file off-chain (e.g., via Google Drive, Dropbox) and marks the escrow as delivered in the dashboard.

<p align="center">
  <img src="screenshots/06-mark-delivered.png" alt="Mark Delivered" width="800"/>
</p>

### 4. Verify & Release Payment (Buyer)

The buyer drags and drops the received file into the app. The app hashes it locally, verifies that it matches the original on-chain hash, and securely releases the payment to the seller.

<p align="center">
  <img src="screenshots/07-verify-release.png" alt="Verify and Release" width="800"/>
</p>

## 🔐 Security Features

### 1. Hash-Based Verification```
Seller: File → SHA-256 → abc123hash... → Save to Blockchain
Buyer: Receives file → SHA-256 → Compare → If matches, approve
```

### 2. Escrow Protection- Funds are neither with the buyer nor the seller → Controlled by the Contract
- The seller cannot receive the funds until the buyer approves
- If delivery is not made within 24 hours, the buyer can get a refund

### 3. Asymmetric Encryption (Off-Chain)```
Seller:
1. Encrypts content with AES-256
2. Encrypts the AES key with the buyer's Ed25519 public key
3. Sends encrypted content + encrypted key → To the buyer

Buyer:
1. Decrypts the key using their own private key
2. Decrypts the content using the AES key
3. Verifies the hash
4. Approves the payment

Result: No man-in-the-middle can steal the content
```

## 📊 System Flow

```mermaid
sequenceDiagram
    participant S as Seller
    participant C as Contract
    participant B as Buyer

    S->>C: 1. register_content(hash, price)
    Note over C: Hash registered
    
    B->>C: 2. create_escrow(hash, 100 XLM)
    Note over C: Funds locked
    
    S->>B: 3. Send content (encrypted off-chain)
    S->>C: 4. mark_delivered()
    
    B->>B: 5. Verify hash locally
    B->>C: 6. release_payment()
    C->>S: 7. 100 XLM transferred
    
    Note over C: Transaction completed
```

### Alternative Flow: Timeout```
If the seller does not deliver within 24 hours:
Buyer → refund_timeout() → Funds are refunded
```

## 🏗️ Technical Architecture

### Data Structures#### ContentInfo (Persistent Storage)

```ruststruct ContentInfo {
    seller: Address,           // Seller's address
    content_hash: BytesN<32>,  // SHA-256 hash
    price: i128,               // Price (in stroops)
    description: String,       // Content description
    registered_at: u64,        // Registration timestamp
}
```

#### EscrowAgreement (Persistent Storage)

```ruststruct EscrowAgreement {
    content_hash: BytesN<32>,
    seller: Address,
    buyer: Address,
    amount: i128,
    state: EscrowState,        // Locked/Delivered/Completed/Refunded
    created_at: u64,
    timeout_at: u64,           // created_at + 24 hours
    delivered_at: Option<u64>,
}
```

#### ContractStats (Instance Storage)

```ruststruct ContractStats {
    total_contents: u64,       // Total registered content
    total_escrows: u64,        // Total number of escrows
    total_completed: u64,      // Completed transactions
    total_volume: i128,        // Total transaction volume
}
```

### Storage Strategy| Data Type | Storage Type | TTL | Reason |
|-----------|-------------|-----|--------|
| ContentInfo | Persistent | 1 year | Critical record, needs to be stored long-term |
| EscrowAgreement | Persistent | 60 days | Financial data, required post-transaction |
| ContractStats | Instance | 60 days | Global state, cheaper to maintain |

**Optimization**: Instance storage is ~70% cheaper than Persistent storage.

## 🔧 Core Functions

### 1. Content Registration

```rustfn register_content(
    seller: Address,
    content_hash: BytesN<32>,
    price: i128,
    description: String
) -> Result<(), Error>
```

**Security Checks:**
- ✅ Only the seller can register their content (`seller.require_auth()`)
- ✅ Hash must be unique (duplicate check)
- ✅ Price must be > 0

### 2. Escrow Creation

```rustfn create_escrow(
    buyer: Address,
    content_hash: BytesN<32>,
    token: Address,
    amount: i128
) -> Result<(), Error>
```

**Security Checks:**
- ✅ Buyer authentication
- ✅ Price validation (amount == content.price)
- ✅ Token transfer (buyer → contract)
- ✅ Sets a 24-hour timeout

### 3. Mark as Delivered

```rustfn mark_delivered(
    seller: Address,
    content_hash: BytesN<32>,
    buyer: Address
) -> Result<(), Error>
```

**Security Checks:**
- ✅ Only the seller can mark it
- ✅ Escrow must be in the Locked state

### 4. Payment Release

```rustfn release_payment(
    buyer: Address,
    content_hash: BytesN<32>,
    token: Address
) -> Result<(), Error>
```

**Security Checks:**
- ✅ Only the buyer can approve
- ✅ Escrow must be in the Delivered state
- ✅ Token transfer (contract → seller)

### 5. Timeout Refund

```rustfn refund_timeout(
    buyer: Address,
    content_hash: BytesN<32>,
    token: Address
) -> Result<(), Error>
```

**Security Checks:**
- ✅ 24 hours must have passed
- ✅ Escrow must be in the Locked state (no delivery made)
- ✅ Token transfer (contract → buyer)

## 🧪 Testing Strategy

### Unit Tests

#### 1. Content Registration Tests- ✅ `test_register_content_success`: Successful registration
- ✅ `test_register_content_duplicate`: Duplicate hash check
- ✅ `test_register_content_invalid_price`: Invalid price check

#### 2. Escrow Creation Tests- ✅ `test_create_escrow_success`: Successful escrow creation
- ✅ `test_create_escrow_content_not_found`: Content does not exist
- ✅ `test_create_escrow_wrong_amount`: Incorrect payment amount

#### 3. Delivery Tests- ✅ `test_mark_delivered_success`: Successful delivery marking
- ✅ `test_mark_delivered_unauthorized`: Unauthorized marking attempt

#### 4. Payment Release Tests- ✅ `test_release_payment_success`: Successful payment
- ✅ `test_release_payment_not_delivered`: Attempted payment without delivery

#### 5. Timeout Tests- ✅ `test_refund_timeout_success`: Successful refund
- ✅ `test_refund_timeout_too_early`: Premature refund attempt
- ✅ `test_refund_after_delivery`: Refund attempt after delivery

#### 6. Integration Tests- ✅ `test_multiple_buyers_same_content`: Multiple buyers for one item
- ✅ `test_complete_happy_path`: End-to-end flow test

### Running Tests

```bashcd rise-in-contract
cargo test
```

### Test Coverage

```bashcargo tarpaulin --out Html
```

## 🚀 Installation & Build

### Prerequisites

```bash# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Soroban CLI
cargo install --locked soroban-cli

# Add WASM target
rustup target add wasm32-unknown-unknown
```

### Building

```bashcd rise-in-contract

# Optimized build
cargo build --target wasm32-unknown-unknown --release

# Optimize for Soroban
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/rise_in_contract.wasm
```

### Deployment (Testnet)

```bash# Deploy Contract
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/rise_in_contract.optimized.wasm \
  --source SELLER_SECRET_KEY \
  --network testnet

# Output Contract ID:
# CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

## 📝 CLI Usage Example

*(Note: These actions can also be performed via the web UI)*

### 1. Register Content (Seller)

```bash# Hash the file locally
sha256sum my_code.zip
# Output: abc123def456... my_code.zip

# Register on contract
soroban contract invoke \
  --id CONTRACT_ID \
  --source SELLER_SECRET \
  --network testnet \
  -- register_content \
  --seller SELLER_ADDRESS \
  --content_hash abc123def456... \
  --price 1000000000 \
  --description "Premium Code Package"
```

### 2. Purchase (Buyer)

```bashsoroban contract invoke \
  --id CONTRACT_ID \
  --source BUYER_SECRET \
  --network testnet \
  -- create_escrow \
  --buyer BUYER_ADDRESS \
  --content_hash abc123def456... \
  --token NATIVE_TOKEN_ADDRESS \
  --amount 1000000000
```

### 3. Delivery (Seller)

```bash# Off-chain: Encrypt and send content
# Example: openssl enc -aes-256-cbc -in file.zip -out file.enc

# On-chain: Mark as delivered
soroban contract invoke \
  --id CONTRACT_ID \
  --source SELLER_SECRET \
  --network testnet \
  -- mark_delivered \
  --seller SELLER_ADDRESS \
  --content_hash abc123def456... \
  --buyer BUYER_ADDRESS
```

### 4. Approval (Buyer)

```bash# Off-chain: Verify hash
sha256sum received_file.zip
# Does it match? Yes → Approve

# On-chain: Release payment
soroban contract invoke \
  --id CONTRACT_ID \
  --source BUYER_SECRET \
  --network testnet \
  -- release_payment \
  --buyer BUYER_ADDRESS \
  --content_hash abc123def456... \
  --token NATIVE_TOKEN_ADDRESS
```

## 🎓 Learning ObjectivesThrough this project, you will learn:

### Soroban Fundamentals
- Contract structure and lifecycle
- Storage types (Persistent/Instance/Temporary)
- TTL management

### Security Principles
- Authentication (`require_auth()`)
- State machine patterns
- Reentrancy protection (built-in for Soroban)

### Economic Design
- Escrow patterns
- Timeout mechanisms
- Gas optimization

### Test-Driven Development (TDD)
- Unit testing
- Integration testing
- Edge case handling

## 🔍 Advanced Topics

### Gas Optimization

```rustRust// ❌ Bad: Reading from storage multiple times
for i in 0..10 {
    let data = Storage::get_data(&env);
    // ...
}

// ✅ Good: Read once, keep in memory
let data = Storage::get_data(&env);
for i in 0..10 {
    // use data
}
```

### Event Emission

```rustRustenv.events().publish(
    (String::from_str(&env, "payment_released"), buyer),
    (content_hash, seller, amount),
);
```

The frontend can listen to these events to send real-time notifications.

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing`)
5. Open a Pull Request

## 📄 License

MIT License - See the LICENSE file for details.

## 🙏 Acknowledgements

- Stellar Development Foundation
- Soroban Documentation
- Rise In Community

## 📞 Contact

For any questions:
- **GitHub Issues**: [rise-in-contract/issues](https://github.com/rise-in-contract/issues)
- **Discord**: Stellar Developers

---

**Note**: This project is for educational purposes. A professional security audit should be conducted before using it in production.