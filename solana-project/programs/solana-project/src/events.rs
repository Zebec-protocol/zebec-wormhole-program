use anchor_lang::prelude::*;

#[event]
pub struct InitializedPDA {
    pub pda: Pubkey,
}

#[event]
pub struct InitializedPDATokenAccount {
    pub pda: Pubkey,
    pub token_mint: Pubkey,
}

#[event]
pub struct Initialized {
    pub owner: Pubkey,
    pub nonce: u32,
}

#[event]
pub struct RegisteredChain {
    pub chain_id: u16,
    pub emitter_addr: String,
}

#[event]
pub struct StoredMsg {
    pub msg_type: u64,
    pub sender: [u8; 32],
    pub count: u64,
}

#[event]
pub struct Deposited {
    pub sender: [u8; 32],
    pub current_count: u64,
}

#[event]
pub struct StreamUpdated {
    pub sender: [u8; 32],
    pub current_count: u64,
}

#[event]
pub struct PausedResumed {
    pub sender: [u8; 32],
    pub current_count: u64,
}

#[event]
pub struct DirectTransferredNative {
    pub sender: [u8; 32],
    pub sender_chain: u16,
    pub target_chain: u16,
    pub receiver: [u8; 32],
    pub current_count: u64,
}

#[event]
pub struct DirectTransferredWrapped {
    pub sender: [u8; 32],
    pub sender_chain: u16,
    pub target_chain: u16,
    pub receiver: [u8; 32],
    pub current_count: u64,
}

#[event]
pub struct StreamCreated {
    pub sender: [u8; 32],
    pub current_count: u64,
}

#[event]
pub struct CancelCreated {
    pub sender: [u8; 32],
    pub current_count: u64,
}

#[event]
pub struct SenderWithdrawCreated {
    pub sender: [u8; 32],
    pub current_count: u64,
}

#[event]
pub struct InstantTransferCreated {
    pub sender: [u8; 32],
    pub current_count: u64,
}

#[event]
pub struct ReceiverWithdrawCreated {
    pub sender: [u8; 32],
    pub current_count: u64,
}

#[event]
pub struct ExecutedTransaction {
    pub from_chain_id: u16,
    pub eth_add: [u8; 32],
    pub transaction: Pubkey,
}
