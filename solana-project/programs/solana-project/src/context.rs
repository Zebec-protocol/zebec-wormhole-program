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
use zebec::{TokenWithdraw, StreamToken, FeeVaultData};
use zebec::constants::{OPERATE, OPERATEDATA, PREFIX_TOKEN};
use zebec::program::Zebec;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
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
        init,
        seeds=[b"EmitterAddress".as_ref(), chain_id.to_be_bytes().as_ref()],
        payer=owner,
        bump,
        space=8 + 2 + 4 + EVM_CHAIN_ADDRESS_LENGTH
    )]
    pub emitter_acc: Account<'info, EmitterAddrAccount>,
}

#[derive(Accounts)]
#[instruction(_sender:[u8;32], _chain_id:u16)]
pub struct InitializePDA<'info> {
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,
    pub system_program: Program<'info, System>,
    
    #[account(
        init,
        payer=zebec_eoa,
        space=8 + 8,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        bump,
        
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
        mut,
        seeds = [
            &_sender,
            &_chain_id.to_be_bytes()
        ],
        bump
    )]
    /// CHECK:: pda_account are checked inside
    pub pda_account: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(_sender:[u8;32], _chain_id:u16)]
pub struct InitializePDATokenAccount<'info> {
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,   
    pub associated_token_program: Program<'info, AssociatedToken>,

    #[account(
        init,
        payer=zebec_eoa,
        space= 8 + 8,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        bump,
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
    #[account(
        mut,
        seeds=[&_sender, &_chain_id.to_be_bytes()], 
        bump
    )]
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
    accs: Vec<TransactionAccount>,
    data: Vec<u8>,
    sender: [u8; 32],
    current_count: u64
)]
pub struct CreateTransaction<'info> {
    #[account(zero, signer)]
    pub transaction: Box<Account<'info, Transaction>>,
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,
    pub system_program: Program<'info, System>,

   #[account(
        mut,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub data_storage: Account<'info, TransactionData>,

    #[account(
        mut, 
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,
}

#[derive(Accounts)]
#[instruction( 
    accs: Vec<TransactionAccount>,
    data: Vec<u8>,
    chain_id: u16,
    sender: [u8; 32],
    current_count: u64
)]
pub struct CETransaction<'info> {
    #[account(zero, signer)]
    pub transaction: Box<Account<'info, Transaction>>,
    
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,
    pub system_program: Program<'info, System>,

   #[account(
        mut,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub data_storage: Account<'info, TransactionData>,

    ///CHECK: pda seeds checked
    #[account(
        mut,
        seeds = [
            &sender,
            &chain_id.to_be_bytes()
        ],
        bump
    )]
    pub pda_signer: UncheckedAccount<'info>,

    #[account(
        mut, 
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,
}

#[derive(Accounts)]
#[instruction( 
    sender: [u8; 32],
    chain_id: u16,
    current_count: u64,
)]
pub struct DirectTransferNative<'info> {
    
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,
    
   #[account(
        mut,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub data_storage: Box<Account<'info, TransactionData>>,

    #[account(
        mut, 
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,

    ///CHECK: pda seeds checked
    #[account(
        mut,
        seeds = [
            &sender,
            &chain_id.to_be_bytes()
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
    sender_chain: u16,
    _token_address: Vec<u8>,
    _token_chain: u16,
    current_count: u64,
)]
pub struct DirectTransferWrapped<'info> {
    
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,

   #[account(
        mut,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub data_storage: Account<'info, TransactionData>,

    #[account(
        mut, 
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,

    ///CHECK: pda seeds checked
    #[account(
        mut,
        seeds = [
            &sender,
            &sender_chain.to_be_bytes()
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
    accs: Vec<TransactionAccount>,
    data: Vec<u8>,
    sender: [u8; 32],
    _current_count: u64
)]
pub struct CreateTransactionReceiver<'info> {
    #[account(zero, signer)]
    pub transaction: Box<Account<'info, Transaction>>,
    
    #[account(mut)]
    pub zebec_eoa: Signer<'info>,
    pub system_program: Program<'info, System>,

   #[account(
        mut,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &_current_count.to_be_bytes()
        ],
        bump
    )]
    pub data_storage: Account<'info, TransactionData>,

    #[account(
        mut, 
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &_current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,
}

