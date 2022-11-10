use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, TokenAccount, Token}
};
use crate::constants::*;
use crate::portal::TokenPortalBridge;
use crate::state::*;
use std::str::FromStr;
use crate::wormhole::*;
use hex::decode;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init_if_needed,
        seeds=[b"config".as_ref()],
        payer=owner,
        bump,
        space=8+32+4
    )]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(chain_id:u16, emitter_addr:String)]
pub struct RegisterChain<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        constraint = config.owner == owner.key()
    )]
    pub config: Account<'info, Config>,
    #[account(
        init_if_needed,
        seeds=[b"EmitterAddress".as_ref(), chain_id.to_be_bytes().as_ref()],
        payer=owner,
        bump,
        space=8 + 2 + 4 + EVM_CHAIN_ADDRESS_LENGTH
    )]
    pub emitter_acc: Account<'info, EmitterAddrAccount>,
}

#[derive(Accounts)]
pub struct InitializePDA<'info> {
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        init,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        payer=zebec_eoa,
        bump,
        space=8 + 4
    )]
    pub processed_vaa: Account<'info, ProcessedVAA>,
    pub emitter_acc: Account<'info, EmitterAddrAccount>,
    /// This requires some fancy hashing, so confirm it's derived address in the function itself.
    #[account(
        constraint = core_bridge_vaa.to_account_info().owner == &Pubkey::from_str(CORE_BRIDGE_ADDRESS).unwrap()
    )]
    /// CHECK: This account is owned by Core Bridge so we trust it
    pub core_bridge_vaa: AccountInfo<'info>,

    ///CHECK:: pda_account are checked inside
    #[account(mut)]
    pub pda_account: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct InitializePDATokenAccount<'info> {
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,   
    pub associated_token_program: Program<'info, AssociatedToken>,


    #[account(
        init,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        payer=zebec_eoa,
        bump,
        space=8+4
    )]
    pub processed_vaa: Account<'info, ProcessedVAA>,
    pub emitter_acc: Account<'info, EmitterAddrAccount>,
    /// This requires some fancy hashing, so confirm it's derived address in the function itself.
    #[account(
        constraint = core_bridge_vaa.to_account_info().owner == &Pubkey::from_str(CORE_BRIDGE_ADDRESS).unwrap()
    )]
    /// CHECK: This account is owned by Core Bridge so we trust it
    pub core_bridge_vaa: AccountInfo<'info>,

    ///CHECK:: pda_account are checked inside
    #[account(mut)]
    pub pda_account: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = zebec_eoa,
        associated_token::mint = token_mint,
        associated_token::authority = pda_account,
    )]
    pub pda_token_account: Box <Account<'info, TokenAccount>>,
    pub token_mint: Account<'info, Mint>
}

#[derive(Accounts)]
#[instruction( 
    pid: Pubkey,
    accs: Vec<TransactionAccount>,
    data: Vec<u8>,
    sender: [u8; 32],
    current_count: u8
)]
pub struct CreateTransaction<'info> {
    #[account(zero, signer)]
    pub transaction: Box<Account<'info, Transaction>>,
    // One of the owners. Checked in the handler.
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,
    pub system_program: Program<'info, System>,

   #[account(
        mut,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &[current_count]
        ],
        bump
    )]
    /// CHECK: pda_signer is a PDA program signer. Data is never read or written to
    pub data_storage: Account<'info, TransactionData>,

    #[account(
        mut, 
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &[current_count]
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,
}

#[derive(Accounts)]
#[instruction( 
    pid: Pubkey,
    accs: Vec<TransactionAccount>,
    data: Vec<u8>,
    chain_id: Vec<u8>,
    sender: [u8; 32],
    current_count: u8
)]
pub struct CETransaction<'info> {
    #[account(zero, signer)]
    pub transaction: Box<Account<'info, Transaction>>,
    // One of the owners. Checked in the handler.
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,
    pub system_program: Program<'info, System>,

   #[account(
        mut,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &[current_count]
        ],
        bump
    )]
    pub data_storage: Account<'info, TransactionData>,

    ///CHECK: pda seeds checked
    #[account(
        mut,
        seeds = [
            &sender,
            &chain_id
        ],
        bump
    )]
    pub pda_signer: UncheckedAccount<'info>,

    #[account(
        mut, 
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &[current_count]
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,
}

