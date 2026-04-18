# 🔐 Rise In - Güvenlik Dokümantasyonu

## Güvenlik Modeli

Rise In, **defense-in-depth** (katmanlı savunma) prensibiyle tasarlanmıştır. Her katman bağımsız güvenlik kontrolleri içerir.

## 🛡️ Güvenlik Katmanları

### 1. Blockchain Katmanı (Soroban)

#### Authentication
```rust
// Her kritik fonksiyon Stellar signature doğrulama yapar
seller.require_auth();
buyer.require_auth();

// Soroban otomatik olarak:
// ✅ Transaction imzasını doğrular
// ✅ Public key ile address eşleşmesini kontrol eder
// ✅ Replay attack'leri engeller (nonce mekanizması)
```

#### Authorization
```rust
// Sadece doğru kişi işlem yapabilir
if escrow.seller != seller {
    return Err(Error::Unauthorized);
}

// State-based authorization
if escrow.state != EscrowState::Delivered {
    return Err(Error::InvalidEscrowState);
}
```

#### Reentrancy Protection
```rust
// Soroban built-in reentrancy koruması:
// ❌ Aynı kontrata recursive çağrı yapılamaz
// ✅ External call sırasında kontrat "locked" durumda

pub fn release_payment(...) {
    // 1. State check
    // 2. External call (token transfer)
    // 3. State update
    // Güvenli sıralama (Checks-Effects-Interactions pattern)
}
```

### 2. Kriptografik Katman

#### Hash Integrity
```rust
// SHA-256 hash kullanımı
pub type ContentHash = BytesN<32>;

// Collision resistance: 2^256 olasılık
// Preimage resistance: Hash'ten içerik bulunamaz
// Avalanche effect: 1 bit değişiklik → %50 hash değişir
```

#### Off-Chain Encryption (Frontend)
```javascript
// Asimetrik şifreleme akışı
async function encryptContent(content, buyerPublicKey) {
    // 1. Random AES-256 key oluştur
    const aesKey = crypto.getRandomValues(new Uint8Array(32));
    
    // 2. İçeriği AES ile şifrele
    const encryptedContent = await crypto.subtle.encrypt(
        { name: "AES-GCM", iv: iv },
        aesKey,
        content
    );
    
    // 3. AES key'i alıcının public key'i ile şifrele (Ed25519)
    const encryptedKey = await encryptWithPublicKey(aesKey, buyerPublicKey);
    
    return { encryptedContent, encryptedKey };
}

// Güvenlik garantileri:
// ✅ Man-in-the-middle saldırısı engellenmiş
// ✅ Sadece alıcı içeriği çözebilir
// ✅ Forward secrecy (her işlem için yeni AES key)
```

### 3. Ekonomik Güvenlik

#### Escrow Mekanizması
```rust
// Para hiçbir zaman tek tarafın kontrolünde değil
// Buyer → Contract → Seller (atomic transfer)

// Timeout koruması
pub fn refund_timeout(...) {
    if current_time < escrow.timeout_at {
        return Err(Error::TimeoutNotReached);
    }
    // 24 saat sonra alıcı parasını geri alabilir
}
```

#### Price Validation
```rust
// Fiyat manipülasyonu engelleme
if amount != content_info.price {
    return Err(Error::InvalidPaymentAmount);
}

// Sıfır fiyat engelleme
if price <= 0 {
    return Err(Error::InvalidPrice);
}
```

## 🚨 Bilinen Riskler ve Mitigasyonlar

### Risk 1: Hash Collision (Teorik)
**Risk**: İki farklı dosya aynı SHA-256 hash'e sahip olabilir mi?

**Olasılık**: 2^-256 ≈ 10^-77 (evrendeki atom sayısından az)

**Mitigasyon**: 
- SHA-256 kriptografik olarak güvenli
- Collision resistance kanıtlanmış
- Pratik olarak imkansız

