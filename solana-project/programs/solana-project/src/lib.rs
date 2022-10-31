use anchor_lang::prelude::*;
use anchor_lang::solana_program::borsh::try_from_slice_unchecked;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::system_instruction::transfer;
// use anchor_lang::solana_program::keccak::hashv;
// use anchor_lang::solana_program::keccak::Hash;
use anchor_lang::solana_program;
use anchor_spl::token::{approve, Approve};

use primitive_types::U256;
use sha3::Digest;

use byteorder::{BigEndian, WriteBytesExt};
use hex::decode;
use std::io::{Cursor, Write};
use std::str::FromStr;
mod constants;
mod context;
mod errors;
mod portal;
mod state;
mod wormhole;

use constants::*;
use context::*;
use errors::*;
use portal::*;
use state::*;
use wormhole::*;

use std::ops::Deref;

declare_id!("GtyAQgcYTGso352pgR7T8tfESe3TGE5eUkEj9dYyrypS");

#[program]
pub mod solana_project {

    use anchor_lang::solana_program::program::invoke_signed;

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.config.owner = ctx.accounts.owner.key();
        ctx.accounts.config.nonce = 1;
        Ok(())
    }

    pub fn register_chain(
        ctx: Context<RegisterChain>,
        chain_id: u16,
        emitter_addr: String,
    ) -> Result<()> {
        require!( emitter_addr.len() == EVM_CHAIN_ADDRESS_LENGTH, MessengerError::InvalidEmitterAddress);
        
        ctx.accounts.emitter_acc.chain_id = chain_id;
        ctx.accounts.emitter_acc.emitter_addr = emitter_addr;
        Ok(())
    }

    pub fn send_msg(ctx: Context<SendMsg>, msg: String) -> Result<()> {
        //Look Up Fee
        let bridge_data: BridgeData =
            try_from_slice_unchecked(&ctx.accounts.wormhole_config.data.borrow_mut())?;

        //Send Fee
        invoke_signed(
            &transfer(
                &ctx.accounts.payer.key(),
                &ctx.accounts.wormhole_fee_collector.key(),
                bridge_data.config.fee,
            ),
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.wormhole_fee_collector.to_account_info(),
            ],
            &[],
        )?;

        //Send Post Msg Tx
        let sendmsg_ix = Instruction {
            program_id: ctx.accounts.core_bridge.key(),
            accounts: vec![
                AccountMeta::new(ctx.accounts.wormhole_config.key(), false),
                AccountMeta::new(ctx.accounts.wormhole_message_key.key(), true),
                AccountMeta::new_readonly(ctx.accounts.wormhole_derived_emitter.key(), true),
                AccountMeta::new(ctx.accounts.wormhole_sequence.key(), false),
                AccountMeta::new(ctx.accounts.payer.key(), true),
                AccountMeta::new(ctx.accounts.wormhole_fee_collector.key(), false),
                AccountMeta::new_readonly(ctx.accounts.clock.key(), false),
                AccountMeta::new_readonly(ctx.accounts.rent.key(), false),
                AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
            ],
            data: (
                wormhole::Instruction::PostMessage,
                PostMessageData {
                    nonce: ctx.accounts.config.nonce,
                    payload: msg.as_bytes().try_to_vec()?,
                    consistency_level: wormhole::ConsistencyLevel::Confirmed,
                },
            )
                .try_to_vec()?,
        };

        invoke_signed(
            &sendmsg_ix,
            &[
                ctx.accounts.wormhole_config.to_account_info(),
                ctx.accounts.wormhole_message_key.to_account_info(),
                ctx.accounts.wormhole_derived_emitter.to_account_info(),
                ctx.accounts.wormhole_sequence.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.wormhole_fee_collector.to_account_info(),
                ctx.accounts.clock.to_account_info(),
                ctx.accounts.rent.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[&[
                &b"emitter".as_ref(),
                &[*ctx.bumps.get("wormhole_derived_emitter").unwrap()],
            ]],
        )?;

        let sum = ctx.accounts.config.nonce.checked_add(1);
        match sum {
            None => return Err(MessengerError::Overflow.into()),
            Some (val) => ctx.accounts.config.nonce = val,
        }

        Ok(())
    }

    pub fn store_msg(ctx: Context<StoreMsg>, current_count: u8, sender: [u8; 32]) -> Result<()> {
        //Hash a VAA Extract and derive a VAA Key
        let vaa = PostedMessageData::try_from_slice(&ctx.accounts.core_bridge_vaa.data.borrow())?.0;
        let serialized_vaa = serialize_vaa(&vaa);

        let mut h = sha3::Keccak256::default();
        h.write_all(serialized_vaa.as_slice()).unwrap();
        let vaa_hash: [u8; 32] = h.finalize().into();

        let (vaa_key, _) = Pubkey::find_program_address(
            &[b"PostedVAA", &vaa_hash],
            &Pubkey::from_str(CORE_BRIDGE_ADDRESS).unwrap(),
        );

        if ctx.accounts.core_bridge_vaa.key() != vaa_key {
            return err!(MessengerError::VAAKeyMismatch);
        }

        // Already checked that the SignedVaa is owned by core bridge in account constraint logic
        // Check that the emitter chain and address match up with the vaa
        if vaa.emitter_chain != ctx.accounts.emitter_acc.chain_id
            || vaa.emitter_address
                != &decode(&ctx.accounts.emitter_acc.emitter_addr.as_str()).unwrap()[..]
        {
            return err!(MessengerError::VAAEmitterMismatch);
        }

        // Encoded String
        let encoded_str = vaa.payload.clone();

        // Decode Encoded String and Store Value based upon the code sent on message passing
        let code = get_u8(encoded_str[0..1].to_vec());

        // Change Transaction Count to Current Count
        let txn_count = &mut ctx.accounts.txn_count;
        let sum = txn_count.count.checked_add(1);

         match sum {
            None => return Err(MessengerError::Overflow.into()),
            Some (val) => txn_count.count = val,
        }

        let count_stored = ctx.accounts.txn_count.count;
        require!(
            count_stored == current_count,
            MessengerError::InvalidDataProvided
        );

        // Switch Based on the code
        //TODO: return error if otherwise
        match code {
            2 => process_stream(encoded_str, vaa.emitter_chain, ctx),
            4 => process_withdraw_stream(encoded_str, vaa.emitter_chain, ctx),
            6 => process_deposit(encoded_str, vaa.emitter_chain, ctx),
            8 => process_pause(encoded_str, vaa.emitter_chain, ctx),
            10 => process_withdraw(encoded_str, vaa.emitter_chain, ctx),
            12 => process_instant_transfer(encoded_str, vaa.emitter_chain, ctx),
            14 => process_update_stream(encoded_str, vaa.emitter_chain, ctx),
            16 => process_cancel_stream(encoded_str, vaa.emitter_chain, ctx),
            17 => process_direct_transfer(encoded_str, vaa.emitter_chain, ctx),
            _ => return err!(MessengerError::InvalidPayload),
        }
        Ok(())
    }

    //creates and executes deposit transaction
    pub fn transaction_deposit(
        ctx: Context<CETransaction>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
        current_count: u8,
        chain_id: Vec<u8>,
        sender: [u8; 32],
    ) -> Result<()> {
        //Build Transactions
        let tx = &mut ctx.accounts.transaction;
        tx.program_id = pid;
        tx.accounts = accs.clone();
        tx.did_execute = false;
        tx.data = data.clone();

        let count_stored = ctx.accounts.txn_count.count;
        require!(
            count_stored == current_count,
            MessengerError::InvalidDataProvided
        );

        //check Mint passed
        let mint_pubkey_passed: Pubkey = accs[6].pubkey;
        require!(
            mint_pubkey_passed == ctx.accounts.data_storage.token_mint,
            MessengerError::InvalidDataProvided
        );

        //check sender
        let pda_sender_passed: Pubkey = accs[1].pubkey;
        let sender_stored = ctx.accounts.data_storage.sender.clone();
        require!(sender.to_vec() == sender_stored, MessengerError::InvalidDataProvided);

        //check pdaSender
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let derived_pubkey: (Pubkey, u8) =
            Pubkey::find_program_address(&[&sender[..], &chain_id_seed[..]], &ctx.program_id);
        require!(
            pda_sender_passed == derived_pubkey.0,
            MessengerError::InvalidPDASigner
        );

        //check data params passed
        let data: &[u8] = data.as_slice();
        let data_slice = &data[8..];
        let decode_data = TokenAmount::try_from_slice(data_slice)?;
        let amount_passed = decode_data.amount;
        require!(
            amount_passed == ctx.accounts.data_storage.amount,
            MessengerError::InvalidDataProvided
        );

        //execute txn
        if ctx.accounts.transaction.did_execute {
            return Err(MessengerError::AlreadyExecuted.into());
        }
        // Burn the transaction to ensure one time use.
        ctx.accounts.transaction.did_execute = true;

        // Execute the transaction signed by the pdasender/pdareceiver.
        let mut ix: Instruction = (*ctx.accounts.transaction).deref().into();
        ix.accounts = ix
            .accounts
            .iter()
            .map(|acc| {
                let mut acc = acc.clone();
                if &acc.pubkey == ctx.accounts.pda_signer.key {
                    acc.is_signer = true;
                }
                acc
            })
            .collect();

        let bump = ctx.bumps.get("pda_signer").unwrap().to_le_bytes();
        let seeds: &[&[_]] = &[&sender, &chain_id, bump.as_ref()];
        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;

        msg!("Transaction Execute");

        solana_program::program::invoke_signed(&ix, accounts, signer)?;
        Ok(())
    }

    //creates transaction stream.
    //Txn size too high so spliting creation and execution
    pub fn create_transaction_stream(
        ctx: Context<CreateTransaction>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
        current_count: u8,
        sender: [u8; 32],
    ) -> Result<()> {
        //Build Transactions
        let tx = &mut ctx.accounts.transaction;
        tx.program_id = pid;
        tx.accounts = accs.clone();
        tx.did_execute = false;
        tx.data = data.clone();

        let count_stored = ctx.accounts.txn_count.count;
        require!(
            count_stored == current_count,
            MessengerError::InvalidDataProvided
        );

        //check Mint passed
        let mint_pubkey_passed: Pubkey = accs[9].pubkey;
        require!(
            mint_pubkey_passed == ctx.accounts.data_storage.token_mint,
            MessengerError::InvalidDataProvided
        );

        //check sender
        let pda_sender_passed: Pubkey = accs[5].pubkey;
        let sender_stored = ctx.accounts.data_storage.sender.clone();
        require!(sender.to_vec() == sender_stored, MessengerError::InvalidDataProvided);

        //check receiver
        let pda_receiver_passed: Pubkey = accs[6].pubkey;
        let receiver_stored = ctx.accounts.data_storage.receiver.clone();

        //check pdaSender
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let sender_derived_pubkey: (Pubkey, u8) =
            Pubkey::find_program_address(&[&sender[..], &chain_id_seed[..]], &ctx.program_id);
        require!(
            pda_sender_passed == sender_derived_pubkey.0,
            MessengerError::InvalidPDASigner
        );

        //check pdaReceiver
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let receiver_derived_pubkey: (Pubkey, u8) = Pubkey::find_program_address(
            &[&receiver_stored[..], &chain_id_seed[..]],
            &ctx.program_id,
        );
        require!(
            pda_receiver_passed == receiver_derived_pubkey.0,
            MessengerError::InvalidDataProvided
        );

        //check data params passed
        let data: &[u8] = data.as_slice();
        let data_slice = &data[8..];
        let decode_data = Stream::try_from_slice(data_slice)?;
        require!(
            decode_data.amount == ctx.accounts.data_storage.amount,
            MessengerError::InvalidDataProvided
        );
        require!(
            decode_data.start_time == ctx.accounts.data_storage.start_time,
            MessengerError::InvalidDataProvided
        );
        require!(
            decode_data.end_time == ctx.accounts.data_storage.end_time,
            MessengerError::InvalidDataProvided
        );
        require!(
            decode_data.can_cancel == ctx.accounts.data_storage.can_cancel,
            MessengerError::InvalidDataProvided
        );
        require!(
            decode_data.can_update == ctx.accounts.data_storage.can_update,
            MessengerError::InvalidDataProvided
        );

        Ok(())
    }

    //creates and executes transaction stream update
    pub fn transaction_stream_update(
        ctx: Context<CETransaction>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
        current_count: u8,
        chain_id: Vec<u8>,
        sender: [u8; 32],
    ) -> Result<()> {
        //Build Transactions
        let tx = &mut ctx.accounts.transaction;
        tx.program_id = pid;
        tx.accounts = accs.clone();
        tx.did_execute = false;
        tx.data = data.clone();

        let count_stored = ctx.accounts.txn_count.count;
        require!(
            count_stored == current_count,
            MessengerError::InvalidDataProvided
        );

        //check Mint passed
        let mint_pubkey_passed: Pubkey = accs[4].pubkey;
        require!(
            mint_pubkey_passed == ctx.accounts.data_storage.token_mint,
            MessengerError::InvalidDataProvided
        );

        //check data account
        let data_account_passed: Pubkey = accs[0].pubkey;
        require!(
            data_account_passed == ctx.accounts.data_storage.data_account,
            MessengerError::InvalidDataProvided
        );

        //check sender
        let pda_sender_passed: Pubkey = accs[2].pubkey;
        let sender_stored = ctx.accounts.data_storage.sender.clone();
        require!(sender.to_vec() == sender_stored, MessengerError::InvalidDataProvided);

        //check receiver
        let pda_receiver_passed: Pubkey = accs[3].pubkey;
        let receiver_stored = ctx.accounts.data_storage.receiver.clone();

        //check pdaSender
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let sender_derived_pubkey: (Pubkey, u8) =
            Pubkey::find_program_address(&[&sender[..], &chain_id_seed[..]], &ctx.program_id);
        require!(
            pda_sender_passed == sender_derived_pubkey.0,
            MessengerError::InvalidPDASigner
        );

        //check pdaReceiver
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let receiver_derived_pubkey: (Pubkey, u8) = Pubkey::find_program_address(
            &[&receiver_stored[..], &chain_id_seed[..]],
            &ctx.program_id,
        );
        require!(
            pda_receiver_passed == receiver_derived_pubkey.0,
            MessengerError::InvalidDataProvided
        );

        //check data params passed
        let data: &[u8] = data.as_slice();
        let data_slice = &data[8..];
        let decode_data = StreamUpdate::try_from_slice(data_slice)?;
        require!(
            decode_data.amount == ctx.accounts.data_storage.amount,
            MessengerError::InvalidDataProvided
        );
        require!(
            decode_data.start_time == ctx.accounts.data_storage.start_time,
            MessengerError::InvalidDataProvided
        );
        require!(
            decode_data.end_time == ctx.accounts.data_storage.end_time,
            MessengerError::InvalidDataProvided
        );

        //execute txn
        if ctx.accounts.transaction.did_execute {
            return Err(MessengerError::AlreadyExecuted.into());
        }
        // Burn the transaction to ensure one time use.
        ctx.accounts.transaction.did_execute = true;

        // Execute the transaction signed by the pdasender/pdareceiver.
        let mut ix: Instruction = (*ctx.accounts.transaction).deref().into();
        ix.accounts = ix
            .accounts
            .iter()
            .map(|acc| {
                let mut acc = acc.clone();
                if &acc.pubkey == ctx.accounts.pda_signer.key {
                    acc.is_signer = true;
                }
                acc
            })
            .collect();

        let bump = ctx.bumps.get("pda_signer").unwrap().to_le_bytes();
        let seeds: &[&[_]] = &[&sender, &chain_id, bump.as_ref()];
        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;

        msg!("Transaction Execute");

        solana_program::program::invoke_signed(&ix, accounts, signer)?;
        Ok(())
    }

    //creates and execute pause/resume stream
    pub fn transaction_pause_resume(
        ctx: Context<CETransaction>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
        current_count: u8,
        chain_id: Vec<u8>,
        sender: [u8; 32],
    ) -> Result<()> {
        //Build Transactions
        let tx = &mut ctx.accounts.transaction;
        tx.program_id = pid;
        tx.accounts = accs.clone();
        tx.did_execute = false;
        tx.data = data.clone();

        let count_stored = ctx.accounts.txn_count.count;
        require!(
            count_stored == current_count,
            MessengerError::InvalidDataProvided
        );

        //check Mint passed
        // TODO: will be added in the later version of zebec contract
        // let mint_pubkey_passed: Pubkey = accs[4].pubkey;
        // require!(mint_pubkey_passed == ctx.accounts.data_storage.token_mint, MessengerError::InvalidDataProvided);

        //check data account
        let data_account_passed: Pubkey = accs[2].pubkey;
        require!(
            data_account_passed == ctx.accounts.data_storage.data_account,
            MessengerError::InvalidDataProvided
        );

        //check sender
        let pda_sender_passed: Pubkey = accs[0].pubkey;
        let sender_stored = ctx.accounts.data_storage.sender.clone();
        require!(sender.to_vec() == sender_stored, MessengerError::InvalidDataProvided);

        //check receiver
        let pda_receiver_passed: Pubkey = accs[1].pubkey;
        let receiver_stored = ctx.accounts.data_storage.receiver.clone();

        //check pdaSender
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let sender_derived_pubkey: (Pubkey, u8) =
            Pubkey::find_program_address(&[&sender[..], &chain_id_seed[..]], &ctx.program_id);
        require!(
            pda_sender_passed == sender_derived_pubkey.0,
            MessengerError::InvalidPDASigner
        );

        //check pdaReceiver
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let receiver_derived_pubkey: (Pubkey, u8) = Pubkey::find_program_address(
            &[&receiver_stored[..], &chain_id_seed[..]],
            &ctx.program_id,
        );
        require!(
            pda_receiver_passed == receiver_derived_pubkey.0,
            MessengerError::InvalidDataProvided
        );

        //check data params passed (no params passed)
        // execute txn
        if ctx.accounts.transaction.did_execute {
            return Err(MessengerError::AlreadyExecuted.into());
        }
        // Burn the transaction to ensure one time use.
        ctx.accounts.transaction.did_execute = true;

        // Execute the transaction signed by the pdasender/pdareceiver.
        let mut ix: Instruction = (*ctx.accounts.transaction).deref().into();
        ix.accounts = ix
            .accounts
            .iter()
            .map(|acc| {
                let mut acc = acc.clone();
                if &acc.pubkey == ctx.accounts.pda_signer.key {
                    acc.is_signer = true;
                }
                acc
            })
            .collect();

        let bump = ctx.bumps.get("pda_signer").unwrap().to_le_bytes();
        let seeds: &[&[_]] = &[&sender, &chain_id, bump.as_ref()];
        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;

        msg!("Transaction Execute");

        solana_program::program::invoke_signed(&ix, accounts, signer)?;
        Ok(())
    }

    // sender is stream token receiver
    // create and then execute
    pub fn create_transaction_receiver_withdraw(
        ctx: Context<CreateTransactionReceiver>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
        current_count: u8,
        sender: [u8; 32],
    ) -> Result<()> {
        //Build Transactions
        let tx = &mut ctx.accounts.transaction;
        tx.program_id = pid;
        tx.accounts = accs.clone();
        tx.did_execute = false;
        tx.data = data.clone();

        let count_stored = ctx.accounts.txn_count.count;
        require!(
            count_stored == current_count,
            MessengerError::InvalidDataProvided
        );

        //check Mint passed
        let mint_pubkey_passed: Pubkey = accs[12].pubkey;
        require!(
            mint_pubkey_passed == ctx.accounts.data_storage.token_mint,
            MessengerError::InvalidDataProvided
        );

        //check data account
        let data_account_passed: Pubkey = accs[6].pubkey;
        require!(
            data_account_passed == ctx.accounts.data_storage.data_account,
            MessengerError::InvalidDataProvided
        );

        //check sender
        let pda_sender_passed: Pubkey = accs[2].pubkey;
        let sender_stored = ctx.accounts.data_storage.sender.clone();

        //check receiver
        let pda_receiver_passed: Pubkey = accs[1].pubkey;
        let receiver_stored = ctx.accounts.data_storage.receiver.clone();
        require!(
            sender.to_vec() == receiver_stored,
            MessengerError::InvalidDataProvided
        );

        //check pdaSender
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let sender_derived_pubkey: (Pubkey, u8) = Pubkey::find_program_address(
            &[&sender_stored[..], &chain_id_seed[..]],
            &ctx.program_id,
        );
        require!(
            pda_sender_passed == sender_derived_pubkey.0,
            MessengerError::InvalidPDASigner
        );

        //check pdaReceiver
        let receiver_derived_pubkey: (Pubkey, u8) = Pubkey::find_program_address(
            &[&receiver_stored[..], &chain_id_seed[..]],
            &ctx.program_id,
        );
        require!(
            pda_receiver_passed == receiver_derived_pubkey.0,
            MessengerError::InvalidDataProvided
        );

        //check data params passed
        Ok(())
    }

    // creates transaction cancel
    pub fn create_transaction_cancel(
        ctx: Context<CreateTransaction>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
        current_count: u8,
        sender: [u8; 32],
    ) -> Result<()> {
        //Build Transactions
        let tx = &mut ctx.accounts.transaction;
        tx.program_id = pid;
        tx.accounts = accs.clone();
        tx.did_execute = false;
        tx.data = data.clone();

        let count_stored = ctx.accounts.txn_count.count;
        require!(
            count_stored == current_count,
            MessengerError::InvalidDataProvided
        );

        //check Mint passed
        let mint_pubkey_passed: Pubkey = accs[12].pubkey;
        require!(
            mint_pubkey_passed == ctx.accounts.data_storage.token_mint,
            MessengerError::InvalidDataProvided
        );

        //check data account
        let data_account_passed: Pubkey = accs[6].pubkey;
        require!(
            data_account_passed == ctx.accounts.data_storage.data_account,
            MessengerError::InvalidDataProvided
        );

        //check sender
        let pda_sender_passed: Pubkey = accs[2].pubkey;
        let sender_stored = ctx.accounts.data_storage.sender.clone();
        require!(sender.to_vec() == sender_stored, MessengerError::InvalidDataProvided);

        //check receiver
        let pda_receiver_passed: Pubkey = accs[1].pubkey;
        let receiver_stored = ctx.accounts.data_storage.receiver.clone();

        //check pdaSender
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let sender_derived_pubkey: (Pubkey, u8) =
            Pubkey::find_program_address(&[&sender[..], &chain_id_seed[..]], &ctx.program_id);
        require!(
            pda_sender_passed == sender_derived_pubkey.0,
            MessengerError::InvalidPDASigner
        );

        //check pdaReceiver
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let receiver_derived_pubkey: (Pubkey, u8) = Pubkey::find_program_address(
            &[&receiver_stored[..], &chain_id_seed[..]],
            &ctx.program_id,
        );
        require!(
            pda_receiver_passed == receiver_derived_pubkey.0,
            MessengerError::InvalidDataProvided
        );

        //check data params passed
        Ok(())
    }

    // create transaction
    pub fn create_transaction_sender_withdraw(
        ctx: Context<CreateTransaction>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
        current_count: u8,
        sender: [u8; 32],
    ) -> Result<()> {
        //Build Transactions
        let tx = &mut ctx.accounts.transaction;
        tx.program_id = pid;
        tx.accounts = accs.clone();
        tx.did_execute = false;
        tx.data = data.clone();

        let count_stored = ctx.accounts.txn_count.count;
        require!(
            count_stored == current_count,
            MessengerError::InvalidDataProvided
        );

        //check Mint passed
        let mint_pubkey_passed: Pubkey = accs[7].pubkey;
        require!(
            mint_pubkey_passed == ctx.accounts.data_storage.token_mint,
            MessengerError::InvalidDataProvided
        );

        //check sender
        let pda_sender_passed: Pubkey = accs[2].pubkey;
        let sender_stored = ctx.accounts.data_storage.sender.clone();
        require!(sender.to_vec() == sender_stored, MessengerError::InvalidDataProvided);

        //check pdaSender
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let sender_derived_pubkey: (Pubkey, u8) =
            Pubkey::find_program_address(&[&sender[..], &chain_id_seed[..]], &ctx.program_id);
        require!(
            pda_sender_passed == sender_derived_pubkey.0,
            MessengerError::InvalidPDASigner
        );

        //check data params passed
        let data: &[u8] = data.as_slice();
        let data_slice = &data[8..];
        let decode_data = TokenAmount::try_from_slice(data_slice)?;
        require!(
            decode_data.amount == ctx.accounts.data_storage.amount,
            MessengerError::InvalidDataProvided
        );

        Ok(())
    }

    // create transaction
    pub fn create_transaction_instant_transfer(
        ctx: Context<CreateTransaction>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
        current_count: u8,
        sender: [u8; 32],
    ) -> Result<()> {
        //Build Transactions
        let tx = &mut ctx.accounts.transaction;
        tx.program_id = pid;
        tx.accounts = accs.clone();
        tx.did_execute = false;
        tx.data = data.clone();

        let count_stored = ctx.accounts.txn_count.count;
        require!(
            count_stored == current_count,
            MessengerError::InvalidDataProvided
        );

        //check Mint passed
        let mint_pubkey_passed: Pubkey = accs[8].pubkey;
        require!(
            mint_pubkey_passed == ctx.accounts.data_storage.token_mint,
            MessengerError::InvalidDataProvided
        );

        //check sender
        let pda_sender_passed: Pubkey = accs[2].pubkey;
        let sender_stored = ctx.accounts.data_storage.sender.clone();
        require!(sender.to_vec() == sender_stored, MessengerError::InvalidDataProvided);

        //check receiver
        let pda_receiver_passed: Pubkey = accs[1].pubkey;
        let receiver_stored = ctx.accounts.data_storage.receiver.clone();

        //check pdaSender
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let sender_derived_pubkey: (Pubkey, u8) =
            Pubkey::find_program_address(&[&sender[..], &chain_id_seed[..]], &ctx.program_id);
        require!(
            pda_sender_passed == sender_derived_pubkey.0,
            MessengerError::InvalidPDASigner
        );

        //check pdaReceiver
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let receiver_derived_pubkey: (Pubkey, u8) = Pubkey::find_program_address(
            &[&receiver_stored[..], &chain_id_seed[..]],
            &ctx.program_id,
        );
        require!(
            pda_receiver_passed == receiver_derived_pubkey.0,
            MessengerError::InvalidDataProvided
        );

        //check data params passed
        let data: &[u8] = data.as_slice();
        let data_slice = &data[8..];
        let decode_data = TokenAmount::try_from_slice(data_slice)?;
        require!(
            decode_data.amount == ctx.accounts.data_storage.amount,
            MessengerError::InvalidDataProvided
        );

        Ok(())
    }

    //create and execute direct transfer native
    pub fn transaction_direct_transfer_native(
        ctx: Context<DirectTransferNative>,
        sender: [u8; 32],
        chain_id: Vec<u8>,
        current_count: u8,
        target_chain: u16,
        fee: u64,
    ) -> Result<()> {
        let count_stored = ctx.accounts.txn_count.count;
        require!(
            count_stored == current_count,
            MessengerError::InvalidDataProvided
        );

        require!(
            ctx.accounts.data_storage.token_mint == ctx.accounts.mint.key(),
            MessengerError::InvalidDataProvided
        );

        //check sender
        let sender_stored = ctx.accounts.data_storage.sender.clone();
        require!(sender.to_vec() == sender_stored, MessengerError::InvalidDataProvided);

        //check receiver
        let receiver_stored = ctx.accounts.data_storage.receiver.clone();

        //check pdaSender
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let (sender_derived_pubkey, _): (Pubkey, u8) =
            Pubkey::find_program_address(&[&sender[..], &chain_id_seed[..]], &ctx.program_id);
        require!(
            ctx.accounts.pda_signer.key() == sender_derived_pubkey,
            MessengerError::InvalidPDASigner
        );

        msg!("Transaction Direct Transfer Native");

        transfer_native(ctx, sender, chain_id, target_chain, fee, receiver_stored)
    }

    //create and execute direct transfer wrapped
    pub fn transaction_direct_transfer_wrapped(
        ctx: Context<DirectTransferWrapped>,
        sender: [u8; 32],
        sender_chain: Vec<u8>,
        token_address: Vec<u8>,
        token_chain: u16,
        current_count: u8,
        target_chain: u16,
        amount: u64,
        fee: u64,
    ) -> Result<()> {
        let count_stored = ctx.accounts.txn_count.count;
        require!(
            count_stored == current_count,
            MessengerError::InvalidDataProvided
        );

        require!(
            ctx.accounts.data_storage.token_mint == ctx.accounts.wrapped_mint.key(),
            MessengerError::InvalidDataProvided
        );

        //check sender
        let sender_stored = ctx.accounts.data_storage.sender.clone();
        require!(sender.to_vec() == sender_stored, MessengerError::InvalidDataProvided);

        //check receiver
        let receiver_stored = ctx.accounts.data_storage.receiver.clone();

        //check pdaSender
        let chain_id_stored = (ctx.accounts.data_storage.from_chain_id).to_string();
        let chain_id_seed = chain_id_stored.as_bytes();
        let (sender_derived_pubkey, _): (Pubkey, u8) =
            Pubkey::find_program_address(&[&sender[..], &chain_id_seed[..]], &ctx.program_id);
        require!(
            ctx.accounts.pda_signer.key() == sender_derived_pubkey,
            MessengerError::InvalidPDASigner
        );

        msg!("Transaction Direct Transfer Wrapped");

        transfer_wrapped(
            ctx,
            sender.to_vec(),
            sender_chain,
            target_chain,
            fee,
            receiver_stored,
        )
    }

    pub fn execute_transaction(
        ctx: Context<ExecuteTransaction>,
        from_chain_id: Vec<u8>,
        eth_add: Vec<u8>,
    ) -> Result<()> {
        // params if passed incorrecrtly the signature will not work and the txn will panic.
        // Has this been executed already?
        if ctx.accounts.transaction.did_execute {
            return Err(MessengerError::AlreadyExecuted.into());
        }
        // Burn the transaction to ensure one time use.
        ctx.accounts.transaction.did_execute = true;

        // Execute the transaction signed by the pdasender/pdareceiver.
        let mut ix: Instruction = (*ctx.accounts.transaction).deref().into();
        ix.accounts = ix
            .accounts
            .iter()
            .map(|acc| {
                let mut acc = acc.clone();
                if &acc.pubkey == ctx.accounts.pda_signer.key {
                    acc.is_signer = true;
                }
                acc
            })
            .collect();

        let bump = ctx.bumps.get("pda_signer").unwrap().to_le_bytes();
        let seeds: &[&[_]] = &[&eth_add, &from_chain_id, bump.as_ref()];
        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;

        msg!("Transaction Execute");

        solana_program::program::invoke_signed(&ix, accounts, signer)?;

        Ok(())
    }

    pub fn transfer_wrapped(
        ctx: Context<DirectTransferWrapped>,
        sender: Vec<u8>,
        sender_chain: Vec<u8>,
        target_chain: u16,
        fee: u64,
        receiver: Vec<u8>,
    ) -> Result<()> {
        let amount = ctx.accounts.data_storage.amount;

        //Check EOA
        require!(
            ctx.accounts.config.owner == ctx.accounts.zebec_eoa.key(),
            MessengerError::InvalidCaller
        );
        msg!("updated");
        let bump = ctx.bumps.get("pda_signer").unwrap().to_le_bytes();

        let signer_seeds: &[&[&[u8]]] = &[&[&sender, &sender_chain, &bump]];

        let approve_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Approve {
                to: ctx.accounts.from.to_account_info(),
                delegate: ctx.accounts.portal_authority_signer.to_account_info(),
                authority: ctx.accounts.pda_signer.to_account_info(),
            },
            signer_seeds,
        );

        // Delgate transfer authority to Token Bridge for the tokens
        approve(approve_ctx, amount)?;

        let target_address: [u8; 32] = receiver.as_slice().try_into().unwrap();
        // Instruction
        let transfer_ix = Instruction {
            program_id: Pubkey::from_str(TOKEN_BRIDGE_ADDRESS).unwrap(),
            accounts: vec![
                AccountMeta::new(ctx.accounts.zebec_eoa.key(), true),
                AccountMeta::new_readonly(ctx.accounts.portal_config.key(), false),
                AccountMeta::new(ctx.accounts.from.key(), false),
                AccountMeta::new_readonly(ctx.accounts.pda_signer.key(), true),
                AccountMeta::new(ctx.accounts.wrapped_mint.key(), false),
                AccountMeta::new_readonly(ctx.accounts.wrapped_meta.key(), false),
                AccountMeta::new_readonly(ctx.accounts.portal_authority_signer.key(), false),
                AccountMeta::new(ctx.accounts.bridge_config.key(), false),
                AccountMeta::new(ctx.accounts.portal_message.key(), true),
                AccountMeta::new_readonly(ctx.accounts.portal_emitter.key(), false),
                AccountMeta::new(ctx.accounts.portal_sequence.key(), false),
                AccountMeta::new(ctx.accounts.bridge_fee_collector.key(), false),
                AccountMeta::new_readonly(ctx.accounts.clock.key(), false),
                // Dependencies
                AccountMeta::new_readonly(ctx.accounts.rent.key(), false),
                AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
                // Program
                AccountMeta::new_readonly(ctx.accounts.core_bridge_program.key(), false),
                AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
            ],
            data: (
                crate::portal::Instruction::TransferWrapped,
                TransferWrappedData {
                    nonce: ctx.accounts.config.nonce,
                    amount,
                    fee,
                    target_address,
                    target_chain,
                },
            )
                .try_to_vec()?,
        };

        // Accounts
        let transfer_accs = vec![
            ctx.accounts.zebec_eoa.to_account_info(),
            ctx.accounts.portal_config.to_account_info(),
            ctx.accounts.from.to_account_info(),
            ctx.accounts.pda_signer.to_account_info(),
            ctx.accounts.wrapped_mint.to_account_info(),
            ctx.accounts.wrapped_meta.to_account_info(),
            ctx.accounts.portal_authority_signer.to_account_info(),
            ctx.accounts.bridge_config.to_account_info(),
            ctx.accounts.portal_message.to_account_info(),
            ctx.accounts.portal_emitter.to_account_info(),
            ctx.accounts.portal_sequence.to_account_info(),
            ctx.accounts.bridge_fee_collector.to_account_info(),
            ctx.accounts.clock.to_account_info(),
            // Dependencies
            ctx.accounts.rent.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            // Program
            ctx.accounts.core_bridge_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ];

        invoke_signed(&transfer_ix, &transfer_accs, signer_seeds)?;

        let sum = ctx.accounts.config.nonce.checked_add(1);
        match sum {
            None => return Err(MessengerError::Overflow.into()),
            Some (val) => ctx.accounts.config.nonce = val,
        }

        Ok(())
    }

    //transfer
    pub fn transfer_native(
        ctx: Context<DirectTransferNative>,
        sender: [u8; 32],
        sender_chain: Vec<u8>,
        target_chain: u16,
        fee: u64,

        receiver: Vec<u8>,
    ) -> Result<()> {
        let amount = ctx.accounts.data_storage.amount;
        //Check EOA
        require!(
            ctx.accounts.config.owner == ctx.accounts.zebec_eoa.key(),
            MessengerError::InvalidCaller
        );

        let bump = ctx.bumps.get("pda_signer").unwrap().to_le_bytes();

        let signer_seeds: &[&[&[u8]]] = &[&[&sender, &sender_chain, &bump]];

        let approve_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Approve {
                to: ctx.accounts.from.to_account_info(),
                delegate: ctx.accounts.portal_authority_signer.to_account_info(),
                authority: ctx.accounts.pda_signer.to_account_info(),
            },
            signer_seeds,
        );

        // Delgate transfer authority to Token Bridge for the tokens
        approve(approve_ctx, amount)?;

        let target_address: [u8; 32] = receiver.as_slice().try_into().unwrap();
        // Instruction
        let transfer_ix = Instruction {
            program_id: Pubkey::from_str(TOKEN_BRIDGE_ADDRESS).unwrap(),
            accounts: vec![
                AccountMeta::new(ctx.accounts.zebec_eoa.key(), true),
                AccountMeta::new_readonly(ctx.accounts.portal_config.key(), false),
                AccountMeta::new(ctx.accounts.from.key(), false),
                AccountMeta::new(ctx.accounts.mint.key(), false),
                AccountMeta::new(ctx.accounts.portal_custody.key(), false),
                AccountMeta::new_readonly(ctx.accounts.portal_authority_signer.key(), false),
                AccountMeta::new_readonly(ctx.accounts.portal_custody_signer.key(), false),
                AccountMeta::new(ctx.accounts.bridge_config.key(), false),
                AccountMeta::new(ctx.accounts.portal_message.key(), true),
                AccountMeta::new_readonly(ctx.accounts.portal_emitter.key(), false),
                AccountMeta::new(ctx.accounts.portal_sequence.key(), false),
                AccountMeta::new(ctx.accounts.bridge_fee_collector.key(), false),
                AccountMeta::new_readonly(ctx.accounts.clock.key(), false),
                // Dependencies
                AccountMeta::new_readonly(ctx.accounts.rent.key(), false),
                AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
                // Program
                AccountMeta::new_readonly(ctx.accounts.core_bridge_program.key(), false),
                AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
            ],
            data: (
                crate::portal::Instruction::TransferNative,
                TransferNativeData {
                    nonce: ctx.accounts.config.nonce,
                    amount,
                    fee,
                    target_address,
                    target_chain,
                },
            )
                .try_to_vec()?,
        };

        // Accounts
        let transfer_accs = vec![
            ctx.accounts.zebec_eoa.to_account_info(),
            ctx.accounts.portal_config.to_account_info(),
            ctx.accounts.from.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.portal_custody.to_account_info(),
            ctx.accounts.portal_authority_signer.to_account_info(),
            ctx.accounts.portal_custody_signer.to_account_info(),
            ctx.accounts.bridge_config.to_account_info(),
            ctx.accounts.portal_message.to_account_info(),
            ctx.accounts.portal_emitter.to_account_info(),
            ctx.accounts.portal_sequence.to_account_info(),
            ctx.accounts.bridge_fee_collector.to_account_info(),
            ctx.accounts.clock.to_account_info(),
            // Dependencies
            ctx.accounts.rent.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            // Program
            ctx.accounts.core_bridge_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ];

        invoke_signed(&transfer_ix, &transfer_accs, signer_seeds)?;

        let sum = ctx.accounts.config.nonce.checked_add(1);
        match sum {
            None => return Err(MessengerError::Overflow.into()),
            Some (val) => ctx.accounts.config.nonce = val,
        }

        Ok(())
    }
}

