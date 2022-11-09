// SPDX-License-Identifier: Apache 2
pragma solidity ^0.8.0;

import "./Messages.sol";

contract Encoder is Messages {
    uint8 public constant TOKEN_STREAM = 2;
    uint8 public constant TOKEN_WITHDRAW_STREAM = 4;
    uint8 public constant DEPOSIT_TOKEN = 6;
    uint8 public constant PAUSE_TOKEN = 8;
    uint8 public constant WITHDRAW_TOKEN = 10;
    uint8 public constant INSTANT_TOKEN = 12;
    uint8 public constant TOKEN_STREAM_UPDATE = 14;
    uint8 public constant CANCEL_TOKEN = 16;
    uint8 public constant DIRECT_TRANSFER = 17;
    uint8 public constant INITIALIZE_PDA = 18;
    uint8 public constant INITIALIZE_TOKEN_ACCOUNT = 19;

    function encode_initialize_pda(Messages.InitializePDA memory initializePDA) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            INITIALIZE_PDA,
            initializePDA.account
        );
    }

    function encode_initialize_token_account(Messages.InitializeTokenAccount memory initializeTokenAccount) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            INITIALIZE_TOKEN_ACCOUNT,
            initializeTokenAccount.account,
            initializeTokenAccount.tokenMint
        );
    }

    function encode_token_stream(Messages.ProcessStreamToken memory processStream) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            TOKEN_STREAM,
            processStream.start_time,
            processStream.end_time,
            processStream.amount,
            processStream.toChain,
            processStream.sender,
            processStream.receiver,
            processStream.can_cancel,
            processStream.can_update,
            processStream.token_mint
        );
    }

    function encode_token_stream_update(Messages.UpdateStreamToken memory processStream) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            TOKEN_STREAM_UPDATE,
            processStream.start_time,
            processStream.end_time,
            processStream.amount,
            processStream.toChain,
            processStream.sender,
            processStream.receiver,
            processStream.token_mint,
            processStream.data_account_address
        );
    }

    function encode_token_withdraw_stream(Messages.ProcessWithdrawStreamToken memory processWithdrawStream) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            TOKEN_WITHDRAW_STREAM,
            processWithdrawStream.toChain,
            processWithdrawStream.withdrawer,
            processWithdrawStream.token_mint,
            processWithdrawStream.sender_address,
            processWithdrawStream.data_account_address
        );
    }

    function encode_process_deposit_token(Messages.ProcessDepositToken memory processDeposit) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            DEPOSIT_TOKEN,
            processDeposit.amount,
            processDeposit.toChain,
            processDeposit.depositor,
            processDeposit.token_mint
        );
    }

    function encode_process_pause_token_stream(Messages.PauseStreamToken memory pauseStream) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            PAUSE_TOKEN,
            pauseStream.toChain,
            pauseStream.sender,
            pauseStream.token_mint,
            pauseStream.reciever_address,
            pauseStream.data_account_address
        );
    }

    function encode_process_cancel_token_stream(Messages.CancelStreamToken memory cancelStream) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            CANCEL_TOKEN,
            cancelStream.toChain,
            cancelStream.sender,
            cancelStream.token_mint,
            cancelStream.reciever_address,
            cancelStream.data_account_address
        );
    }

    function encode_process_token_withdrawal(Messages.ProcessWithdrawToken memory processWithdraw) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            WITHDRAW_TOKEN,
            processWithdraw.amount,
            processWithdraw.toChain,
            processWithdraw.withdrawer,
            processWithdraw.token_mint
        );
    }

    function encode_process_instant_token_transfer(Messages.ProcessTransferToken memory processTransfer) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            INSTANT_TOKEN,
            processTransfer.amount,
            processTransfer.toChain,
            processTransfer.sender,
            processTransfer.token_mint,
            processTransfer.receiver
        );
    }

    function encode_process_direct_transfer(Messages.ProcessTransferToken memory processTransfer) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            DIRECT_TRANSFER,
            processTransfer.amount,
            processTransfer.toChain,
            processTransfer.sender,
            processTransfer.token_mint,
            processTransfer.receiver
        );
    }
}