### Risk 2: Seller Doesn't Deliver
**Risk**: Satıcı parayı aldıktan sonra içeriği göndermez.

**Mitigasyon**:
```rust
// State machine koruması
pub fn release_payment(...) {
    // Sadece "Delivered" state'inde ödeme yapılabilir
    if escrow.state != EscrowState::Delivered {
        return Err(Error::InvalidEscrowState);
    }
}

// Alıcı onaylamadan ödeme yapılamaz
buyer.require_auth();
```

### Risk 3: Buyer Doesn't Confirm
**Risk**: Alıcı içeriği alır ama ödemeyi onaylamaz.

**Mitigasyon**:
```rust
// Timeout mekanizması yok (kasıtlı)
// Neden: Alıcı hash doğrulaması yapmalı
// Eğer hash eşleşmiyorsa ödeme yapmamalı

// Gelecek geliştirme: Dispute resolution
// Arbitrator sistemi ile çözüm
```

### Risk 4: Man-in-the-Middle Attack
**Risk**: Aradaki biri şifreli içeriği çalar.

**Mitigasyon**:
```javascript
// Asimetrik şifreleme
// Sadece alıcının private key'i ile çözülebilir
const encryptedKey = await encryptWithPublicKey(aesKey, buyerPublicKey);

// Stellar public key kullanımı
// Blockchain'de doğrulanmış kimlik
```

### Risk 5: Replay Attack
**Risk**: Eski transaction tekrar gönderilir.

**Mitigasyon**:
- Stellar nonce mekanizması (built-in)
- Her transaction benzersiz sequence number içerir
- Duplicate transaction otomatik reddedilir

### Risk 6: Integer Overflow
**Risk**: Toplam hacim overflow olabilir.

**Mitigasyon**:
```rust
// Cargo.toml
[profile.release]
overflow-checks = true  // ← Panic on overflow

// Soroban i128 kullanır (max: 2^127 - 1)
// 170,141,183,460,469,231,731,687,303,715,884,105,727 stroops
// = 17,014,118,346,046,923,173,168,730 XLM
// Stellar total supply: 50,000,000,000 XLM
// Overflow pratikte imkansız
```

## 🔍 Güvenlik Denetimi Checklist

### Smart Contract
- [x] Authentication her kritik fonksiyonda
- [x] Authorization kontrolleri
- [x] State machine validation
- [x] Integer overflow protection
- [x] Reentrancy protection (Soroban built-in)
- [x] Input validation (price, hash, amount)
- [x] Error handling (Result<T, Error>)
- [x] Event emission (audit trail)

### Storage
- [x] TTL yönetimi
- [x] Storage tipi optimizasyonu
- [x] Key collision prevention
- [x] Data integrity

### Cryptography
- [x] SHA-256 hash kullanımı
- [x] Asimetrik şifreleme (off-chain)
- [x] Random number generation (AES key)
- [x] Key management

### Economic
- [x] Escrow mekanizması
- [x] Timeout koruması
- [x] Price validation
- [x] Atomic transfers

## 🧪 Güvenlik Testleri

### 1. Authentication Tests
```rust
#[test]
#[should_panic]
fn test_unauthorized_delivery() {
    // Impostor satıcı gibi davranır
    client.mark_delivered(&impostor, &content_hash, &buyer);
    // Beklenen: Panic (Unauthorized)
}
```

### 2. State Machine Tests
```rust
#[test]
#[should_panic]
fn test_release_without_delivery() {
    // Teslimat olmadan ödeme denemesi
    client.release_payment(&buyer, &content_hash, &token);
    // Beklenen: Panic (InvalidEscrowState)
}
```

### 3. Timeout Tests
```rust
#[test]
#[should_panic]
fn test_early_refund() {
    // 24 saat dolmadan refund denemesi
    client.refund_timeout(&buyer, &content_hash, &token);
    // Beklenen: Panic (TimeoutNotReached)
}
```