pub fn vaa_hash(vaa: AccountInfo) -> [u8; 32] {
    let vaa = PostedMessageData::try_from_slice(&vaa.data.borrow())
        .unwrap()
        .0;
    let serialized_vaa = serialize_vaa(&vaa);
    let mut h = sha3::Keccak256::default();
    h.write_all(serialized_vaa.as_slice()).unwrap();
    let vaa_hash: [u8; 32] = h.finalize().into();
    return vaa_hash;
}

fn get_u64(data_bytes: Vec<u8>) -> u64 {
    let data_u8 = <[u8; 8]>::try_from(data_bytes).unwrap();
    return u64::from_be_bytes(data_u8);
}

fn get_u16(data_bytes: Vec<u8>) -> u64 {
    let prefix_bytes = vec![0; 6];
    let joined_bytes = [prefix_bytes, data_bytes].concat();
    let data_u8 = <[u8; 8]>::try_from(joined_bytes).unwrap();
    return u64::from_be_bytes(data_u8);
}

fn get_u256(data_bytes: Vec<u8>) -> U256 {
    let data_u8 = <[u8; 32]>::try_from(data_bytes).unwrap();
    return U256::from_big_endian(&data_u8);
}

fn get_u8(data_bytes: Vec<u8>) -> u64 {
    let prefix_bytes = vec![0; 7];
    let joined_bytes = [prefix_bytes, data_bytes].concat();
    let data_u8 = <[u8; 8]>::try_from(joined_bytes).unwrap();
    return u64::from_be_bytes(data_u8);
}