#[derive(Accounts)]
#[instruction(
    current_count: u64, 
    sender: [u8; 32], 
)]
pub struct StoreMsg<'info>{
    // ZEBEC's EOA.
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,

    #[account(
        init,
        payer=payer,
        space= 8 + 8,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        bump,
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
        space = 8 + 156,
        payer = payer,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &current_count.to_be_bytes()
        ],
        bump,
    )]
    pub data_storage: Account<'info, TransactionData>,

    #[account(
        init_if_needed,
        payer = payer, 
        space = 8 + 8,
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
        space = 8 + 1,
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,
}

#[derive(Accounts)]
#[instruction(  
    eth_add:[u8; 32],
    from_chain_id: u16,
    _current_count: u64
)]
pub struct ExecuteTransaction<'info> {
    pub system_program: Program<'info, System>,
    ///CHECK: seeds are checked while creating transaction,
    /// if different seeds passed the signature will not match
    #[account(
        mut,
        seeds = [
            &eth_add,
            &from_chain_id.to_be_bytes()
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
            &_current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Account<'info, TransactionStatus>,
}

#[derive(Accounts)]
#[instruction(
    sender:[u8;32],
    from_chain_id: u16,
    current_count: u64
)]
pub struct XstreamStart<'info> {
    // ZEBEC's EOA.
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer=payer,
        space= 8 + 8,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        bump,
    )]
    pub processed_vaa: Box<Account<'info, ProcessedVAA>>,
    pub emitter_acc: Box<Account<'info, EmitterAddrAccount>>,
    /// This requires some fancy hashing, so confirm it's derived address in the function itself.
    #[account(
        constraint = core_bridge_vaa.to_account_info().owner == &Pubkey::from_str(CORE_BRIDGE_ADDRESS).unwrap()
    )]
    /// CHECK: This account is owned by Core Bridge so we trust it
    pub core_bridge_vaa: AccountInfo<'info>,

    #[account(
        init,
        space = 8 + 156,
        payer = payer,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &current_count.to_be_bytes()
        ],
        bump,
    )]
    pub data_storage: Box<Account<'info, TransactionData>>,

    #[account(
        init_if_needed,
        payer = payer, 
        space = 8 + 8,
        seeds = [
            b"txn_count".as_ref(),
            &sender,
        ],
        bump
    )]
    pub txn_count: Box<Account<'info, Count>>,

    #[account(
        init, 
        payer = payer,
        space = 8 + 1,
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Box<Account<'info, TransactionStatus>>,
    #[account(zero)]
    pub data_account:  Account<'info, StreamToken>,
    #[account(
        init_if_needed,
        payer=source_account,
        seeds = [
            PREFIX_TOKEN.as_bytes(),
            source_account.key().as_ref(),
            mint.key().as_ref(),
        ],bump,
        space=8+8,
    )]
    pub withdraw_data: Box<Account<'info, TokenWithdraw>>,
    /// CHECK: validated in fee_vault constraint
    pub fee_owner:AccountInfo<'info>,
    #[account(
        seeds = [
            fee_owner.key().as_ref(),
            OPERATEDATA.as_bytes(),
            fee_vault.key().as_ref(),
        ],bump
    )]
    pub fee_vault_data: Account<'info,FeeVaultData>,
    #[account(
        constraint = fee_vault_data.fee_owner == fee_owner.key(),
        constraint = fee_vault_data.fee_vault_address == fee_vault.key(),
        seeds = [
            fee_owner.key().as_ref(),
            OPERATE.as_bytes(),           
        ],bump,        
    )]
    /// CHECK: seeds has been checked
    pub fee_vault:AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            &sender,
            &from_chain_id.to_be_bytes()
        ],
        bump
    )]
    pub source_account: UncheckedAccount<'info>,
    /// CHECK: new stream receiver, do not need to be checked
    pub dest_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program:Program<'info,Token>,
    pub mint:Account<'info,Mint>,
    pub rent: Sysvar<'info, Rent>,
    pub zebec_program: Program<'info, Zebec>

}

