use anchor_lang::prelude::*;

#[error_code]
pub enum MessengerError {
    #[msg("Posted VAA Key Mismatch")]
    VAAKeyMismatch,

    #[msg("Posted VAA Emitter Chain ID or Address Mismatch")]
    VAAEmitterMismatch,

    #[msg("Sender Wallet Mismatch")]
    InvalidSenderWallet,
    
    #[msg("The given transaction has already been executed.")]
    AlreadyExecuted,

    #[msg("Invalid CPI")]
    InvalidCPI,

    #[msg("Invalid Caller")]
    InvalidCaller,

    #[msg("Overflow")]
    Overflow,

    #[msg("Invalid Payload")]
    InvalidPayload,

    #[msg("Invalid Emitter Address Provided")]
    InvalidEmitterAddress,

    #[msg("Invalid Count")]
    CountMismatch,

    #[msg("Invalid Mint Key")]
    MintKeyMismatch,

    #[msg("Invalid Pda Sender")]
    PdaSenderMismatch,

    #[msg("Invalid Pda Receiver")]
    PdaReceiverMismatch,

    #[msg("Invalid Sender Derived Public Key")]
    SenderDerivedKeyMismatch,

    #[msg("Invalid Receiver Derived Public Key")]
    ReceiverDerivedKeyMismatch,

    #[msg("Invalid Amount")]
    AmountMismatch,

    #[msg("Invalid Start Time")]
    StartTimeMismatch,

    #[msg("Invalid End Time")]
    EndTimeMismatch,

    #[msg("Invalid Can Cancel")]
    CanCancelMismatch,

    #[msg("Invalid Can Update")]
    CanUpdateMismatch,

    #[msg("Invalid Data Account")]
    DataAccountMismatch,

    #[msg("Transaction Already Created")]
    TransactionAlreadyCreated,

    #[msg("Transaction Already Executed")]
    TransactionAlreadyExecuted
}