// SPDX-License-Identifier: Apache 2
pragma solidity ^0.8.0;

import "./Messages.sol";

contract Encoder is Messages {
    uint8 public constant SOL_STREAM = 1;
    uint8 public constant TOKEN_STREAM = 2;
    uint8 public constant SOL_WITHDRAW_STREAM = 3;
    uint8 public constant TOKEN_WITHDRAW_STREAM = 4;
    uint8 public constant DEPOSIT_SOL = 5;
    uint8 public constant DEPOSIT_TOKEN = 6;
    uint8 public constant PAUSE_SOL = 7;
    uint8 public constant PAUSE_TOKEN = 8;
    uint8 public constant WITHDRAW_SOL = 9;
    uint8 public constant WITHDRAW_TOKEN = 10;
    uint8 public constant INSTANT_SOL = 11;
    uint8 public constant INSTANT_TOKEN = 12;
    uint8 public constant SOL_STREAM_UPDATE = 13;
    uint8 public constant TOKEN_STREAM_UPDATE = 14;
    uint8 public constant CANCEL_SOL = 15;
    uint8 public constant CANCEL_TOKEN = 16;

    function encode_native_stream(Messages.ProcessStream memory processStream) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            SOL_STREAM,
            processStream.start_time,
            processStream.end_time,
            processStream.amount,
            processStream.toChain,
            processStream.sender,
            processStream.receiver,
            processStream.can_cancel,
            processStream.can_update
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

  function encode_native_stream_update(Messages.UpdateStream memory processStream) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            SOL_STREAM_UPDATE,
            processStream.start_time,
            processStream.end_time,
            processStream.amount,
            processStream.toChain,
            processStream.sender,
            processStream.receiver
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
            processStream.token_mint
        );
    }

    function encode_native_withdraw_stream(Messages.ProcessWithdrawStream memory processWithdrawStream) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            SOL_WITHDRAW_STREAM,
            processWithdrawStream.toChain,
            processWithdrawStream.withdrawer
        );
    }

    function encode_token_withdraw_stream(Messages.ProcessWithdrawStreamToken memory processWithdrawStream) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            TOKEN_WITHDRAW_STREAM,
            processWithdrawStream.toChain,
            processWithdrawStream.withdrawer,
            processWithdrawStream.token_mint
        );
    }

    function encode_process_deposit_sol(Messages.ProcessDeposit memory processDeposit) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            DEPOSIT_SOL,
            processDeposit.amount,
            processDeposit.toChain,
            processDeposit.depositor
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

    function encode_process_pause_native_stream(Messages.PauseStream memory pauseStream) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            PAUSE_SOL,
            pauseStream.toChain,
            pauseStream.sender
        );
    }

    function encode_process_pause_token_stream(Messages.PauseStreamToken memory pauseStream) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            PAUSE_TOKEN,
            pauseStream.toChain,
            pauseStream.sender,
            pauseStream.token_mint
        );
    }

    function encode_process_cancel_native_stream(Messages.CancelStream memory cancelStream) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            CANCEL_SOL,
            cancelStream.toChain,
            cancelStream.sender
        );
    }

    function encode_process_cancel_token_stream(Messages.CancelStreamToken memory cancelStream) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            CANCEL_TOKEN,
            cancelStream.toChain,
            cancelStream.sender,
            cancelStream.token_mint
        );
    }

    function encode_process_native_withdrawal(Messages.ProcessWithdraw memory processWithdraw) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            WITHDRAW_SOL,
            processWithdraw.amount,
            processWithdraw.toChain,
            processWithdraw.withdrawer
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

    function encode_process_instant_native_transfer(Messages.ProcessTransfer memory processTransfer) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            INSTANT_SOL,
            processTransfer.amount,
            processTransfer.toChain,
            processTransfer.sender
        );
    }

    function encode_process_instant_token_transfer(Messages.ProcessTransferToken memory processTransfer) public pure returns (bytes memory encoded){
        encoded = abi.encodePacked(
            INSTANT_TOKEN,
            processTransfer.amount,
            processTransfer.toChain,
            processTransfer.sender,
            processTransfer.token_mint
        );
    }
}