#[derive(Accounts)]
#[instruction( 
    sender: [u8; 32],
    chain_id: Vec<u8>,
    current_count: u8
)]
pub struct DirectTransferNative<'info> {
    // One of the owners. Checked in the handler.
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,

   #[account(
        mut,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &[current_count]
        ],
        bump
    )]
    pub data_storage: Box<Account<'info, TransactionData>>,

    #[account(
        mut, 
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &[current_count]
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,

    ///CHECK: pda seeds checked
    #[account(
        mut,
        seeds = [
            &sender,
            &chain_id
        ],
        bump
    )]
    pub pda_signer: UncheckedAccount<'info>,

    //Native Transfer
    #[account(
        mut,
        seeds = [b"config"],
        bump,
    )]
    pub config: Account<'info, Config>,

    //from_owner = pda_signer

    #[account(
        mut,
        seeds = [b"config"],
        seeds::program = portal_bridge_program.key(),
        bump,
    )]
    /// CHECK: portal config
    pub portal_config: AccountInfo<'info>,
    
    #[account(
        mut,
        constraint = from.owner == pda_signer.key(),
        constraint = from.mint == mint.key(),
    )]
    pub from: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    /// CHECK: No need of data
    pub mint: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [mint.key().as_ref()],
        seeds::program = portal_bridge_program.key(),
        bump
    )]
    /// CHECK: portal custody
    pub portal_custody: AccountInfo<'info>,

    #[account(
        seeds = [b"authority_signer"],
        seeds::program = portal_bridge_program.key(),
        bump
    )]
    /// CHECK: portal authority signer
    pub portal_authority_signer: AccountInfo<'info>,

    #[account(
        seeds = [b"custody_signer"],
        seeds::program = portal_bridge_program.key(),
        bump
    )]
    /// CHECK: portal custody signer
    pub portal_custody_signer: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"Bridge"],
        seeds::program = core_bridge_program.key(),
        bump
    )]
    /// CHECK: bridge config
    pub bridge_config: AccountInfo<'info>,
    
    #[account(
        mut,
        signer
    )]
    /// CHECK: portal message
    pub portal_message: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"emitter"],
        seeds::program = portal_bridge_program.key(),
        bump
    )]
    /// CHECK: portal emitter
    pub portal_emitter: AccountInfo<'info>,
    
    #[account(
        mut,
        seeds = [b"Sequence", portal_emitter.key().as_ref()],
        seeds::program = core_bridge_program.key(),
        bump
    )]
    /// CHECK: portal sequence
    pub portal_sequence: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"fee_collector"],
        seeds::program = core_bridge_program.key(),
        bump
    )]
    /// CHECK: bridge fee collector
    pub bridge_fee_collector: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    pub rent: Sysvar<'info, Rent>,

    pub system_program: Program<'info, System>,

    pub portal_bridge_program: Program<'info, TokenPortalBridge>,

    pub core_bridge_program: Program<'info, WormholeCoreBridge>,

    pub token_program: Program<'info, Token>

}

