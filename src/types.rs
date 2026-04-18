use soroban_sdk::{contracttype, Address, BytesN, String};

/// Escrow state machine
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowState {
    Locked,
    Delivered,
    Completed,
    Refunded,
}

/// Content registration data — Persistent storage
#[contracttype]
#[derive(Clone, Debug)]
pub struct ContentInfo {
    pub seller: Address,
    pub content_hash: BytesN<32>,
    pub price: i128,
    pub description: String,
    pub registered_at: u64,
}

/// Escrow agreement — Persistent storage
#[contracttype]
#[derive(Clone, Debug)]
pub struct EscrowAgreement {
    pub content_hash: BytesN<32>,
    pub seller: Address,
    pub buyer: Address,
    pub amount: i128,
    pub state: EscrowState,
    pub created_at: u64,
    pub timeout_at: u64,
    pub delivered_at: Option<u64>,
}

/// Contract statistics — Instance storage
#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractStats {
    pub total_contents: u64,
    pub total_escrows: u64,
    pub total_completed: u64,
    pub total_volume: i128,
}