#[derive(Accounts)]
#[instruction(
    sender:[u8;32],
    from_chain_id: u16,
    current_count: u64
)]
pub struct XstreamUpdate<'info> {
    // ZEBEC's EOA.
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer=payer,
        space= 8 + 8,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        bump,
    )]
    pub processed_vaa: Box<Account<'info, ProcessedVAA>>,
    pub emitter_acc: Box<Account<'info, EmitterAddrAccount>>,
    /// This requires some fancy hashing, so confirm it's derived address in the function itself.
    #[account(
        constraint = core_bridge_vaa.to_account_info().owner == &Pubkey::from_str(CORE_BRIDGE_ADDRESS).unwrap()
    )]
    /// CHECK: This account is owned by Core Bridge so we trust it
    pub core_bridge_vaa: AccountInfo<'info>,

    #[account(
        init,
        space = 8 + 156,
        payer = payer,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &current_count.to_be_bytes()
        ],
        bump,
    )]
    pub data_storage: Box<Account<'info, TransactionData>>,

    #[account(
        init_if_needed,
        payer = payer, 
        space = 8 + 8,
        seeds = [
            b"txn_count".as_ref(),
            &sender,
        ],
        bump
    )]
    pub txn_count: Box<Account<'info, Count>>,

    #[account(
        init, 
        payer = payer,
        space = 8 + 1,
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Box<Account<'info, TransactionStatus>>,
    #[account(mut,
        constraint= data_account.sender==source_account.key(),
        constraint= data_account.receiver==dest_account.key(), 
        constraint= data_account.token_mint==mint.key(),            
    )]
    pub data_account:  Account<'info, StreamToken>,
    #[account(mut,
        seeds = [
            PREFIX_TOKEN.as_bytes(),
            source_account.key().as_ref(),
            mint.key().as_ref(),
        ],bump
    )]
    pub withdraw_data: Box<Account<'info, TokenWithdraw>>,
    #[account(
        mut,
        seeds = [
            &sender,
            &from_chain_id.to_be_bytes()
        ],
        bump
    )]
    pub source_account: UncheckedAccount<'info>,
    /// CHECK: stream receiver checked in data account
    pub dest_account: AccountInfo<'info>,
    pub mint:Account<'info,Mint>,
    pub system_program: Program<'info, System>,
    pub zebec_program: Program<'info, Zebec>

}

#[derive(Accounts)]
#[instruction(
    sender:[u8;32],
    from_chain_id: u16,
    current_count: u64
)]
pub struct XstreamDeposit<'info> {
    // ZEBEC's EOA.
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer=payer,
        space= 8 + 8,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        bump,
    )]
    pub processed_vaa: Box<Account<'info, ProcessedVAA>>,
    pub emitter_acc: Box<Account<'info, EmitterAddrAccount>>,
    /// This requires some fancy hashing, so confirm it's derived address in the function itself.
    #[account(
        constraint = core_bridge_vaa.to_account_info().owner == &Pubkey::from_str(CORE_BRIDGE_ADDRESS).unwrap()
    )]
    /// CHECK: This account is owned by Core Bridge so we trust it
    pub core_bridge_vaa: AccountInfo<'info>,

    #[account(
        init,
        space = 8 + 156,
        payer = payer,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &current_count.to_be_bytes()
        ],
        bump,
    )]
    pub data_storage: Box<Account<'info, TransactionData>>,

    #[account(
        init_if_needed,
        payer = payer, 
        space = 8 + 8,
        seeds = [
            b"txn_count".as_ref(),
            &sender,
        ],
        bump
    )]
    pub txn_count: Box<Account<'info, Count>>,

    #[account(
        init, 
        payer = payer,
        space = 8 + 1,
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Box<Account<'info, TransactionStatus>>,
    #[account(
        init_if_needed,
        payer=source_account,
        seeds = [
            source_account.key().as_ref(),
        ],bump,
        space=0,
    )]
     /// CHECK: seeds has been checked
    pub zebec_vault: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            &sender,
            &from_chain_id.to_be_bytes()
        ],
        bump
    )]
    pub source_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program:Program<'info,Token>,
    pub associated_token_program:Program<'info,AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
    pub mint:Account<'info,Mint>,
    #[account(
        mut,
        constraint= source_account_token_account.owner == source_account.key(),
        constraint= source_account_token_account.mint == mint.key()
    )]
    pub source_account_token_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = source_account,
        associated_token::mint = mint,
        associated_token::authority = zebec_vault,
    )]
    pub pda_account_token_account: Account<'info, TokenAccount>,
    pub zebec_program: Program<'info, Zebec>

}