// Convert a full VAA structure into the serialization of its unique components, this structure is
// what is hashed and verified by Guardians.
pub fn serialize_vaa(vaa: &MessageData) -> Vec<u8> {
    let mut v = Cursor::new(Vec::new());
    v.write_u32::<BigEndian>(vaa.vaa_time).unwrap();
    v.write_u32::<BigEndian>(vaa.nonce).unwrap();
    v.write_u16::<BigEndian>(vaa.emitter_chain.clone() as u16)
        .unwrap();
    v.write_all(&vaa.emitter_address).unwrap();
    v.write_u64::<BigEndian>(vaa.sequence).unwrap();
    v.write_u8(vaa.consistency_level).unwrap();
    v.write_all(&vaa.payload).unwrap();
    v.into_inner()
}

fn process_deposit(encoded_str: Vec<u8>, from_chain_id: u16, ctx: Context<StoreMsg>) {
    let transaction_data = &mut ctx.accounts.data_storage;

    let amount = get_u64(encoded_str[1..9].to_vec());
    let _to_chain_id = get_u256(encoded_str[9..41].to_vec());
    let senderbytes = encoded_str[41..73].to_vec();
    let token_mint_bytes = &encoded_str[73..105].to_vec();

    transaction_data.amount = amount;
    transaction_data.sender = senderbytes;
    transaction_data.from_chain_id = from_chain_id as u64;
    transaction_data.token_mint = Pubkey::new(&token_mint_bytes[..]);
}

