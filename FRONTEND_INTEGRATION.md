# 🎨 Rise In - Frontend Entegrasyon Rehberi

## React + Stellar SDK Entegrasyonu

Bu dokümanda Rise In smart contract'ını React frontend'e nasıl entegre edeceğinizi öğreneceksiniz.

## 📦 Gerekli Paketler

```bash
npm install @stellar/stellar-sdk
npm install @stellar/freighter-api  # Wallet entegrasyonu
npm install crypto-js               # Hash hesaplama
```

## 🔧 Temel Kurulum

### 1. Stellar Client Konfigürasyonu

```typescript
// src/lib/stellar.ts
import { SorobanRpc, Contract, Networks } from '@stellar/stellar-sdk';

// Network konfigürasyonu
export const NETWORK = {
  testnet: {
    rpcUrl: 'https://soroban-testnet.stellar.org',
    networkPassphrase: Networks.TESTNET,
  },
  mainnet: {
    rpcUrl: 'https://soroban-mainnet.stellar.org',
    networkPassphrase: Networks.PUBLIC,
  },
};

// RPC client oluştur
export const server = new SorobanRpc.Server(NETWORK.testnet.rpcUrl);

// Contract ID (deploy sonrası alınır)
export const CONTRACT_ID = 'CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX';

// Contract client
export const contract = new Contract(CONTRACT_ID);
```

### 2. Wallet Bağlantısı (Freighter)

```typescript
// src/hooks/useWallet.ts
import { useState, useEffect } from 'react';
import freighter from '@stellar/freighter-api';

export function useWallet() {
  const [publicKey, setPublicKey] = useState<string | null>(null);
  const [isConnected, setIsConnected] = useState(false);

  // Wallet bağlantısı
  const connect = async () => {
    try {
      // Freighter yüklü mü kontrol et
      const isAvailable = await freighter.isConnected();
      if (!isAvailable) {
        throw new Error('Freighter wallet not installed');
      }

      // Public key al
      const { publicKey } = await freighter.getPublicKey();
      setPublicKey(publicKey);
      setIsConnected(true);
      
      return publicKey;
    } catch (error) {
      console.error('Wallet connection failed:', error);
      throw error;
    }
  };

  // Wallet bağlantısını kes
  const disconnect = () => {
    setPublicKey(null);
    setIsConnected(false);
  };

  return { publicKey, isConnected, connect, disconnect };
}
```

### 3. Hash Hesaplama

```typescript
// src/lib/crypto.ts
import CryptoJS from 'crypto-js';

/**
 * Dosyayı SHA-256 ile hash'le
 */
export async function hashFile(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    
    reader.onload = (e) => {
      const arrayBuffer = e.target?.result as ArrayBuffer;
      const wordArray = CryptoJS.lib.WordArray.create(arrayBuffer);
      const hash = CryptoJS.SHA256(wordArray);
      resolve(hash.toString(CryptoJS.enc.Hex));
    };
    
    reader.onerror = reject;
    reader.readAsArrayBuffer(file);
  });
}

/**
 * String'i SHA-256 ile hash'le
 */
export function hashString(content: string): string {
  return CryptoJS.SHA256(content).toString(CryptoJS.enc.Hex);
}

/**
 * Hash'i Stellar BytesN<32> formatına çevir
 */
export function hashToBytes32(hash: string): Buffer {
  return Buffer.from(hash, 'hex');
}
```

### 4. Asimetrik Şifreleme

```typescript
// src/lib/encryption.ts
import { Keypair } from '@stellar/stellar-sdk';

/**
 * İçeriği AES-256 ile şifrele
 */
export async function encryptContent(
  content: Uint8Array,
  buyerPublicKey: string
): Promise<{ encryptedContent: Uint8Array; encryptedKey: Uint8Array }> {
  // 1. Random AES-256 key oluştur
  const aesKey = crypto.getRandomValues(new Uint8Array(32));
  const iv = crypto.getRandomValues(new Uint8Array(12));

  // 2. İçeriği AES-GCM ile şifrele
  const cryptoKey = await crypto.subtle.importKey(
    'raw',
    aesKey,
    { name: 'AES-GCM' },
    false,
    ['encrypt']
  );

  const encryptedContent = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    cryptoKey,
    content
  );

  // 3. AES key'i alıcının public key'i ile şifrele
  // Not: Stellar Ed25519 kullanır, X25519'a dönüştürme gerekir
  const encryptedKey = await encryptAESKey(aesKey, buyerPublicKey);

  return {
    encryptedContent: new Uint8Array(encryptedContent),
    encryptedKey,
  };
}

/**
 * Şifreli içeriği çöz
 */
export async function decryptContent(
  encryptedContent: Uint8Array,
  encryptedKey: Uint8Array,
  privateKey: string
): Promise<Uint8Array> {
  // 1. AES key'i private key ile çöz
  const aesKey = await decryptAESKey(encryptedKey, privateKey);

  // 2. İçeriği AES ile çöz
  const cryptoKey = await crypto.subtle.importKey(
    'raw',
    aesKey,
    { name: 'AES-GCM' },
    false,
    ['decrypt']
  );

  const iv = encryptedContent.slice(0, 12); // IV başta
  const ciphertext = encryptedContent.slice(12);

  const decryptedContent = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv },
    cryptoKey,
    ciphertext
  );

  return new Uint8Array(decryptedContent);
}
```