#[derive(Accounts)]
#[instruction(
    sender:[u8;32],
    from_chain_id: u16,
    current_count: u64
)]
pub struct XstreamSenderWithdraw<'info> {
    // ZEBEC's EOA.
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer=payer,
        space= 8 + 8,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        bump,
    )]
    pub processed_vaa: Box<Account<'info, ProcessedVAA>>,
    pub emitter_acc: Box<Account<'info, EmitterAddrAccount>>,
    /// This requires some fancy hashing, so confirm it's derived address in the function itself.
    #[account(
        constraint = core_bridge_vaa.to_account_info().owner == &Pubkey::from_str(CORE_BRIDGE_ADDRESS).unwrap()
    )]
    /// CHECK: This account is owned by Core Bridge so we trust it
    pub core_bridge_vaa: AccountInfo<'info>,

    #[account(
        init,
        space = 8 + 156,
        payer = payer,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &current_count.to_be_bytes()
        ],
        bump,
    )]
    pub data_storage: Box<Account<'info, TransactionData>>,

    #[account(
        init_if_needed,
        payer = payer, 
        space = 8 + 8,
        seeds = [
            b"txn_count".as_ref(),
            &sender,
        ],
        bump
    )]
    pub txn_count: Box<Account<'info, Count>>,

    #[account(
        init, 
        payer = payer,
        space = 8 + 1,
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Box<Account<'info, TransactionStatus>>,
    #[account(
        seeds = [
            source_account.key().as_ref(),
        ],bump,
    )]
    /// CHECK: seeds has been checked
    pub zebec_vault: AccountInfo<'info>,
    #[account(
        init_if_needed,
        payer=source_account,
        seeds = [
            PREFIX_TOKEN.as_bytes(),
            source_account.key().as_ref(),
            mint.key().as_ref(),
        ],bump,
        space=8+8,
    )]
    pub withdraw_data: Box<Account<'info, TokenWithdraw>>,
    #[account(
        mut,
        seeds = [
            &sender,
            &from_chain_id.to_be_bytes()
        ],
        bump
    )]
    pub source_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program:Program<'info,Token>,
    pub associated_token_program:Program<'info,AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
    pub mint:Account<'info,Mint>,
    #[account(
        mut,
        constraint= source_account_token_account.owner == source_account.key(),
        constraint= source_account_token_account.mint == mint.key()
    )]
    pub source_account_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = zebec_vault,
    )]
    pub pda_account_token_account: Account<'info, TokenAccount>,
    pub zebec_program: Program<'info, Zebec>

}

