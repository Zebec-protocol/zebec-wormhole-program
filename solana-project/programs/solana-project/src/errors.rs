use anchor_lang::prelude::*;

#[error_code]
pub enum MessengerError {
    #[msg("Posted VAA Key Mismatch")]
    VAAKeyMismatch,

    #[msg("Posted VAA Emitter Chain ID or Address Mismatch")]
    VAAEmitterMismatch,

    #[msg("The given owner is not part of this multisig.")]
    InvalidOwner,

    #[msg("The given transaction has already been executed.")]
    AlreadyExecuted,

    #[msg("Not enough owners signed this transaction.")]
    NotEnoughSigners,

    #[msg("The given PDA Signer is invalid for the current transation. ")]
    InvalidPDASigner,
    
    #[msg("Threshold must be less than or equal to the number of owners.")]
    InvalidThreshold,

    #[msg("Invalid leng.")]
    InvalidOwnersLen,

    #[msg("Not unique.")]
    UniqueOwners, 

    #[msg("Data differs from the Wormhole and Client Side.")]
    InvalidDataProvided
}