#[derive(Accounts)]
#[instruction( 
    sender: [u8; 32],
    sender_chain: Vec<u8>,
    _token_address: Vec<u8>,
    _token_chain: u16,
    current_count: u8
)]
pub struct DirectTransferWrapped<'info> {
    // One of the owners. Checked in the handler.
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,

   #[account(
        mut,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &[current_count]
        ],
        bump
    )]
    /// CHECK: pda_signer is a PDA program signer. Data is never read or written to
    pub data_storage: Account<'info, TransactionData>,

    #[account(
        mut, 
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &[current_count]
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,

    ///CHECK: pda seeds checked
    #[account(
        mut,
        seeds = [
            &sender,
            &sender_chain
        ],
        bump
    )]
    pub pda_signer: UncheckedAccount<'info>,

    //Wrapped Transfer
    #[account(
        mut,
        seeds = [b"config"],
        bump,
    )]
    pub config: Account<'info, Config>,
        
    #[account(
        mut,
        constraint = from.owner == pda_signer.key(),
        constraint = from.mint == wrapped_mint.key(),
    )]
    pub from: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [b"config"],
        seeds::program = portal_bridge_program.key(),
        bump,
    )]
    /// CHECK: portal config
    pub portal_config: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [
            b"wrapped",
            _token_chain.to_be_bytes().as_ref(),
            _token_address.as_ref()
        ],
        seeds::program = portal_bridge_program.key(),
        bump,
    )]
    /// CHECK: portal config
    pub wrapped_mint: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [
            b"meta",
            wrapped_mint.key().as_ref()
        ],
        seeds::program = portal_bridge_program.key(),
        bump,
    )]
    /// CHECK: portal config
    pub wrapped_meta: AccountInfo<'info>,

    #[account(
        seeds = [b"authority_signer"],
        seeds::program = portal_bridge_program.key(),
        bump
    )]
    /// CHECK: portal authority signer
    pub portal_authority_signer: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"Bridge"],
        seeds::program = core_bridge_program.key(),
        bump
    )]
    /// CHECK: bridge config
    pub bridge_config: AccountInfo<'info>,

    #[account(
        mut,
        signer
    )]
    /// CHECK: portal message
    pub portal_message: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"emitter"],
        seeds::program = portal_bridge_program.key(),
        bump
    )]
    /// CHECK: portal emitter
    pub portal_emitter: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"Sequence", portal_emitter.key().as_ref()],
        seeds::program = core_bridge_program.key(),
        bump
    )]
    /// CHECK: portal sequence
    pub portal_sequence: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"fee_collector"],
        seeds::program = core_bridge_program.key(),
        bump
    )]
    /// CHECK: bridge fee collector
    pub bridge_fee_collector: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    pub rent: Sysvar<'info, Rent>,

    pub system_program: Program<'info, System>,

    pub portal_bridge_program: Program<'info, TokenPortalBridge>,

    pub core_bridge_program: Program<'info, WormholeCoreBridge>,

    pub token_program: Program<'info, Token>

}


#[derive(Accounts)]
#[instruction( 
    pid: Pubkey,
    accs: Vec<TransactionAccount>,
    data: Vec<u8>,
    sender: [u8; 32],
    _current_count: u8
)]
pub struct CreateTransactionReceiver<'info> {
    #[account(zero, signer)]
    pub transaction: Box<Account<'info, Transaction>>,
    // One of the owners. Checked in the handler.
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,
    pub system_program: Program<'info, System>,

   #[account(
        mut,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &[_current_count]
        ],
        bump
    )]
    /// CHECK: pda_signer is a PDA program signer. Data is never read or written to
    pub data_storage: Account<'info, TransactionData>,

    #[account(
        mut, 
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &[_current_count]
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,
}

#[derive(Accounts)]
#[instruction(
    current_count: u8, 
    sender: [u8; 32], 
)]
pub struct StoreMsg<'info>{

    // ZEBEC's EOA.
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,

    #[account(
        init,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        payer=payer,
        bump,
        space=8 + 4
    )]
    pub processed_vaa: Account<'info, ProcessedVAA>,
    pub emitter_acc: Account<'info, EmitterAddrAccount>,
    /// This requires some fancy hashing, so confirm it's derived address in the function itself.
    #[account(
        constraint = core_bridge_vaa.to_account_info().owner == &Pubkey::from_str(CORE_BRIDGE_ADDRESS).unwrap()
    )]
    /// CHECK: This account is owned by Core Bridge so we trust it
    pub core_bridge_vaa: AccountInfo<'info>,

    #[account(
        init,
        space = 8 + 174,
        payer = payer,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &[current_count]
        ],
        bump,
    )]
    pub data_storage: Account<'info, TransactionData>,

    #[account(
        init_if_needed,
        payer = payer, 
        space = 8 + 4,
        seeds = [
            b"txn_count".as_ref(),
            &sender,
        ],
        bump
    )]
    pub txn_count: Account<'info, Count>,

    #[account(
        init, 
        payer = payer,
        space = 8 + 1 + 1,
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &[current_count]
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,
}

#[derive(Accounts)]
#[instruction(  
    eth_add:[u8; 32],
    from_chain_id: Vec<u8>,
    _current_count: u8
)]
pub struct ExecuteTransaction<'info> {
    pub system_program: Program<'info, System>,
    ///CHECK: seeds are checked while creating transaction,
    /// if different seeds passed the signature will not match
    #[account(
        mut,
        seeds = [
            &eth_add,
            &from_chain_id
        ],
        bump
    )]
    pub pda_signer: UncheckedAccount<'info>,
    #[account(mut)]
    pub transaction: Box<Account<'info, Transaction>>,

    #[account(
        mut, 
        seeds = [
            b"txn_status".as_ref(),
            &eth_add,
            &[_current_count]
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,
}
