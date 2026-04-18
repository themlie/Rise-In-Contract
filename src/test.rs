#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, BytesN, Env, String,
};

use crate::{
    contract::{RiseInContract, RiseInContractClient},
    types::EscrowState,
};

/// Test helper to create a mock token contract
fn create_token_contract<'a>(env: &Env, admin: &Address) -> (Address, token::StellarAssetClient<'a>) {
    let token_address = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_admin = token::StellarAssetClient::new(env, &token_address);
    (token_address, token_admin)
}

/// Test helper to mint tokens to an address
fn mint_tokens(token_admin: &token::StellarAssetClient, to: &Address, amount: i128) {
    token_admin.mint(to, &amount);
}

/// Test helper to get token balance
fn get_balance(env: &Env, token_address: &Address, account: &Address) -> i128 {
    let client = token::Client::new(env, token_address);
    client.balance(account)
}

/// Test helper to create a sample content hash
fn sample_hash(env: &Env, seed: u8) -> BytesN<32> {
    let mut bytes = [seed; 32];
    bytes[0] = seed;
    BytesN::from_array(env, &bytes)
}

// ==================== CONTENT REGISTRATION TESTS ====================

#[test]
fn test_register_content_success() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    let seller = Address::generate(&env);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128; // 100 XLM
    let description = String::from_str(&env, "Premium Code Package");
    
    // Register content
    client.register_content(&seller, &content_hash, &price, &description);
    
    // Verify content info
    let content_info = client.get_content(&content_hash);
    assert_eq!(content_info.seller, seller);
    assert_eq!(content_info.price, price);
    assert_eq!(content_info.description, description);
    
    // Verify stats
    let stats = client.get_stats();
    assert_eq!(stats.total_contents, 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // AlreadyRegistered
fn test_register_content_duplicate() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    let seller = Address::generate(&env);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128;
    let description = String::from_str(&env, "Test Content");
    
    // Register once
    client.register_content(&seller, &content_hash, &price, &description);
    
    // Try to register again (should panic)
    client.register_content(&seller, &content_hash, &price, &description);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // InvalidPrice
fn test_register_content_invalid_price() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    let seller = Address::generate(&env);
    let content_hash = sample_hash(&env, 1);
    let price = 0i128; // Invalid price
    let description = String::from_str(&env, "Test Content");
    
    // Should panic
    client.register_content(&seller, &content_hash, &price, &description);
}

// ==================== ESCROW CREATION TESTS ====================

#[test]
fn test_create_escrow_success() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    // Setup
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let admin = Address::generate(&env);
    
    let (token_address, token_admin) = create_token_contract(&env, &admin);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128; // 100 XLM
    let description = String::from_str(&env, "Test Content");
    
    // Register content
    client.register_content(&seller, &content_hash, &price, &description);
    
    // Mint tokens to buyer
    mint_tokens(&token_admin, &buyer, price);
    
    // Create escrow
    client.create_escrow(&buyer, &content_hash, &token_address, &price);
    
    // Verify escrow
    let escrow = client.get_escrow(&content_hash, &buyer);
    assert_eq!(escrow.buyer, buyer);
    assert_eq!(escrow.seller, seller);
    assert_eq!(escrow.amount, price);
    assert_eq!(escrow.state, EscrowState::Locked);
    
    // Verify stats
    let stats = client.get_stats();
    assert_eq!(stats.total_escrows, 1);
    
    // Verify contract balance
    let contract_balance = get_balance(&env, &token_address, &contract_id);
    assert_eq!(contract_balance, price);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")] // ContentNotFound
fn test_create_escrow_content_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    let buyer = Address::generate(&env);
    let admin = Address::generate(&env);
    let (token_address, _) = create_token_contract(&env, &admin);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128;
    
    // Try to create escrow without registering content (should panic)
    client.create_escrow(&buyer, &content_hash, &token_address, &price);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // InvalidPaymentAmount
fn test_create_escrow_wrong_amount() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let admin = Address::generate(&env);
    let (token_address, token_admin) = create_token_contract(&env, &admin);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128;
    let wrong_amount = 500_000_000i128; // Wrong amount
    let description = String::from_str(&env, "Test Content");
    
    // Register content
    client.register_content(&seller, &content_hash, &price, &description);
    
    // Mint tokens
    mint_tokens(&token_admin, &buyer, wrong_amount);
    
    // Try to create escrow with wrong amount (should panic)
    client.create_escrow(&buyer, &content_hash, &token_address, &wrong_amount);
}

// ==================== DELIVERY TESTS ====================