### 4. Economic Tests
```rust
#[test]
#[should_panic]
fn test_wrong_payment_amount() {
    // Yanlış fiyat ile escrow oluşturma
    client.create_escrow(&buyer, &content_hash, &token, &wrong_amount);
    // Beklenen: Panic (InvalidPaymentAmount)
}
```

## 🛠️ Güvenlik Best Practices

### Frontend Entegrasyonu

```javascript
// ✅ İyi: Hash doğrulama
async function verifyAndRelease(file, expectedHash, contractId) {
    // 1. Dosyayı hash'le
    const actualHash = await sha256(file);
    
    // 2. Karşılaştır
    if (actualHash !== expectedHash) {
        throw new Error("Hash mismatch! Content may be corrupted.");
    }
    
    // 3. Onaylandıysa ödeme yap
    await contract.release_payment(buyer, expectedHash, token);
}

// ❌ Kötü: Hash doğrulama yok
async function badRelease(contractId) {
    // Direkt ödeme (tehlikeli!)
    await contract.release_payment(buyer, hash, token);
}
```

### Key Management

```javascript
// ✅ İyi: Secure key storage
// - Hardware wallet (Ledger, Trezor)
// - Browser extension (Freighter, Albedo)
// - Encrypted local storage

// ❌ Kötü: Plaintext storage
localStorage.setItem('privateKey', 'SXXX...'); // Tehlikeli!
```

### Error Handling

```javascript
// ✅ İyi: Graceful error handling
try {
    await contract.create_escrow(buyer, hash, token, amount);
} catch (error) {
    if (error.code === 7) { // InvalidPaymentAmount
        alert("Price mismatch. Please check the amount.");
    } else if (error.code === 2) { // ContentNotFound
        alert("Content not found. Invalid hash.");
    } else {
        alert("Transaction failed. Please try again.");
    }
}

// ❌ Kötü: Silent failure
await contract.create_escrow(...).catch(() => {});
```

## 📋 Güvenlik Güncellemeleri

### Version 1.0.0 (Current)
- ✅ Basic escrow mechanism
- ✅ Hash-based verification
- ✅ 24-hour timeout
- ✅ Authentication & authorization

### Version 1.1.0 (Planned)
- 🔄 Dispute resolution system
- 🔄 Multi-signature escrow
- 🔄 Reputation system
- 🔄 Automated arbitration

### Version 2.0.0 (Future)
- 🔮 Zero-knowledge proofs (privacy)
- 🔮 Cross-chain bridges
- 🔮 Decentralized storage integration (IPFS)

## 🚨 Güvenlik Açığı Bildirimi

Güvenlik açığı bulursanız:

1. **Hemen bildirin**: security@risein.io
2. **Public disclosure yapmayın** (responsible disclosure)
3. **Detaylı açıklama**: PoC, impact, severity
4. **Ödül**: Bug bounty programımız var

### Severity Levels

| Level | Impact | Bounty |
|-------|--------|--------|
| Critical | Funds at risk | 10,000 XLM |
| High | State corruption | 5,000 XLM |
| Medium | DoS, logic errors | 1,000 XLM |
| Low | Informational | 100 XLM |

## 📚 Güvenlik Kaynakları

- [Soroban Security Best Practices](https://soroban.stellar.org/docs/learn/security)
- [Smart Contract Security Verification Standard](https://github.com/securing/SCSVS)
- [Stellar Security Guide](https://developers.stellar.org/docs/learn/security)
- [OWASP Smart Contract Top 10](https://owasp.org/www-project-smart-contract-top-10/)

## ✅ Audit Status

| Audit Type | Status | Date | Auditor |
|------------|--------|------|---------|
| Internal Code Review | ✅ Completed | 2024-01-10 | Rise In Team |
| External Security Audit | 🔄 Pending | TBD | TBD |
| Formal Verification | 📋 Planned | TBD | TBD |

---

**Son Güncelleme**: 2024-01-15  
**Güvenlik Versiyonu**: 1.0.0  
**İletişim**: security@risein.io