#[derive(Accounts)]
#[instruction(
    eth_add:[u8;32],
    from_chain_id: u16,
    current_count: u64
)]
pub struct XstreamWithdraw<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer=payer,
        space= 8 + 8,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        bump,
    )]
    pub processed_vaa: Box<Account<'info, ProcessedVAA>>,
    pub emitter_acc: Box<Account<'info, EmitterAddrAccount>>,
    /// This requires some fancy hashing, so confirm it's derived address in the function itself.
    #[account(
        constraint = core_bridge_vaa.to_account_info().owner == &Pubkey::from_str(CORE_BRIDGE_ADDRESS).unwrap()
    )]
    /// CHECK: This account is owned by Core Bridge so we trust it
    pub core_bridge_vaa: AccountInfo<'info>,

    #[account(
        init,
        space = 8 + 156,
        payer = payer,
        seeds = [
            b"data_store".as_ref(),
            &eth_add, 
            &current_count.to_be_bytes()
        ],
        bump,
    )]
    pub data_storage: Box<Account<'info, TransactionData>>,

    #[account(
        init_if_needed,
        payer = payer, 
        space = 8 + 8,
        seeds = [
            b"txn_count".as_ref(),
            &eth_add,
        ],
        bump
    )]
    pub txn_count: Box<Account<'info, Count>>,

    #[account(
        init, 
        payer = payer,
        space = 8 + 1,
        seeds = [
            b"txn_status".as_ref(),
            &eth_add,
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Box<Account<'info, TransactionStatus>>,
    ///CHECK: seeds are checked while creating transaction,
    /// if different seeds passed the signature will not match
    #[account(
        mut,
        seeds = [
            &eth_add,
            &from_chain_id.to_be_bytes()
        ],
        bump
    )]
    pub pda_signer: UncheckedAccount<'info>,
    /// CHECK: seeds has been checked
    pub zebec_vault: AccountInfo<'info>,
    // #[account(mut)]
    // pub dest_account: Signer<'info>,
    #[account(mut)]
    /// CHECK: validated in data_account constraint
    pub source_account: AccountInfo<'info>,
    /// CHECK: validated in fee_vault constraint
    pub fee_owner:AccountInfo<'info>,
    pub fee_vault_data: Box<Account<'info, FeeVaultData>>,
    /// CHECK: seeds has been checked
    pub fee_vault:AccountInfo<'info>,
       #[account(mut,
            constraint= data_account.sender==source_account.key(),
            constraint= data_account.receiver==pda_signer.key(),    
            constraint= data_account.fee_owner==fee_owner.key(), 
            constraint= data_account.token_mint==mint.key(),          
        )]
    pub data_account:  Box<Account<'info, StreamToken>>,
    pub withdraw_data: Box<Account<'info, TokenWithdraw>>,
    pub system_program: Program<'info, System>,
    pub token_program:Program<'info,Token>,
    pub associated_token_program:Program<'info,AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
    pub mint:Account<'info,Mint>,
    #[account(
        associated_token::mint = mint,
        associated_token::authority = zebec_vault,
    )]
    pub pda_account_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = pda_signer,
        associated_token::mint = mint,
        associated_token::authority = pda_signer,
    )]
    pub dest_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = pda_signer,
        associated_token::mint = mint,
        associated_token::authority = fee_vault,
    )]
    pub fee_receiver_token_account: Box<Account<'info, TokenAccount>>,
    pub zebec_program: Program<'info, Zebec>
}

#[derive(Accounts)]
#[instruction(
    sender:[u8;32],
    from_chain_id: u16,
    current_count: u64
)]
pub struct XstreamPause<'info> {
    // ZEBEC's EOA.
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer=payer,
        space= 8 + 8,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        bump,
    )]
    pub processed_vaa: Box<Account<'info, ProcessedVAA>>,
    pub emitter_acc: Box<Account<'info, EmitterAddrAccount>>,
    /// This requires some fancy hashing, so confirm it's derived address in the function itself.
    #[account(
        constraint = core_bridge_vaa.to_account_info().owner == &Pubkey::from_str(CORE_BRIDGE_ADDRESS).unwrap()
    )]
    /// CHECK: This account is owned by Core Bridge so we trust it
    pub core_bridge_vaa: AccountInfo<'info>,

    #[account(
        init,
        space = 8 + 156,
        payer = payer,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &current_count.to_be_bytes()
        ],
        bump,
    )]
    pub data_storage: Box<Account<'info, TransactionData>>,

    #[account(
        init_if_needed,
        payer = payer, 
        space = 8 + 8,
        seeds = [
            b"txn_count".as_ref(),
            &sender,
        ],
        bump
    )]
    pub txn_count: Box<Account<'info, Count>>,

    #[account(
        init, 
        payer = payer,
        space = 8 + 1,
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Box<Account<'info, TransactionStatus>>,
    #[account(
        mut,
        seeds = [
            &sender,
            &from_chain_id.to_be_bytes()
        ],
        bump
    )]
    pub source_account: UncheckedAccount<'info>,
    /// CHECK: validated in data_account constraint
    pub dest_account: AccountInfo<'info>,
    #[account(mut,
        constraint = data_account.receiver == dest_account.key(),
        constraint = data_account.sender == source_account.key(),
        constraint= data_account.token_mint==mint.key(),
    )]
    pub data_account:  Account<'info, StreamToken>,
    pub mint:Account<'info,Mint>,
    #[account(
        mut,
        seeds = [
            PREFIX_TOKEN.as_bytes(),
            source_account.key().as_ref(),
            mint.key().as_ref(),
        ],bump,
    )]
    pub withdraw_data: Box<Account<'info, TokenWithdraw>>,
    pub system_program: Program<'info, System>,
    pub zebec_program: Program<'info, Zebec>
    
}

