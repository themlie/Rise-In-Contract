use soroban_sdk::contracterror;

/// Rise In Contract Error Codes
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Content already registered with this hash
    AlreadyRegistered = 1,
    
    /// Content not found in registry
    ContentNotFound = 2,
    
    /// Escrow agreement not found
    EscrowNotFound = 3,
    
    /// Caller is not authorized for this operation
    Unauthorized = 4,
    
    /// Escrow already exists for this content and buyer
    EscrowAlreadyExists = 5,
    
    /// Escrow is not in the expected state
    InvalidEscrowState = 6,
    
    /// Payment amount doesn't match the content price
    InvalidPaymentAmount = 7,
    
    /// Timeout period has not elapsed yet
    TimeoutNotReached = 8,
    
    /// Hash verification failed
    HashMismatch = 9,
    
    /// Invalid price (must be greater than 0)
    InvalidPrice = 10,

    /// Content has active escrows and cannot be deleted
    ContentHasActiveEscrows = 11,
}