fn process_stream(encoded_str: Vec<u8>, from_chain_id: u16, ctx: Context<StoreMsg>) {
    let transaction_data = &mut ctx.accounts.data_storage;
    let start_time = get_u64(encoded_str[1..9].to_vec());
    let end_time = get_u64(encoded_str[9..17].to_vec());
    let amount = get_u64(encoded_str[17..25].to_vec());
    let _to_chain_id = get_u256(encoded_str[25..57].to_vec());
    let senderwallet_bytes = encoded_str[57..89].to_vec();
    let receiver_wallet_bytes = encoded_str[89..121].to_vec();
    let can_update = get_u64(encoded_str[121..129].to_vec());
    let can_cancel = get_u64(encoded_str[129..137].to_vec());
    let token_mint_bytes = &encoded_str[137..169].to_vec();

    transaction_data.start_time = start_time;
    transaction_data.end_time = end_time;

    transaction_data.can_update = can_update == 1;
    transaction_data.can_cancel = can_cancel == 1;

    transaction_data.amount = amount;
    transaction_data.sender = senderwallet_bytes;
    transaction_data.receiver = receiver_wallet_bytes;
    transaction_data.from_chain_id = from_chain_id as u64;
    transaction_data.token_mint = Pubkey::new(&token_mint_bytes[..]);
}

