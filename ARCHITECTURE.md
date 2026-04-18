# Rise In - Teknik Mimari Dokümantasyonu

## 📐 Sistem Mimarisi

### Katmanlı Yapı

```
┌─────────────────────────────────────────┐
│         Frontend (React)                │
│  - Hash hesaplama (SHA-256)             │
│  - Şifreleme (AES-256 + Ed25519)        │
│  - Soroban RPC iletişimi                │
└─────────────────────────────────────────┘
                  ↕ RPC
┌─────────────────────────────────────────┐
│      Soroban Smart Contract             │
│  ┌───────────────────────────────────┐  │
│  │  Contract Layer (contract.rs)     │  │
│  │  - İş mantığı                     │  │
│  │  - Yetkilendirme                  │  │
│  │  - State yönetimi                 │  │
│  └───────────────────────────────────┘  │
│  ┌───────────────────────────────────┐  │
│  │  Storage Layer (storage.rs)       │  │
│  │  - Persistent storage             │  │
│  │  - Instance storage               │  │
│  │  - TTL yönetimi                   │  │
│  └───────────────────────────────────┘  │
│  ┌───────────────────────────────────┐  │
│  │  Types Layer (types.rs)           │  │
│  │  - Veri yapıları                  │  │
│  │  - Enum'lar                       │  │
│  └───────────────────────────────────┘  │
│  ┌───────────────────────────────────┐  │
│  │  Error Layer (errors.rs)          │  │
│  │  - Hata kodları                   │  │
│  │  - Hata yönetimi                  │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
                  ↕
┌─────────────────────────────────────────┐
│      Stellar Blockchain                 │
│  - Ledger storage                       │
│  - Token transfers                      │
│  - Event emission                       │
└─────────────────────────────────────────┘
```

## 🔄 State Machine

### Escrow State Transitions

```
                    create_escrow()
        ┌──────────────────────────────────┐
        │                                  │
        ▼                                  │
    ┌────────┐                             │
    │ Locked │                             │
    └────────┘                             │
        │                                  │
        │ mark_delivered()                 │
        ▼                                  │
    ┌───────────┐                          │
    │ Delivered │                          │
    └───────────┘                          │
        │                                  │
        ├─────────────────┬────────────────┤
        │                 │                │
        │ release_payment()│ refund_timeout()
        │                 │                │
        ▼                 ▼                │
    ┌───────────┐    ┌──────────┐         │
    │ Completed │    │ Refunded │         │
    └───────────┘    └──────────┘         │
                                           │
                    (24h timeout)          │
                    refund_timeout()       │
                    ───────────────────────┘
```

### State Validation Rules

| Current State | Allowed Transitions | Required Conditions |
|--------------|---------------------|---------------------|
| Locked | → Delivered | Seller auth + timeout not reached |
| Locked | → Refunded | Buyer auth + timeout reached |
| Delivered | → Completed | Buyer auth |
| Completed | (final) | - |
| Refunded | (final) | - |

## 💾 Storage Detayları

### Storage Tipi Seçim Kriterleri

```rust
// Persistent Storage: Kritik, uzun süreli veri
// - Escrow agreements (finansal veri)
// - Content registrations (hash kayıtları)
// Maliyet: Yüksek, TTL: Uzun (1 yıl+)

// Instance Storage: Global durum, sık erişilen veri
// - Contract statistics
// - Configuration
// Maliyet: Orta, TTL: Orta (60 gün)

// Temporary Storage: Geçici, kısa süreli veri
// - Rate limiting counters
// - Temporary locks
// Maliyet: Düşük, TTL: Kısa (1 gün)
```

### Storage Key Patterns

```rust
// Content Key Pattern
("CONTENT", content_hash) → ContentInfo

// Escrow Key Pattern
("ESCROW", content_hash, buyer_address) → EscrowAgreement

// Stats Key Pattern
"STATS" → ContractStats
```

### TTL Stratejisi

```rust
// Content: 1 yıl (5,184,000 ledgers)
// Neden: İçerik kayıtları uzun süre saklanmalı
env.storage().persistent().extend_ttl(&key, 5184000, 5184000);

// Escrow: 60 gün (1,036,800 ledgers)
// Neden: İşlem 24 saatte biter, ama kayıt tutulmalı
env.storage().persistent().extend_ttl(&key, 1036800, 1036800);

// Stats: 60 gün (1,036,800 ledgers)
// Neden: İstatistikler sık güncellenir
env.storage().instance().extend_ttl(1036800, 1036800);
```

