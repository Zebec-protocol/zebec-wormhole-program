// SPDX-License-Identifier: Apache 2

pragma solidity ^0.8.0;

/**
 * @title Messages
 */
contract Messages {
    
    struct InitializePDA{
        bytes account;
        uint256 toChain;
    }

    struct InitializeTokenAccount{
        bytes account;
        bytes tokenMint;
        uint256 toChain;
    }

    struct UpdateStreamToken {
        uint64 start_time;
        uint64 end_time;
        uint64 amount;
        uint256 toChain;
        bytes sender;
        bytes receiver;
        bytes token_mint;
        bytes data_account_address;
    }

    struct ProcessStreamToken {
        uint64 start_time;
        uint64 end_time;
        uint64 amount;
        uint256 toChain;
        bytes sender;
        bytes receiver;
        uint64 can_cancel;
        uint64 can_update;
        bytes token_mint;
    }

    struct ProcessWithdrawStreamToken {
        uint256 toChain;
        bytes withdrawer;
        bytes token_mint;
        bytes sender_address;
        bytes data_account_address;
    }

    struct PauseStreamToken {
        uint256 toChain;
        bytes sender;
        bytes token_mint;
        bytes reciever_address;
        bytes data_account_address;
    }

    struct CancelStreamToken {
        uint256 toChain;
        bytes sender;
        bytes token_mint;
        bytes reciever_address;
        bytes data_account_address;
    }

    struct ProcessDepositToken {
        uint64 amount;
        uint256 toChain;
        bytes depositor;
        bytes token_mint;
    }

    struct ProcessTransferToken {
        uint64 amount;
        uint256 toChain;
        bytes sender;
        bytes token_mint;
        bytes receiver;
    }

    struct ProcessWithdrawToken {
        uint64 amount;
        uint256 toChain;
        bytes withdrawer;
        bytes token_mint;
    }

}