#[test]
fn test_mark_delivered_success() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    // Setup
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let admin = Address::generate(&env);
    let (token_address, token_admin) = create_token_contract(&env, &admin);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128;
    let description = String::from_str(&env, "Test Content");
    
    // Register and create escrow
    client.register_content(&seller, &content_hash, &price, &description);
    mint_tokens(&token_admin, &buyer, price);
    client.create_escrow(&buyer, &content_hash, &token_address, &price);
    
    // Mark as delivered
    client.mark_delivered(&seller, &content_hash, &buyer);
    
    // Verify state changed
    let escrow = client.get_escrow(&content_hash, &buyer);
    assert_eq!(escrow.state, EscrowState::Delivered);
    assert!(escrow.delivered_at.is_some());
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // Unauthorized
fn test_mark_delivered_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    // Setup
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let impostor = Address::generate(&env); // Not the seller
    let admin = Address::generate(&env);
    let (token_address, token_admin) = create_token_contract(&env, &admin);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128;
    let description = String::from_str(&env, "Test Content");
    
    // Register and create escrow
    client.register_content(&seller, &content_hash, &price, &description);
    mint_tokens(&token_admin, &buyer, price);
    client.create_escrow(&buyer, &content_hash, &token_address, &price);
    
    // Try to mark as delivered by impostor (should panic)
    client.mark_delivered(&impostor, &content_hash, &buyer);
}

// ==================== PAYMENT RELEASE TESTS ====================

#[test]
fn test_release_payment_success() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    // Setup
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let admin = Address::generate(&env);
    let (token_address, token_admin) = create_token_contract(&env, &admin);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128;
    let description = String::from_str(&env, "Test Content");
    
    // Full flow: register -> escrow -> deliver -> release
    client.register_content(&seller, &content_hash, &price, &description);
    mint_tokens(&token_admin, &buyer, price);
    client.create_escrow(&buyer, &content_hash, &token_address, &price);
    client.mark_delivered(&seller, &content_hash, &buyer);
    
    // Release payment
    client.release_payment(&buyer, &content_hash, &token_address);
    
    // Verify state
    let escrow = client.get_escrow(&content_hash, &buyer);
    assert_eq!(escrow.state, EscrowState::Completed);
    
    // Verify seller received payment
    let seller_balance = get_balance(&env, &token_address, &seller);
    assert_eq!(seller_balance, price);
    
    // Verify contract balance is zero
    let contract_balance = get_balance(&env, &token_address, &contract_id);
    assert_eq!(contract_balance, 0);
    
    // Verify stats
    let stats = client.get_stats();
    assert_eq!(stats.total_completed, 1);
    assert_eq!(stats.total_volume, price);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // InvalidEscrowState
fn test_release_payment_not_delivered() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    // Setup
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let admin = Address::generate(&env);
    let (token_address, token_admin) = create_token_contract(&env, &admin);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128;
    let description = String::from_str(&env, "Test Content");
    
    // Register and create escrow (but don't deliver)
    client.register_content(&seller, &content_hash, &price, &description);
    mint_tokens(&token_admin, &buyer, price);
    client.create_escrow(&buyer, &content_hash, &token_address, &price);
    
    // Try to release without delivery (should panic)
    client.release_payment(&buyer, &content_hash, &token_address);
}

// ==================== TIMEOUT REFUND TESTS ====================

