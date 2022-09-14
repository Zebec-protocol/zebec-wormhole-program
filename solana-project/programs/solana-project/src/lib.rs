use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::system_instruction::transfer;
use anchor_lang::solana_program::borsh::try_from_slice_unchecked;
use anchor_lang::solana_program::keccak::hashv;
use anchor_lang::solana_program::keccak::Hash;
use anchor_lang::solana_program;

use sha3::Digest;

use byteorder::{
    BigEndian,
    WriteBytesExt,
};
use std::io::{
    Cursor,
    Write,
};
use std::str::FromStr;
use hex::decode;
mod context;
mod constants;
mod state;
mod wormhole;
mod errors;

use wormhole::*;
use context::*;
use constants::*;
use errors::*;
use state::*;

use std::ops::Deref;

declare_id!("3DwkDP1FrPSgvM2hXJ6PSwP3rm4nq54HX16gU41ckWGd");

#[program]
pub mod solana_project {

    use anchor_lang::solana_program::program::invoke_signed;

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.config.owner = ctx.accounts.owner.key();
        ctx.accounts.config.nonce = 1;
        Ok(())
    }

    pub fn register_chain(ctx:Context<RegisterChain>, chain_id:u16, emitter_addr:String) -> Result<()> {
        ctx.accounts.emitter_acc.chain_id = chain_id;
        ctx.accounts.emitter_acc.emitter_addr = emitter_addr;
        Ok(())
    }

    pub fn send_msg(ctx:Context<SendMsg>, msg:String) -> Result<()> {
        //Look Up Fee
        let bridge_data:BridgeData = try_from_slice_unchecked(&ctx.accounts.wormhole_config.data.borrow_mut())?;
        
        //Send Fee
        invoke_signed(
            &transfer(
                &ctx.accounts.payer.key(),
                &ctx.accounts.wormhole_fee_collector.key(),
                bridge_data.config.fee
            ),
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.wormhole_fee_collector.to_account_info()
            ],
            &[]
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
            ).try_to_vec()?,
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
            &[
                &[
                    &b"emitter".as_ref(),
                    &[*ctx.bumps.get("wormhole_derived_emitter").unwrap()]
                ]
            ]
        )?;

        ctx.accounts.config.nonce += 1;
        Ok(())
    }

    pub fn store_msg(
        ctx: Context<StoreMsg>, 
        current_count: u8, 
        sender: Vec<u8>,
    ) -> Result<()> {

        //Hash a VAA Extract and derive a VAA Key
        let vaa = PostedMessageData::try_from_slice(&ctx.accounts.core_bridge_vaa.data.borrow())?.0;
        let serialized_vaa = serialize_vaa(&vaa);

        let mut h = sha3::Keccak256::default();
        h.write(serialized_vaa.as_slice()).unwrap();
        let vaa_hash: [u8; 32] = h.finalize().into();

        let (vaa_key, _) = Pubkey::find_program_address(&[
            b"PostedVAA",
            &vaa_hash
        ], &Pubkey::from_str(CORE_BRIDGE_ADDRESS).unwrap());

        if ctx.accounts.core_bridge_vaa.key() != vaa_key {
            return err!(MessengerError::VAAKeyMismatch);
        }

        // Already checked SignedVaa is owned by core bridge in account constraint logic
        // Check that the emitter chain and address match up with the vaa
        if vaa.emitter_chain != ctx.accounts.emitter_acc.chain_id ||
           vaa.emitter_address != &decode(&ctx.accounts.emitter_acc.emitter_addr.as_str()).unwrap()[..] {
            return err!(MessengerError::VAAEmitterMismatch)
        }

        // Encoded String
        let encoded_str = vaa.payload.clone();
        
        // Decode Encoded String and Store Value based upon the code sent on message passing
        let code = get_u8(encoded_str[0..1].to_vec()); 

        // Change Transaction Count to Current Count
        let txn_count = &mut ctx.accounts.txn_count;
        txn_count.count += 1;

        // Switch Based on the code
        match code {
            2 => process_stream(encoded_str, code, ctx),
            4 => process_withdraw_stream(encoded_str, code, ctx),
            6 => process_deposit(encoded_str, code, ctx),
            8 => process_pause(encoded_str, code, ctx),
            10 => process_withdraw(encoded_str, code, ctx), 
            12 => process_instant_transfer(encoded_str, code, ctx),
            14 => process_update_stream(encoded_str, code, ctx),
            16 => process_cancel_stream(encoded_str, code, ctx),
            _ =>
                msg!("error"),
        }

       

        Ok(())
    }

    pub fn create_transaction(
        ctx: Context<CreateTransaction>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
        current_count: u8,

        //Detail Data
        sender: Vec<u8>,
        transaction_hash: Vec<u8>
    ) -> Result<()> {

        // validate data
        let txn_hash = ctx.accounts.data_storage.transaction_hash.to_vec();

        // require!(txn_hash == transaction_hash, MessengerError::InvalidDataProvided);
        
        //Build Transactions
        let tx = &mut ctx.accounts.transaction;
        tx.program_id = pid;
        tx.accounts = accs;
        tx.data = data;
        tx.did_execute = false;

        Ok(())
    }

    pub fn execute_transaction(
        ctx: Context<ExecuteTransaction>,
        from_chain_id: Vec<u8>,
        eth_add_hash: Vec<u8>
    ) -> Result<()> {
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
        let seeds : &[&[_]] = &[
            &eth_add_hash,
            &from_chain_id, 
            bump.as_ref()
        ];
        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;

        msg!("Transaction Execute");
        
        solana_program::program::invoke_signed(&ix, accounts, signer)?;

        Ok(())
    }

}

