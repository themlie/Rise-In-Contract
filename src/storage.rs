use soroban_sdk::{Address, BytesN, Env};

use crate::types::{ContentInfo, ContractStats, EscrowAgreement};

#[soroban_sdk::contracttype]
pub enum StorageKey {
    Content(BytesN<32>),
    Escrow(BytesN<32>, Address),
    Stats,
}

pub struct Storage;

impl Storage {
    // ==================== CONTENT ====================

    pub fn save_content(env: &Env, content_hash: &BytesN<32>, info: &ContentInfo) {
        let key = StorageKey::Content(content_hash.clone());
        env.storage().persistent().set(&key, info);
        env.storage().persistent().extend_ttl(&key, 5_184_000, 5_184_000);
    }

    pub fn get_content(env: &Env, content_hash: &BytesN<32>) -> Option<ContentInfo> {
        let key = StorageKey::Content(content_hash.clone());
        env.storage().persistent().get(&key)
    }

    pub fn has_content(env: &Env, content_hash: &BytesN<32>) -> bool {
        let key = StorageKey::Content(content_hash.clone());
        env.storage().persistent().has(&key)
    }

    pub fn delete_content(env: &Env, content_hash: &BytesN<32>) {
        let key = StorageKey::Content(content_hash.clone());
        env.storage().persistent().remove(&key);
    }

    // ==================== ESCROW ====================

    pub fn save_escrow(env: &Env, escrow: &EscrowAgreement) {
        let key = StorageKey::Escrow(escrow.content_hash.clone(), escrow.buyer.clone());
        env.storage().persistent().set(&key, escrow);
        env.storage().persistent().extend_ttl(&key, 1_036_800, 1_036_800);
    }

    pub fn get_escrow(env: &Env, content_hash: &BytesN<32>, buyer: &Address) -> Option<EscrowAgreement> {
        let key = StorageKey::Escrow(content_hash.clone(), buyer.clone());
        env.storage().persistent().get(&key)
    }

    pub fn has_escrow(env: &Env, content_hash: &BytesN<32>, buyer: &Address) -> bool {
        let key = StorageKey::Escrow(content_hash.clone(), buyer.clone());
        env.storage().persistent().has(&key)
    }

    // ==================== STATS (Instance) ====================

    pub fn get_stats(env: &Env) -> ContractStats {
        if !env.storage().instance().has(&StorageKey::Stats) {
            return ContractStats {
                total_contents: 0,
                total_escrows: 0,
                total_completed: 0,
                total_volume: 0,
            };
        }
        env.storage()
            .instance()
            .get::<StorageKey, ContractStats>(&StorageKey::Stats)
            .unwrap_or(ContractStats {
                total_contents: 0,
                total_escrows: 0,
                total_completed: 0,
                total_volume: 0,
            })
    }

    pub fn save_stats(env: &Env, stats: &ContractStats) {
        env.storage().instance().set(&StorageKey::Stats, stats);
        env.storage().instance().extend_ttl(1_036_800, 1_036_800);
    }

    pub fn increment_content_count(env: &Env) {
        let mut stats = Self::get_stats(env);
        stats.total_contents += 1;
        Self::save_stats(env, &stats);
    }

    pub fn increment_escrow_count(env: &Env) {
        let mut stats = Self::get_stats(env);
        stats.total_escrows += 1;
        Self::save_stats(env, &stats);
    }

    pub fn record_completion(env: &Env, amount: i128) {
        let mut stats = Self::get_stats(env);
        stats.total_completed += 1;
        stats.total_volume += amount;
        Self::save_stats(env, &stats);
    }
}