#[test]
fn test_refund_timeout_success() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    // Setup
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let admin = Address::generate(&env);
    let (token_address, token_admin) = create_token_contract(&env, &admin);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128;
    let description = String::from_str(&env, "Test Content");
    
    // Register and create escrow
    client.register_content(&seller, &content_hash, &price, &description);
    mint_tokens(&token_admin, &buyer, price);
    client.create_escrow(&buyer, &content_hash, &token_address, &price);
    
    // Simulate 24 hours passing
    env.ledger().set(LedgerInfo {
        timestamp: env.ledger().timestamp() + 24 * 60 * 60 + 1, // 24h + 1s
        protocol_version: 20,
        sequence_number: env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 16,
        min_persistent_entry_ttl: 16,
        max_entry_ttl: 6312000,
    });
    
    // Request refund
    client.refund_timeout(&buyer, &content_hash, &token_address);
    
    // Verify state
    let escrow = client.get_escrow(&content_hash, &buyer);
    assert_eq!(escrow.state, EscrowState::Refunded);
    
    // Verify buyer received refund
    let buyer_balance = get_balance(&env, &token_address, &buyer);
    assert_eq!(buyer_balance, price);
    
    // Verify contract balance is zero
    let contract_balance = get_balance(&env, &token_address, &contract_id);
    assert_eq!(contract_balance, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")] // TimeoutNotReached
fn test_refund_timeout_too_early() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    // Setup
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let admin = Address::generate(&env);
    let (token_address, token_admin) = create_token_contract(&env, &admin);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128;
    let description = String::from_str(&env, "Test Content");
    
    // Register and create escrow
    client.register_content(&seller, &content_hash, &price, &description);
    mint_tokens(&token_admin, &buyer, price);
    client.create_escrow(&buyer, &content_hash, &token_address, &price);
    
    // Try to refund immediately (should panic)
    client.refund_timeout(&buyer, &content_hash, &token_address);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // InvalidEscrowState
fn test_refund_after_delivery() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    // Setup
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let admin = Address::generate(&env);
    let (token_address, token_admin) = create_token_contract(&env, &admin);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128;
    let description = String::from_str(&env, "Test Content");
    
    // Register, escrow, and deliver
    client.register_content(&seller, &content_hash, &price, &description);
    mint_tokens(&token_admin, &buyer, price);
    client.create_escrow(&buyer, &content_hash, &token_address, &price);
    client.mark_delivered(&seller, &content_hash, &buyer);
    
    // Simulate timeout
    env.ledger().set(LedgerInfo {
        timestamp: env.ledger().timestamp() + 24 * 60 * 60 + 1,
        protocol_version: 20,
        sequence_number: env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 16,
        min_persistent_entry_ttl: 16,
        max_entry_ttl: 6312000,
    });
    
    // Try to refund after delivery (should panic)
    client.refund_timeout(&buyer, &content_hash, &token_address);
}

// ==================== INTEGRATION TESTS ====================

#[test]
fn test_multiple_buyers_same_content() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    // Setup
    let seller = Address::generate(&env);
    let buyer1 = Address::generate(&env);
    let buyer2 = Address::generate(&env);
    let admin = Address::generate(&env);
    let (token_address, token_admin) = create_token_contract(&env, &admin);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128;
    let description = String::from_str(&env, "Popular Content");
    
    // Register content once
    client.register_content(&seller, &content_hash, &price, &description);
    
    // Two buyers purchase the same content
    mint_tokens(&token_admin, &buyer1, price);
    mint_tokens(&token_admin, &buyer2, price);
    
    client.create_escrow(&buyer1, &content_hash, &token_address, &price);
    client.create_escrow(&buyer2, &content_hash, &token_address, &price);
    
    // Verify both escrows exist
    let escrow1 = client.get_escrow(&content_hash, &buyer1);
    let escrow2 = client.get_escrow(&content_hash, &buyer2);
    
    assert_eq!(escrow1.buyer, buyer1);
    assert_eq!(escrow2.buyer, buyer2);
    assert_eq!(escrow1.seller, seller);
    assert_eq!(escrow2.seller, seller);
    
    // Verify stats
    let stats = client.get_stats();
    assert_eq!(stats.total_escrows, 2);
    assert_eq!(stats.total_contents, 1);
}

#[test]
fn test_complete_happy_path() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, RiseInContract);
    let client = RiseInContractClient::new(&env, &contract_id);
    
    // Setup
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let admin = Address::generate(&env);
    let (token_address, token_admin) = create_token_contract(&env, &admin);
    let content_hash = sample_hash(&env, 1);
    let price = 1_000_000_000i128;
    let description = String::from_str(&env, "Complete Flow Test");
    
    // Step 1: Seller registers content
    client.register_content(&seller, &content_hash, &price, &description);
    
    // Step 2: Buyer creates escrow
    mint_tokens(&token_admin, &buyer, price);
    client.create_escrow(&buyer, &content_hash, &token_address, &price);
    
    // Step 3: Seller delivers content (off-chain: encrypted with buyer's public key)
    client.mark_delivered(&seller, &content_hash, &buyer);
    
    // Step 4: Buyer verifies hash and releases payment
    client.release_payment(&buyer, &content_hash, &token_address);
    
    // Verify final state
    let escrow = client.get_escrow(&content_hash, &buyer);
    assert_eq!(escrow.state, EscrowState::Completed);
    
    let seller_balance = get_balance(&env, &token_address, &seller);
    assert_eq!(seller_balance, price);
    
    let stats = client.get_stats();
    assert_eq!(stats.total_contents, 1);
    assert_eq!(stats.total_escrows, 1);
    assert_eq!(stats.total_completed, 1);
    assert_eq!(stats.total_volume, price);
}