fn get_u64(data_bytes: Vec<u8>) -> u64 {
    let data_u8 = <[u8; 8]>::try_from(data_bytes).unwrap();
    return u64::from_be_bytes(data_u8);
}

fn get_u16(data_bytes: Vec<u8>) -> u64{
    let prefix_bytes = vec![0; 6];
    let joined_bytes = [prefix_bytes, data_bytes].concat();
    let data_u8 = <[u8; 8]>::try_from(joined_bytes).unwrap();
    return u64::from_be_bytes(data_u8);
}

fn get_u8(data_bytes: Vec<u8>) -> u64 {
    let prefix_bytes = vec![0; 7];
    let joined_bytes = [prefix_bytes, data_bytes].concat();
    let data_u8 = <[u8; 8]>::try_from(joined_bytes).unwrap();
    return u64::from_be_bytes(data_u8);
}

fn get_hash(
    code: u64,
    start_time: u64,
    end_time: u64,
    amount: u64,
    from_chain_id: u64,
    sender: Vec<u8>,
    receiver: Vec<u8>,
    can_cancel: u64,
    can_update: u64,
    token_mint: Vec<u8>,
) -> Hash{

    let combined_data = [
        code.to_be_bytes(),
        start_time.to_be_bytes(), 
        end_time.to_be_bytes(),
        amount.to_be_bytes(),
        from_chain_id.to_be_bytes(),
        // sender, 
        // receiver, 
        can_cancel.to_be_bytes(),
        can_update.to_be_bytes(),
        // token_mint,
    ].concat();

    hashv(&[&combined_data])
}

// Convert a full VAA structure into the serialization of its unique components, this structure is
// what is hashed and verified by Guardians.
pub fn serialize_vaa(vaa: &MessageData) -> Vec<u8> {
    let mut v = Cursor::new(Vec::new());
    v.write_u32::<BigEndian>(vaa.vaa_time).unwrap();
    v.write_u32::<BigEndian>(vaa.nonce).unwrap();
    v.write_u16::<BigEndian>(vaa.emitter_chain.clone() as u16).unwrap();
    v.write(&vaa.emitter_address).unwrap();
    v.write_u64::<BigEndian>(vaa.sequence).unwrap();
    v.write_u8(vaa.consistency_level).unwrap();
    v.write(&vaa.payload).unwrap();
    v.into_inner()
}

