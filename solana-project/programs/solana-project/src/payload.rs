use borsh::{BorshDeserialize, BorshSerialize};
use primitive_types::U256;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct XstreamStartPayload {
    pub start_time: u64,
    pub end_time: u64,
    pub amount: u64,
    pub to_chain_id: [u8; 32],
    pub sender: [u8; 32],
    pub receiver: [u8; 32],
    pub can_update: u64,
    pub can_cancel: u64,
    pub token_mint: [u8; 32],
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct XstreamDepositPayload {
    pub amount: u64,
    pub to_chain_id: [u8; 32],
    pub sender: [u8; 32],
    pub token_mint: [u8; 32],
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct XstreamUpdatePayload {
    pub start_time: u64,
    pub end_tme: u64,
    pub amount: u64,
    pub to_chain_id: [u8; 32],
    pub sender: [u8; 32],
    pub receiver: [u8; 32],
    pub can_update: u64,
    pub can_cancel: u64,
    pub token_mint: [u8; 32],
    pub data_account: [u8; 32],
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct XstreamPausePayload {
    pub to_chain_id: [u8; 32],
    pub depositor: [u8; 32],
    pub token_mint: [u8; 32],
    pub receiver: [u8; 32],
    pub data_account: [u8; 32],
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct XstreamWithdrawPayload {
    pub to_chain_id: [u8; 32],
    pub withdrawer: [u8; 32],
    pub token_mint: [u8; 32],
    pub depositor: [u8; 32],
    pub data_account: [u8; 32],
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct XstreamCancelPayload {
    pub to_chain_id: [u8; 32],
    pub depositor: [u8; 32],
    pub token_mint: [u8; 32],
    pub receiver: [u8; 32],
    pub data_account: [u8; 32],
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct XstreamWithdrawDepositPayload {
    pub amount: u64,
    pub to_chain_id: [u8; 32],
    pub withdrawer: [u8; 32],
    pub token_mint: [u8; 32],
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct XstreamInstantTransferPayload {
    pub amount: u64,
    pub to_chain_id: [u8; 32],
    pub sender: [u8; 32],
    pub token_mint: [u8; 32],
    pub receiver: [u8; 32],
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct XstreamDirectTransferPayload {
    pub amount: u64,
    pub to_chain_id: [u8; 32],
    pub sender: [u8; 32],
    pub token_mint: [u8; 32],
    pub withdrawer: [u8; 32],
}
