use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, String};

use crate::errors::Error;
use crate::storage::Storage;
use crate::types::{ContentInfo, ContractStats, EscrowAgreement, EscrowState};

#[contract]
pub struct RiseInContract;

#[contractimpl]
impl RiseInContract {
    // ==================== CONTENT REGISTRATION ====================

    pub fn register_content(
        env: Env,
        seller: Address,
        content_hash: BytesN<32>,
        price: i128,
        description: String,
    ) -> Result<(), Error> {
        seller.require_auth();

        if price <= 0 {
            return Err(Error::InvalidPrice);
        }

        if Storage::has_content(&env, &content_hash) {
            return Err(Error::AlreadyRegistered);
        }

        let content_info = ContentInfo {
            seller: seller.clone(),
            content_hash: content_hash.clone(),
            price,
            description,
            registered_at: env.ledger().timestamp(),
        };

        Storage::save_content(&env, &content_hash, &content_info);
        Storage::increment_content_count(&env);

        env.events().publish(
            (String::from_str(&env, "content_registered"), seller),
            content_hash,
        );

        Ok(())
    }

    pub fn get_content(env: Env, content_hash: BytesN<32>) -> Result<ContentInfo, Error> {
        Storage::get_content(&env, &content_hash).ok_or(Error::ContentNotFound)
    }

    // ==================== ESCROW ====================

    pub fn create_escrow(
        env: Env,
        buyer: Address,
        content_hash: BytesN<32>,
        token: Address,
        amount: i128,
    ) -> Result<(), Error> {
        buyer.require_auth();

        let content_info = Storage::get_content(&env, &content_hash)
            .ok_or(Error::ContentNotFound)?;

        if amount != content_info.price {
            return Err(Error::InvalidPaymentAmount);
        }

        if Storage::has_escrow(&env, &content_hash, &buyer) {
            return Err(Error::EscrowAlreadyExists);
        }

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&buyer, &env.current_contract_address(), &amount);

        let current_time = env.ledger().timestamp();
        let escrow = EscrowAgreement {
            content_hash: content_hash.clone(),
            seller: content_info.seller.clone(),
            buyer: buyer.clone(),
            amount,
            state: EscrowState::Locked,
            created_at: current_time,
            timeout_at: current_time + 86_400, // 24 saat
            delivered_at: None,
        };

        Storage::save_escrow(&env, &escrow);
        Storage::increment_escrow_count(&env);

        env.events().publish(
            (String::from_str(&env, "escrow_created"), buyer),
            content_hash,
        );

        Ok(())
    }

    pub fn mark_delivered(
        env: Env,
        seller: Address,
        content_hash: BytesN<32>,
        buyer: Address,
    ) -> Result<(), Error> {
        seller.require_auth();

        let mut escrow = Storage::get_escrow(&env, &content_hash, &buyer)
            .ok_or(Error::EscrowNotFound)?;

        if escrow.seller != seller {
            return Err(Error::Unauthorized);
        }

        if escrow.state != EscrowState::Locked {
            return Err(Error::InvalidEscrowState);
        }

        escrow.state = EscrowState::Delivered;
        escrow.delivered_at = Some(env.ledger().timestamp());
        Storage::save_escrow(&env, &escrow);

        env.events().publish(
            (String::from_str(&env, "content_delivered"), seller),
            (content_hash, buyer),
        );

        Ok(())
    }

    pub fn release_payment(
        env: Env,
        buyer: Address,
        content_hash: BytesN<32>,
        token: Address,
    ) -> Result<(), Error> {
        buyer.require_auth();

        let mut escrow = Storage::get_escrow(&env, &content_hash, &buyer)
            .ok_or(Error::EscrowNotFound)?;

        if escrow.buyer != buyer {
            return Err(Error::Unauthorized);
        }

        if escrow.state != EscrowState::Delivered {
            return Err(Error::InvalidEscrowState);
        }

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &escrow.seller, &escrow.amount);

        escrow.state = EscrowState::Completed;
        Storage::save_escrow(&env, &escrow);
        Storage::record_completion(&env, escrow.amount);

        env.events().publish(
            (String::from_str(&env, "payment_released"), buyer),
            (content_hash, escrow.seller, escrow.amount),
        );

        Ok(())
    }

    pub fn refund_timeout(
        env: Env,
        buyer: Address,
        content_hash: BytesN<32>,
        token: Address,
    ) -> Result<(), Error> {
        buyer.require_auth();

        let mut escrow = Storage::get_escrow(&env, &content_hash, &buyer)
            .ok_or(Error::EscrowNotFound)?;

        if escrow.buyer != buyer {
            return Err(Error::Unauthorized);
        }

        if escrow.state != EscrowState::Locked {
            return Err(Error::InvalidEscrowState);
        }

        if env.ledger().timestamp() < escrow.timeout_at {
            return Err(Error::TimeoutNotReached);
        }

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &buyer, &escrow.amount);

        escrow.state = EscrowState::Refunded;
        Storage::save_escrow(&env, &escrow);

        env.events().publish(
            (String::from_str(&env, "refund_issued"), buyer),
            (content_hash, escrow.amount),
        );

        Ok(())
    }

    pub fn get_escrow(
        env: Env,
        content_hash: BytesN<32>,
        buyer: Address,
    ) -> Result<EscrowAgreement, Error> {
        Storage::get_escrow(&env, &content_hash, &buyer).ok_or(Error::EscrowNotFound)
    }

    pub fn get_stats(env: Env) -> ContractStats {
        Storage::get_stats(&env)
    }
}