fn process_stream(mut encoded_str: Vec<u8>, code: u64, ctx: Context<StoreMsg>){  
    let transaction_data = &mut ctx.accounts.data_storage;

    let start_time = get_u64(encoded_str[1..9].to_vec());
    let end_time = get_u64(encoded_str[9..17].to_vec());
    let amount = get_u64(encoded_str[17..25].to_vec());
    let from_chain_id = get_u16(encoded_str[25..27].to_vec());

    let sender_wallet_bytes = &encoded_str[27..59].to_vec();
    
    let receiver_wallet_bytes = &encoded_str[59..91].to_vec();

    let can_update = get_u64(encoded_str[91..99].to_vec());
    let can_cancel = get_u64(encoded_str[99..107].to_vec());

    let token_mint_bytes =&encoded_str[107..132].to_vec();
    msg!("Token Stream");
    
     
    let transaction_hash = get_hash(
        code,
        start_time, 
        end_time,
        amount, 
        from_chain_id, 
        sender_wallet_bytes.to_vec(), 
        receiver_wallet_bytes.to_vec(), 
        can_update, 
        can_cancel,
        token_mint_bytes.to_vec()
    );

    
    transaction_data.transaction_hash = transaction_hash.to_bytes();

}

fn process_withdraw_stream(mut encoded_str: Vec<u8>, code: u64, ctx: Context<StoreMsg>) {  
    let transaction_data = &mut ctx.accounts.data_storage;

    let from_chain_id = get_u16(encoded_str[1..3].to_vec());

    let withdrawer_wallet_bytes = encoded_str[3..35].to_vec();

    let token_mint_bytes = encoded_str[35..67].to_vec();
    msg!("Token Withdraw Stream");
    
     
    let transaction_hash = get_hash(
        code,
        0u64, 
        0u64,
        0u64, 
        from_chain_id, 
        [].to_vec(), 
        withdrawer_wallet_bytes, 
        0u64, 
        0u64,
        token_mint_bytes
    );

    
    transaction_data.transaction_hash = transaction_hash.to_bytes();

}

fn process_deposit(mut encoded_str: Vec<u8>, code: u64, ctx: Context<StoreMsg>)  {
    let transaction_data = &mut ctx.accounts.data_storage;

    let amount = get_u64(encoded_str[1..9].to_vec());
    msg!("amount {:?}", amount);

    let from_chain_id = get_u16(encoded_str[9..11].to_vec());
    msg!("from_chain_id {:?}",from_chain_id);

    let sender_bytes = &encoded_str[11..43].to_vec();
    msg!("sender_bytes {:?}",sender_bytes);

    let token_mint_bytes = &encoded_str[43..75].to_vec();
    msg!("token_mint_bytes {:?}",token_mint_bytes);

    
    let txn_hash = get_hash(
        code, 
        0u64, 
        0u64, 
        amount, 
        from_chain_id, 
        sender_bytes.to_vec(), 
        [].to_vec(), 
        0u64, 
        0u64, 
        token_mint_bytes.to_vec()
    );
    msg!("Token Deposit");

    transaction_data.transaction_hash = txn_hash.to_bytes();
}

fn process_pause(mut encoded_str: Vec<u8>, code: u64, ctx: Context<StoreMsg>) {
    let transaction_data = &mut ctx.accounts.data_storage;

    let to_chain_id = get_u16(encoded_str[1..3].to_vec());
    let depositor_wallet_bytes = encoded_str[3..35].to_vec();
    let token_mint = encoded_str[35..67].to_vec();
    msg!("Process Pause");

    let transaction_hash = get_hash(
        code, 
        0u64, 
        0u64, 
        0u64, 
        to_chain_id, 
        depositor_wallet_bytes,
        [].to_vec(),
        0u64, 
        0u64,
        token_mint
    );

    transaction_data.transaction_hash = transaction_hash.to_bytes();

}