#[derive(Accounts)]
#[instruction(
    sender:[u8;32],
    from_chain_id: u16,
    current_count: u64
)]
pub struct XstreamCancel<'info> {
    // ZEBEC's EOA.
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer=payer,
        space= 8 + 8,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        bump,
    )]
    pub processed_vaa: Box<Account<'info, ProcessedVAA>>,
    pub emitter_acc: Box<Account<'info, EmitterAddrAccount>>,
    /// This requires some fancy hashing, so confirm it's derived address in the function itself.
    #[account(
        constraint = core_bridge_vaa.to_account_info().owner == &Pubkey::from_str(CORE_BRIDGE_ADDRESS).unwrap()
    )]
    /// CHECK: This account is owned by Core Bridge so we trust it
    pub core_bridge_vaa: AccountInfo<'info>,

    #[account(
        init,
        space = 8 + 156,
        payer = payer,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &current_count.to_be_bytes()
        ],
        bump,
    )]
    pub data_storage: Box<Account<'info, TransactionData>>,

    #[account(
        init_if_needed,
        payer = payer, 
        space = 8 + 8,
        seeds = [
            b"txn_count".as_ref(),
            &sender,
        ],
        bump
    )]
    pub txn_count: Box<Account<'info, Count>>,

    #[account(
        init, 
        payer = payer,
        space = 8 + 1,
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Box<Account<'info, TransactionStatus>>,
    #[account(
        seeds = [
            source_account.key().as_ref(),
        ],bump,
    )]
    /// CHECK: seeds has been checked
    pub zebec_vault: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: validated in data_account constraint
    pub dest_account: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            &sender,
            &from_chain_id.to_be_bytes()
        ],
        bump
    )]
    pub source_account: UncheckedAccount<'info>,
    /// CHECK: validated in fee_vault constraint
    pub fee_owner:AccountInfo<'info>, 
    #[account(
        seeds = [
            fee_owner.key().as_ref(),
            OPERATEDATA.as_bytes(),
            fee_vault.key().as_ref(),
        ],bump
    )]
    pub fee_vault_data: Account<'info,FeeVaultData>, 
    #[account(
        constraint = fee_vault_data.fee_owner == fee_owner.key(),
        constraint = fee_vault_data.fee_vault_address == fee_vault.key(),
        seeds = [
            fee_owner.key().as_ref(),
            OPERATE.as_bytes(),          
        ],bump,       
    )]
    /// CHECK: seeds has been checked
    pub fee_vault:AccountInfo<'info>, 
    #[account(mut,
        constraint= data_account.sender==source_account.key(),
        constraint= data_account.receiver==dest_account.key(),   
        constraint= data_account.fee_owner==fee_owner.key(),   
        close = source_account //to close the data account and send rent exempt lamports to sender       
    )]
    pub data_account:  Account<'info, StreamToken>,
    #[account(
        mut,
        seeds = [
            PREFIX_TOKEN.as_bytes(),
            source_account.key().as_ref(),
            mint.key().as_ref(),
        ],bump,
    )]
    pub withdraw_data: Box<Account<'info, TokenWithdraw>>,
    pub system_program: Program<'info, System>,
    pub token_program:Program<'info,Token>,
    pub associated_token_program:Program<'info,AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
    pub mint:Account<'info,Mint>,
    #[account(
        init_if_needed,
        payer = source_account,
        associated_token::mint = mint,
        associated_token::authority = zebec_vault,
    )]
    pub pda_account_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = source_account,
        associated_token::mint = mint,
        associated_token::authority = dest_account,
    )]
    pub dest_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = source_account,
        associated_token::mint = mint,
        associated_token::authority = fee_vault,
    )]
    pub fee_receiver_token_account: Box<Account<'info, TokenAccount>>,
    pub zebec_program: Program<'info, Zebec>

}