## 🔐 Güvenlik Modeli

### 1. Authentication Flow

```rust
// Her kritik fonksiyon kimlik doğrulama yapar
pub fn register_content(env: Env, seller: Address, ...) {
    seller.require_auth(); // ← Stellar signature doğrulama
    // ...
}

// Soroban otomatik olarak:
// 1. Transaction'ı imzalayan adresi kontrol eder
// 2. İmza geçerliliğini doğrular
// 3. Geçersizse panic atar
```

### 2. Authorization Matrix

| Function | Caller | Auth Check | Additional Checks |
|----------|--------|------------|-------------------|
| register_content | Seller | ✅ seller.require_auth() | Price > 0, Hash unique |
| create_escrow | Buyer | ✅ buyer.require_auth() | Amount == price, Content exists |
| mark_delivered | Seller | ✅ seller.require_auth() | Escrow.seller == caller |
| release_payment | Buyer | ✅ buyer.require_auth() | Escrow.buyer == caller, State == Delivered |
| refund_timeout | Buyer | ✅ buyer.require_auth() | Timeout reached, State == Locked |

### 3. Reentrancy Protection

Soroban'da reentrancy saldırıları **built-in olarak engellenmiştir**:

```rust
// ❌ Ethereum'da tehlikeli:
function withdraw() {
    uint amount = balances[msg.sender];
    msg.sender.call.value(amount)(""); // ← Reentrancy riski
    balances[msg.sender] = 0;
}

// ✅ Soroban'da güvenli:
pub fn release_payment(...) {
    // 1. State kontrolü
    if escrow.state != EscrowState::Delivered {
        return Err(Error::InvalidEscrowState);
    }
    
    // 2. Token transfer (external call)
    token_client.transfer(&contract, &seller, &amount);
    
    // 3. State güncelleme
    escrow.state = EscrowState::Completed;
    
    // Soroban, external call sırasında kontratı "lock" eder
    // Aynı kontrata tekrar çağrı yapılamaz
}
```

### 4. Integer Overflow Protection

```rust
// Cargo.toml'da overflow kontrolü aktif:
[profile.release]
overflow-checks = true  // ← Panic on overflow

// Örnek:
let total_volume = stats.total_volume + amount;
// Eğer overflow olursa → Panic → Transaction revert
```

## 🎯 Optimizasyon Teknikleri

### 1. Storage Okuma Optimizasyonu

```rust
// ❌ Kötü: Çoklu okuma
pub fn bad_function(env: Env) {
    for i in 0..10 {
        let stats = Storage::get_stats(&env); // 10 kez okuma
        // ...
    }
}

// ✅ İyi: Tek okuma
pub fn good_function(env: Env) {
    let stats = Storage::get_stats(&env); // 1 kez okuma
    for i in 0..10 {
        // stats kullan
    }
}
```

### 2. Storage Yazma Optimizasyonu

```rust
// ❌ Kötü: Her değişiklikte yazma
pub fn bad_update(env: Env) {
    let mut stats = Storage::get_stats(&env);
    stats.total_contents += 1;
    Storage::save_stats(&env, &stats); // Yazma 1
    
    stats.total_escrows += 1;
    Storage::save_stats(&env, &stats); // Yazma 2
}

// ✅ İyi: Toplu yazma
pub fn good_update(env: Env) {
    let mut stats = Storage::get_stats(&env);
    stats.total_contents += 1;
    stats.total_escrows += 1;
    Storage::save_stats(&env, &stats); // Tek yazma
}
```

### 3. Event Emission

```rust
// Event'ler off-chain indexing için kritik
env.events().publish(
    (topic1, topic2),  // ← Indexed (filtrelenebilir)
    data               // ← Non-indexed (payload)
);

// Örnek:
env.events().publish(
    (String::from_str(&env, "payment_released"), buyer),
    (content_hash, seller, amount),
);

// Frontend'de dinleme:
// soroban.events.filter({ topic: "payment_released", buyer: "GXXX..." })
```

## 🧪 Test Mimarisi

### Test Piramidi

```
        ┌─────────────┐
        │ Integration │  ← 2 test (happy path, multi-buyer)
        │   Tests     │
        └─────────────┘
       ┌───────────────┐
       │  Functional   │  ← 8 test (delivery, payment, refund)
       │    Tests      │
       └───────────────┘
      ┌─────────────────┐
      │   Unit Tests    │  ← 15 test (registration, escrow, errors)
      └─────────────────┘
```

