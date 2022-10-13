use anchor_lang::prelude::*;

#[error_code]
pub enum MessengerError {
    #[msg("Posted VAA Key Mismatch")]
    VAAKeyMismatch,

    #[msg("Posted VAA Emitter Chain ID or Address Mismatch")]
    VAAEmitterMismatch,

    #[msg("The given transaction has already been executed.")]
    AlreadyExecuted,

    #[msg("The given PDA Signer is invalid for the current transation. ")]
    InvalidPDASigner,
    
    #[msg("Invalid leng.")]
    InvalidOwnersLen,

    #[msg("Not unique.")]
    UniqueOwners, 

    #[msg("Data differs from the Wormhole and Client Side.")]
    InvalidDataProvided,
    
    #[msg("Receipt already claimed!")]
    ReceiptClaimed,

    #[msg("Invalid Caller")]
    InvalidCaller
}