#[derive(Accounts)]
#[instruction(
    sender:[u8;32],
    from_chain_id: u16,
    current_count: u64
)]
pub struct XstreamInstant<'info> {
    // ZEBEC's EOA.
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer=payer,
        space= 8 + 8,
        seeds=[
            &decode(&emitter_acc.emitter_addr.as_str()).unwrap()[..],
            emitter_acc.chain_id.to_be_bytes().as_ref(),
            (PostedMessageData::try_from_slice(&core_bridge_vaa.data.borrow())?.0).sequence.to_be_bytes().as_ref()
        ],
        bump,
    )]
    pub processed_vaa: Box<Account<'info, ProcessedVAA>>,
    pub emitter_acc: Box<Account<'info, EmitterAddrAccount>>,
    /// This requires some fancy hashing, so confirm it's derived address in the function itself.
    #[account(
        constraint = core_bridge_vaa.to_account_info().owner == &Pubkey::from_str(CORE_BRIDGE_ADDRESS).unwrap()
    )]
    /// CHECK: This account is owned by Core Bridge so we trust it
    pub core_bridge_vaa: AccountInfo<'info>,

    #[account(
        init,
        space = 8 + 156,
        payer = payer,
        seeds = [
            b"data_store".as_ref(),
            &sender, 
            &current_count.to_be_bytes()
        ],
        bump,
    )]
    pub data_storage: Box<Account<'info, TransactionData>>,

    #[account(
        init_if_needed,
        payer = payer, 
        space = 8 + 8,
        seeds = [
            b"txn_count".as_ref(),
            &sender,
        ],
        bump
    )]
    pub txn_count: Box<Account<'info, Count>>,

    #[account(
        init, 
        payer = payer,
        space = 8 + 1,
        seeds = [
            b"txn_status".as_ref(),
            &sender,
            &current_count.to_be_bytes()
        ],
        bump
    )]
    pub txn_status: Box<Account<'info, TransactionStatus>>,
    #[account(
        seeds = [
            source_account.key().as_ref(),
        ],bump,
    )]
    /// CHECK: seeds has been checked
    pub zebec_vault: AccountInfo<'info>,
    /// CHECK: This is the receiver account, since the funds are transferred directly, we do not need to check it
    #[account(mut)]
    pub dest_account: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            &sender,
            &from_chain_id.to_be_bytes()
        ],
        bump
    )]
    pub source_account: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer=source_account,
        seeds = [
            PREFIX_TOKEN.as_bytes(),
            source_account.key().as_ref(),
            mint.key().as_ref(),
        ],bump,
        space=8+8,
    )]
    pub withdraw_data: Box<Account<'info, TokenWithdraw>>,
    pub system_program: Program<'info, System>,
    pub token_program:Program<'info,Token>,
    pub associated_token_program:Program<'info,AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
    pub mint:Account<'info,Mint>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = zebec_vault,
    )]
    pub pda_account_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = source_account,
        associated_token::mint = mint,
        associated_token::authority = dest_account,
    )]
    pub dest_token_account: Box<Account<'info, TokenAccount>>,
    pub zebec_program: Program<'info, Zebec>
}