fn process_update_stream(encoded_str: Vec<u8>, from_chain_id: u16, ctx: Context<StoreMsg>) {
    let transaction_data = &mut ctx.accounts.data_storage;
    let start_time = get_u64(encoded_str[1..9].to_vec());
    let end_time = get_u64(encoded_str[9..17].to_vec());
    let amount = get_u64(encoded_str[17..25].to_vec());
    let _to_chain_id = get_u256(encoded_str[25..57].to_vec());
    let senderwallet_bytes = encoded_str[57..89].to_vec();
    let receiver_wallet_bytes = encoded_str[89..121].to_vec();
    let token_mint = &encoded_str[121..153].to_vec();
    let data_account = &encoded_str[153..185].to_vec();

    transaction_data.start_time = start_time;
    transaction_data.end_time = end_time;
    transaction_data.amount = amount;
    transaction_data.sender = senderwallet_bytes;
    transaction_data.receiver = receiver_wallet_bytes;
    transaction_data.from_chain_id = from_chain_id as u64;
    transaction_data.token_mint = Pubkey::new(&token_mint[..]);
    transaction_data.data_account = Pubkey::new(&data_account[..]);
}

fn process_pause(encoded_str: Vec<u8>, from_chain_id: u16, ctx: Context<StoreMsg>) {
    let transaction_data = &mut ctx.accounts.data_storage;
    let _to_chain_id = get_u256(encoded_str[1..33].to_vec());
    let depositor_wallet_bytes = encoded_str[33..65].to_vec();
    let token_mint = encoded_str[65..97].to_vec();
    let receiver_wallet_bytes = encoded_str[97..129].to_vec();
    let data_account = encoded_str[129..161].to_vec();

    transaction_data.sender = depositor_wallet_bytes;
    transaction_data.receiver = receiver_wallet_bytes;
    transaction_data.from_chain_id = from_chain_id as u64;
    transaction_data.token_mint = Pubkey::new(&token_mint[..]);
    transaction_data.data_account = Pubkey::new(&data_account[..]);
}