## 🎯 Contract Fonksiyonları

### 1. İçerik Kaydı (Seller)

```typescript
// src/lib/contract.ts
import {
  SorobanRpc,
  TransactionBuilder,
  Operation,
  Asset,
  Address,
  xdr,
} from '@stellar/stellar-sdk';
import { server, contract, NETWORK } from './stellar';
import freighter from '@stellar/freighter-api';

/**
 * İçerik kaydet
 */
export async function registerContent(
  sellerPublicKey: string,
  contentHash: string,
  price: string, // XLM cinsinden (örn: "100")
  description: string
): Promise<string> {
  try {
    // 1. Account bilgisi al
    const account = await server.getAccount(sellerPublicKey);

    // 2. Fiyatı stroops'a çevir (1 XLM = 10^7 stroops)
    const priceInStroops = BigInt(parseFloat(price) * 10_000_000);

    // 3. Contract fonksiyonunu çağır
    const operation = contract.call(
      'register_content',
      new Address(sellerPublicKey).toScVal(),
      xdr.ScVal.scvBytes(Buffer.from(contentHash, 'hex')),
      xdr.ScVal.scvI128(
        new xdr.Int128Parts({
          lo: xdr.Uint64.fromString((priceInStroops & BigInt(0xFFFFFFFFFFFFFFFF)).toString()),
          hi: xdr.Int64.fromString((priceInStroops >> BigInt(64)).toString()),
        })
      ),
      xdr.ScVal.scvString(description)
    );

    // 4. Transaction oluştur
    const transaction = new TransactionBuilder(account, {
      fee: '100000',
      networkPassphrase: NETWORK.testnet.networkPassphrase,
    })
      .addOperation(operation)
      .setTimeout(30)
      .build();

    // 5. Transaction'ı simüle et
    const simulated = await server.simulateTransaction(transaction);
    
    if (SorobanRpc.Api.isSimulationError(simulated)) {
      throw new Error(`Simulation failed: ${simulated.error}`);
    }

    // 6. Transaction'ı hazırla
    const prepared = SorobanRpc.assembleTransaction(transaction, simulated).build();

    // 7. Freighter ile imzala
    const signedXDR = await freighter.signTransaction(prepared.toXDR(), {
      networkPassphrase: NETWORK.testnet.networkPassphrase,
    });

    // 8. Gönder
    const tx = TransactionBuilder.fromXDR(signedXDR, NETWORK.testnet.networkPassphrase);
    const result = await server.sendTransaction(tx);

    // 9. Sonucu bekle
    let status = await server.getTransaction(result.hash);
    while (status.status === 'NOT_FOUND') {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      status = await server.getTransaction(result.hash);
    }

    if (status.status === 'SUCCESS') {
      return result.hash;
    } else {
      throw new Error(`Transaction failed: ${status.status}`);
    }
  } catch (error) {
    console.error('Register content failed:', error);
    throw error;
  }
}
```

### 2. Escrow Oluşturma (Buyer)

```typescript
/**
 * Escrow oluştur ve ödeme kilitle
 */
export async function createEscrow(
  buyerPublicKey: string,
  contentHash: string,
  tokenAddress: string, // Native XLM için: Asset.native()
  amount: string // XLM cinsinden
): Promise<string> {
  try {
    const account = await server.getAccount(buyerPublicKey);
    const amountInStroops = BigInt(parseFloat(amount) * 10_000_000);

    const operation = contract.call(
      'create_escrow',
      new Address(buyerPublicKey).toScVal(),
      xdr.ScVal.scvBytes(Buffer.from(contentHash, 'hex')),
      new Address(tokenAddress).toScVal(),
      xdr.ScVal.scvI128(
        new xdr.Int128Parts({
          lo: xdr.Uint64.fromString((amountInStroops & BigInt(0xFFFFFFFFFFFFFFFF)).toString()),
          hi: xdr.Int64.fromString((amountInStroops >> BigInt(64)).toString()),
        })
      )
    );

    // Transaction build, sign, send (yukarıdaki gibi)
    // ...

    return transactionHash;
  } catch (error) {
    console.error('Create escrow failed:', error);
    throw error;
  }
}
```

