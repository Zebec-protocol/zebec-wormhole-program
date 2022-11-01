use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use borsh::{BorshDeserialize, BorshSerialize};

#[account]
#[derive(Default)]
pub struct Config {
    pub owner: Pubkey,
    pub nonce: u32,
}

#[account]
#[derive(Default)]
pub struct EmitterAddrAccount {
    pub chain_id: u16,
    pub emitter_addr: String,
}

//Empty account, we just need to check that it *exists*
#[account]
pub struct ProcessedVAA {}

#[account]
pub struct Transaction {
    //450
    // Target program to execute against.32
    pub program_id: Pubkey,
    // Accounts requried for the transaction.8+9*34
    pub accounts: Vec<TransactionAccount>,
    // Instruction data for the transaction.8+8
    pub data: Vec<u8>,
    // Boolean ensuring one time execution.1+8
    pub did_execute: bool,
}

#[account]
// TODO: can_update and cancel are bools
pub struct TransactionData {
    pub sender: Vec<u8>,
    pub receiver: Vec<u8>,
    pub data_account: Pubkey,
    pub from_chain_id: u64,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub can_update: bool,
    pub can_cancel: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TransactionAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[account]
#[derive(Default)]
pub struct Count {
    pub count: u8,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct TokenAmount {
    pub amount: u64,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Stream {
    pub start_time: u64,
    pub end_time: u64,
    pub amount: u64,
    pub can_cancel: bool,
    pub can_update: bool,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct StreamUpdate {
    pub start_time: u64,
    pub end_time: u64,
    pub amount: u64,
}

#[account]
pub struct Receipt {
    pub amt_to_mint: u64,
    pub foreign_receipient: [u8; 32],
    pub foreign_chain: u16,
    pub claimed: bool,
}

#[account]
pub struct MintInfo {
    pub mint: Pubkey,
}

impl From<&Transaction> for Instruction {
    fn from(tx: &Transaction) -> Instruction {
        Instruction {
            program_id: tx.program_id,
            accounts: tx.accounts.iter().map(Into::into).collect(),
            data: tx.data.clone(),
        }
    }
}

impl From<&TransactionAccount> for AccountMeta {
    fn from(account: &TransactionAccount) -> AccountMeta {
        match account.is_writable {
            false => AccountMeta::new_readonly(account.pubkey, account.is_signer),
            true => AccountMeta::new(account.pubkey, account.is_signer),
        }
    }
}

impl From<&AccountMeta> for TransactionAccount {
    fn from(account_meta: &AccountMeta) -> TransactionAccount {
        TransactionAccount {
            pubkey: account_meta.pubkey,
            is_signer: account_meta.is_signer,
            is_writable: account_meta.is_writable,
        }
    }
}