fn process_withdraw(mut encoded_str: Vec<u8>, code: u64, ctx: Context<StoreMsg>) {  
    let transaction_data = &mut ctx.accounts.data_storage;

    let amount = get_u64(encoded_str[1..9].to_vec());
    let to_chain_id = get_u16(encoded_str[9..11].to_vec());
    let withdrawer_wallet_bytes = encoded_str[11..43].to_vec();
    let token_stream =  encoded_str[43..75].to_vec();

    msg!("Process Withdraw");

    let transaction_hash = get_hash(
        code, 
        0u64, 
        0u64, 
        amount, 
        to_chain_id, 
        [].to_vec(), 
        withdrawer_wallet_bytes, 
        0u64, 0u64, 
        token_stream);
    
    transaction_data.transaction_hash = transaction_hash.to_bytes();

}

fn process_instant_transfer(mut encoded_str: Vec<u8>, code: u64, ctx: Context<StoreMsg>) {  
    let transaction_data = &mut ctx.accounts.data_storage;

    let amount = get_u64(encoded_str[1..9].to_vec());
    
    let to_chain_id = get_u16(encoded_str[9..11].to_vec());

    let sender_wallet_bytes = encoded_str[11..43].to_vec();

    let withdrawer_wallet_bytes = encoded_str[43..75].to_vec();


    let token_mint = encoded_str[75..107].to_vec();
    msg!("Token Instant Transfer");
    
    let transaction_hash = get_hash(
        code,
        0u64,
         0u64, 
         amount, 
         to_chain_id, 
         sender_wallet_bytes, 
         withdrawer_wallet_bytes,
         0u64, 
         0u64, 
         token_mint
    );

    transaction_data.transaction_hash = transaction_hash.to_bytes();


}

fn process_update_stream(mut encoded_str: Vec<u8>, code: u64, ctx: Context<StoreMsg>) {  
    
    let transaction_data = &mut ctx.accounts.data_storage;

    let start_time = get_u64(encoded_str[1..9].to_vec());

    let end_time = get_u64(encoded_str[9..17].to_vec());

    let amount = get_u64(encoded_str[17..25].to_vec());
    
    let from_chain_id = get_u16(encoded_str[25..27].to_vec());

    let sender_wallet_bytes = &encoded_str[27..59].to_vec();
   
    let receiver_wallet_bytes = encoded_str[59..91].to_vec();

    let token_mint = &encoded_str[91..123].to_vec();
    msg!("Token Stream Update");

     
    let transaction_hash = get_hash(
        code,
        start_time, 
        end_time,
        amount, 
        from_chain_id, 
        sender_wallet_bytes.to_vec(), 
        receiver_wallet_bytes.to_vec(), 
        0u64, 
        0u64,
        token_mint.to_vec()
    );

    transaction_data.transaction_hash = transaction_hash.to_bytes();

}

fn process_cancel_stream(mut encoded_str: Vec<u8>, code: u64, ctx: Context<StoreMsg>) {  
    let transaction_data = &mut ctx.accounts.data_storage;

    let to_chain_id = get_u16(encoded_str[1..3].to_vec());

    let sender_wallet_bytes = encoded_str[3..35].to_vec();
 
    let token_mint = encoded_str[35..67].to_vec();
    msg!("Token Stream Cancel");

     
    let transaction_hash = get_hash(
        code,
        0u64, 
        0u64,
        0u64, 
        to_chain_id, 
        sender_wallet_bytes.to_vec(), 
        [].to_vec(), 
        0u64, 
        0u64,
        token_mint.to_vec()
    );


    transaction_data.transaction_hash = transaction_hash.to_bytes();

}