//receiver will withdraw streamed tokens (receiver == withdrawer)
fn process_withdraw_stream(encoded_str: Vec<u8>, from_chain_id: u16, ctx: Context<StoreMsg>) {
    let transaction_data = &mut ctx.accounts.data_storage;
    let _to_chain_id = get_u256(encoded_str[1..33].to_vec());
    let withdrawer_wallet_bytes = encoded_str[33..65].to_vec();
    let token_mint = encoded_str[65..97].to_vec();
    let depositor_wallet_bytes = encoded_str[97..129].to_vec();
    let data_account = encoded_str[129..161].to_vec();

    transaction_data.sender = depositor_wallet_bytes;
    transaction_data.receiver = withdrawer_wallet_bytes;
    transaction_data.from_chain_id = from_chain_id as u64;
    transaction_data.token_mint = Pubkey::new(&token_mint[..]);
    transaction_data.data_account = Pubkey::new(&data_account[..]);
}

fn process_cancel_stream(encoded_str: Vec<u8>, from_chain_id: u16, ctx: Context<StoreMsg>) {
    let transaction_data = &mut ctx.accounts.data_storage;
    let _to_chain_id = get_u256(encoded_str[1..33].to_vec());
    let depositor_wallet_bytes = encoded_str[33..65].to_vec();
    let token_mint = encoded_str[65..97].to_vec();
    let receiver_wallet_bytes = encoded_str[97..129].to_vec();
    let data_account = encoded_str[129..161].to_vec();

    transaction_data.sender = depositor_wallet_bytes;
    transaction_data.receiver = receiver_wallet_bytes;
    transaction_data.from_chain_id = from_chain_id as u64;
    transaction_data.token_mint = Pubkey::new(&token_mint[..]);
    transaction_data.data_account = Pubkey::new(&data_account[..]);
}