### 3. Teslimat İşaretleme (Seller)

```typescript
/**
 * İçeriği teslim edildi olarak işaretle
 */
export async function markDelivered(
  sellerPublicKey: string,
  contentHash: string,
  buyerAddress: string
): Promise<string> {
  try {
    const account = await server.getAccount(sellerPublicKey);

    const operation = contract.call(
      'mark_delivered',
      new Address(sellerPublicKey).toScVal(),
      xdr.ScVal.scvBytes(Buffer.from(contentHash, 'hex')),
      new Address(buyerAddress).toScVal()
    );

    // Transaction build, sign, send
    // ...

    return transactionHash;
  } catch (error) {
    console.error('Mark delivered failed:', error);
    throw error;
  }
}
```

### 4. Ödeme Onayı (Buyer)

```typescript
/**
 * Hash doğrula ve ödemeyi onayla
 */
export async function releasePayment(
  buyerPublicKey: string,
  contentHash: string,
  tokenAddress: string,
  receivedFile: File
): Promise<string> {
  try {
    // 1. Dosyayı hash'le
    const actualHash = await hashFile(receivedFile);

    // 2. Hash'leri karşılaştır
    if (actualHash !== contentHash) {
      throw new Error('Hash mismatch! Content may be corrupted or tampered.');
    }

    // 3. Hash eşleşiyorsa ödemeyi onayla
    const account = await server.getAccount(buyerPublicKey);

    const operation = contract.call(
      'release_payment',
      new Address(buyerPublicKey).toScVal(),
      xdr.ScVal.scvBytes(Buffer.from(contentHash, 'hex')),
      new Address(tokenAddress).toScVal()
    );

    // Transaction build, sign, send
    // ...

    return transactionHash;
  } catch (error) {
    console.error('Release payment failed:', error);
    throw error;
  }
}
```

### 5. Timeout İadesi (Buyer)

```typescript
/**
 * 24 saat sonra iade talep et
 */
export async function refundTimeout(
  buyerPublicKey: string,
  contentHash: string,
  tokenAddress: string
): Promise<string> {
  try {
    const account = await server.getAccount(buyerPublicKey);

    const operation = contract.call(
      'refund_timeout',
      new Address(buyerPublicKey).toScVal(),
      xdr.ScVal.scvBytes(Buffer.from(contentHash, 'hex')),
      new Address(tokenAddress).toScVal()
    );

    // Transaction build, sign, send
    // ...

    return transactionHash;
  } catch (error) {
    console.error('Refund timeout failed:', error);
    throw error;
  }
}
```

## 🎨 React Bileşenleri

### 1. Seller Dashboard

```typescript
// src/components/SellerDashboard.tsx
import React, { useState } from 'react';
import { useWallet } from '../hooks/useWallet';
import { hashFile } from '../lib/crypto';
import { registerContent } from '../lib/contract';

export function SellerDashboard() {
  const { publicKey, isConnected, connect } = useWallet();
  const [file, setFile] = useState<File | null>(null);
  const [price, setPrice] = useState('');
  const [description, setDescription] = useState('');
  const [loading, setLoading] = useState(false);

  const handleRegister = async () => {
    if (!file || !publicKey) return;

    try {
      setLoading(true);

      // 1. Dosyayı hash'le
      const contentHash = await hashFile(file);
      console.log('Content hash:', contentHash);

      // 2. Kontrata kaydet
      const txHash = await registerContent(
        publicKey,
        contentHash,
        price,
        description
      );

      alert(`Content registered! Transaction: ${txHash}`);
    } catch (error) {
      console.error(error);
      alert('Registration failed!');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="seller-dashboard">
      <h2>Seller Dashboard</h2>

      {!isConnected ? (
        <button onClick={connect}>Connect Wallet</button>
      ) : (
        <div>
          <p>Connected: {publicKey}</p>

          <div className="form">
            <input
              type="file"
              onChange={(e) => setFile(e.target.files?.[0] || null)}
            />

            <input
              type="number"
              placeholder="Price (XLM)"
              value={price}
              onChange={(e) => setPrice(e.target.value)}
            />

            <input
              type="text"
              placeholder="Description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />

            <button onClick={handleRegister} disabled={loading}>
              {loading ? 'Registering...' : 'Register Content'}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
```