### Test Kategorileri

#### 1. Positive Tests (Happy Path)
```rust
#[test]
fn test_complete_happy_path() {
    // Register → Escrow → Deliver → Release
    // Tüm adımlar başarılı
}
```

#### 2. Negative Tests (Error Cases)
```rust
#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_register_content_duplicate() {
    // Duplicate hash → AlreadyRegistered error
}
```

#### 3. Edge Cases
```rust
#[test]
fn test_refund_timeout_boundary() {
    // Tam 24 saat sonra refund
    // Boundary condition test
}
```

#### 4. Integration Tests
```rust
#[test]
fn test_multiple_buyers_same_content() {
    // Aynı içerik, farklı alıcılar
    // Concurrent escrow test
}
```

### Mock Stratejisi

```rust
// Soroban testutils kullanımı
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};

// Mock authentication
env.mock_all_auths(); // Tüm require_auth() çağrıları geçer

// Mock time
env.ledger().set(LedgerInfo {
    timestamp: current_time + 24 * 60 * 60, // 24 saat ileri
    // ...
});

// Mock token
let token = env.register_stellar_asset_contract(admin);
```

## 🚀 Deployment Stratejisi

### 1. Build Pipeline

```bash
# 1. Lint
cargo clippy -- -D warnings

# 2. Format check
cargo fmt -- --check

# 3. Test
cargo test

# 4. Build
cargo build --target wasm32-unknown-unknown --release

# 5. Optimize
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/rise_in_contract.wasm

# 6. Size check
ls -lh target/wasm32-unknown-unknown/release/*.wasm
```

### 2. Deployment Environments

```
Development (Local)
    ↓
Testnet (Stellar Testnet)
    ↓
Staging (Futurenet)
    ↓
Production (Mainnet)
```

### 3. Contract Versioning

```rust
// Contract metadata
const VERSION: &str = "1.0.0";
const AUTHOR: &str = "Rise In Team";

// Upgrade stratejisi:
// Soroban'da contract upgrade için:
// 1. Yeni contract deploy et
// 2. Frontend'i yeni contract ID'ye yönlendir
// 3. Eski contract'ı deprecate et (TTL expire)
```

## 📊 Gas Analizi

### Fonksiyon Maliyetleri (Tahmini)

| Function | Storage Reads | Storage Writes | Token Transfers | Estimated Cost |
|----------|---------------|----------------|-----------------|----------------|
| register_content | 1 (check) | 2 (content + stats) | 0 | ~5,000 stroops |
| create_escrow | 2 (content + check) | 2 (escrow + stats) | 1 (buyer→contract) | ~15,000 stroops |
| mark_delivered | 1 (escrow) | 1 (escrow) | 0 | ~3,000 stroops |
| release_payment | 1 (escrow) | 2 (escrow + stats) | 1 (contract→seller) | ~15,000 stroops |
| refund_timeout | 1 (escrow) | 1 (escrow) | 1 (contract→buyer) | ~15,000 stroops |

**Not**: Gerçek maliyetler network durumuna göre değişir.

## 🔮 Gelecek Geliştirmeler

### Phase 2: Dispute Resolution
```rust
pub fn open_dispute(
    env: Env,
    buyer: Address,
    content_hash: ContentHash,
    reason: String,
) -> Result<(), Error>

// Arbitrator sistemi
// Voting mechanism
// Reputation system
```

### Phase 3: Multi-Asset Support
```rust
pub struct ContentInfo {
    // ...
    accepted_tokens: Vec<Address>, // XLM, USDC, custom tokens
}
```

### Phase 4: Subscription Model
```rust
pub struct Subscription {
    content_hash: ContentHash,
    subscriber: Address,
    expires_at: u64,
    auto_renew: bool,
}
```

## 📚 Referanslar

- [Soroban Documentation](https://soroban.stellar.org/docs)
- [Stellar Asset Contract](https://soroban.stellar.org/docs/reference/contracts/token-interface)
- [Storage Best Practices](https://soroban.stellar.org/docs/learn/storage)
- [Security Guidelines](https://soroban.stellar.org/docs/learn/security)

---

**Güncellenme**: 2024-01-15  
**Versiyon**: 1.0.0  
**Yazar**: Rise In Development Team