//sender will withdraw deposited token
fn process_withdraw(encoded_str: Vec<u8>, from_chain_id: u16, ctx: Context<StoreMsg>) {
    let transaction_data = &mut ctx.accounts.data_storage;
    let amount = get_u64(encoded_str[1..9].to_vec());
    let _to_chain_id = get_u256(encoded_str[9..41].to_vec());
    let withdrawer_wallet_bytes = encoded_str[41..73].to_vec();
    let token_mint = encoded_str[73..105].to_vec();

    transaction_data.sender = withdrawer_wallet_bytes;
    transaction_data.from_chain_id = from_chain_id as u64;
    transaction_data.token_mint = Pubkey::new(&token_mint[..]);
    transaction_data.amount = amount;
}

fn process_instant_transfer(encoded_str: Vec<u8>, from_chain_id: u16, ctx: Context<StoreMsg>) {
    let transaction_data = &mut ctx.accounts.data_storage;

    let amount = get_u64(encoded_str[1..9].to_vec());
    let _to_chain_id = get_u256(encoded_str[9..41].to_vec());
    let senderwallet_bytes = encoded_str[41..73].to_vec();
    let token_mint = encoded_str[73..105].to_vec();
    let withdrawer_wallet_bytes = encoded_str[105..137].to_vec();

    transaction_data.sender = senderwallet_bytes;
    transaction_data.receiver = withdrawer_wallet_bytes;
    transaction_data.from_chain_id = from_chain_id as u64;
    transaction_data.token_mint = Pubkey::new(&token_mint[..]);
    transaction_data.amount = amount;
}

fn process_direct_transfer(encoded_str: Vec<u8>, from_chain_id: u16, ctx: Context<StoreMsg>) {
    let transaction_data = &mut ctx.accounts.data_storage;

    let amount = get_u64(encoded_str[1..9].to_vec());
    let _to_chain_id = get_u256(encoded_str[9..41].to_vec());
    let senderwallet_bytes = encoded_str[41..73].to_vec();
    let token_mint = encoded_str[73..105].to_vec();
    let withdrawer_wallet_bytes = encoded_str[105..137].to_vec();

    transaction_data.sender = senderwallet_bytes;
    transaction_data.receiver = withdrawer_wallet_bytes;
    transaction_data.from_chain_id = from_chain_id as u64;
    transaction_data.token_mint = Pubkey::new(&token_mint[..]);
    transaction_data.amount = amount;
}