### 2. Buyer Dashboard

```typescript
// src/components/BuyerDashboard.tsx
import React, { useState } from 'react';
import { useWallet } from '../hooks/useWallet';
import { createEscrow, releasePayment } from '../lib/contract';

export function BuyerDashboard() {
  const { publicKey, isConnected, connect } = useWallet();
  const [contentHash, setContentHash] = useState('');
  const [receivedFile, setReceivedFile] = useState<File | null>(null);
  const [loading, setLoading] = useState(false);

  const handlePurchase = async () => {
    if (!publicKey) return;

    try {
      setLoading(true);

      // Escrow oluştur
      const txHash = await createEscrow(
        publicKey,
        contentHash,
        'NATIVE', // XLM
        '100' // 100 XLM
      );

      alert(`Escrow created! Transaction: ${txHash}`);
    } catch (error) {
      console.error(error);
      alert('Purchase failed!');
    } finally {
      setLoading(false);
    }
  };

  const handleConfirm = async () => {
    if (!publicKey || !receivedFile) return;

    try {
      setLoading(true);

      // Hash doğrula ve ödeme yap
      const txHash = await releasePayment(
        publicKey,
        contentHash,
        'NATIVE',
        receivedFile
      );

      alert(`Payment released! Transaction: ${txHash}`);
    } catch (error) {
      console.error(error);
      alert('Confirmation failed! Hash mismatch?');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="buyer-dashboard">
      <h2>Buyer Dashboard</h2>

      {!isConnected ? (
        <button onClick={connect}>Connect Wallet</button>
      ) : (
        <div>
          <p>Connected: {publicKey}</p>

          <div className="form">
            <h3>Purchase Content</h3>
            <input
              type="text"
              placeholder="Content Hash"
              value={contentHash}
              onChange={(e) => setContentHash(e.target.value)}
            />
            <button onClick={handlePurchase} disabled={loading}>
              {loading ? 'Processing...' : 'Purchase (100 XLM)'}
            </button>
          </div>

          <div className="form">
            <h3>Confirm Receipt</h3>
            <input
              type="file"
              onChange={(e) => setReceivedFile(e.target.files?.[0] || null)}
            />
            <button onClick={handleConfirm} disabled={loading}>
              {loading ? 'Verifying...' : 'Verify & Release Payment'}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
```

## 📊 Event Listening

```typescript
// src/lib/events.ts
import { server, CONTRACT_ID } from './stellar';

/**
 * Contract event'lerini dinle
 */
export async function listenToEvents(
  eventType: string,
  callback: (event: any) => void
) {
  // Son ledger'ı al
  const latestLedger = await server.getLatestLedger();
  let cursor = latestLedger.sequence.toString();

  // Event stream başlat
  const stream = server.getEvents({
    startLedger: cursor,
    filters: [
      {
        type: 'contract',
        contractIds: [CONTRACT_ID],
        topics: [[eventType]],
      },
    ],
  });

  // Event'leri işle
  for await (const event of stream) {
    callback(event);
  }
}

// Kullanım:
listenToEvents('payment_released', (event) => {
  console.log('Payment released:', event);
  // UI güncelle, bildirim gönder, vb.
});
```

## 🧪 Test Örneği

```typescript
// src/__tests__/contract.test.ts
import { describe, it, expect } from 'vitest';
import { hashString } from '../lib/crypto';
import { registerContent } from '../lib/contract';

describe('Rise In Contract', () => {
  it('should hash content correctly', () => {
    const content = 'Hello, Rise In!';
    const hash = hashString(content);
    
    expect(hash).toHaveLength(64); // SHA-256 = 64 hex chars
  });

  it('should register content', async () => {
    const sellerKey = 'GXXX...'; // Test account
    const hash = hashString('test content');
    
    const txHash = await registerContent(
      sellerKey,
      hash,
      '100',
      'Test Content'
    );
    
    expect(txHash).toBeTruthy();
  });
});
```

## 📚 Kaynaklar

- [Stellar SDK Documentation](https://stellar.github.io/js-stellar-sdk/)
- [Soroban RPC](https://soroban.stellar.org/docs/reference/rpc)
- [Freighter Wallet](https://www.freighter.app/)

---

**Not**: Bu örnekler eğitim amaçlıdır. Production kullanımı için error handling, loading states, ve UX iyileştirmeleri ekleyin.
