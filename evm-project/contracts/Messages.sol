// SPDX-License-Identifier: Apache 2

pragma solidity ^0.8.0;

/**
 * @title Messages
 */
contract Messages {
    struct UpdateStream {
        uint64 start_time;
        uint64 end_time;
        uint64 amount;
        uint16 toChain;
        bytes sender;
        bytes receiver;
    }

    struct UpdateStreamToken {
        uint64 start_time;
        uint64 end_time;
        uint64 amount;
        uint16 toChain;
        bytes sender;
        bytes receiver;
        bytes token_mint;
        bytes data_account_address;
    }

    struct ProcessStream {
        uint64 start_time;
        uint64 end_time;
        uint64 amount;
        uint16 toChain;
        bytes sender;
        bytes receiver;
        uint64 can_cancel;
        uint64 can_update;
    }

    struct ProcessStreamToken {
        uint64 start_time;
        uint64 end_time;
        uint64 amount;
        uint16 toChain;
        bytes sender;
        bytes receiver;
        uint64 can_cancel;
        uint64 can_update;
        bytes token_mint;
    }

    struct ProcessWithdrawStream {
        uint16 toChain;
        bytes withdrawer;
    }

    struct ProcessWithdrawStreamToken {
        uint16 toChain;
        bytes withdrawer;
        bytes token_mint;
        bytes sender_address;
        bytes data_account_address;
    }
    
    struct PauseStream {
        uint16 toChain;
        bytes sender;
    }

    struct PauseStreamToken {
        uint16 toChain;
        bytes sender;
        bytes token_mint;
        bytes reciever_address;
        bytes data_account_address;
    }

    struct CancelStream {
        uint16 toChain;
        bytes sender;
    }

    struct CancelStreamToken {
        uint16 toChain;
        bytes sender;
        bytes token_mint;
        bytes reciever_address;
        bytes data_account_address;
    }

    struct ProcessDeposit {
        uint64 amount;
        uint16 toChain;
        bytes depositor;
    }

    struct ProcessDepositToken {
        uint64 amount;
        uint16 toChain;
        bytes depositor;
        bytes token_mint;
    }

    struct ProcessWithdraw {
        uint64 amount;
        uint16 toChain;
        bytes withdrawer;
    }

    struct ProcessTransfer {
        uint64 amount;
        uint16 toChain;
        bytes withdrawer;
        bytes sender;
    }

    struct ProcessTransferToken {
        uint64 amount;
        uint16 toChain;
        bytes sender;
        bytes token_mint;
        bytes receiver;
    }

    struct ProcessWithdrawToken {
        uint64 amount;
        uint16 toChain;
        bytes withdrawer;
        bytes token_mint;
    }

    struct InstantWithdrawal {
        uint64 amount;
        uint16 toChain;
        bytes withdrawer;
    }
}