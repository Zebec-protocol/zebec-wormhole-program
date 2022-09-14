// SPDX-License-Identifier: Apache 2
pragma solidity ^0.8.0;

import "./interfaces/IWormhole.sol";
import "./Encoder.sol";

contract Messenger is Encoder {
    bytes16 private constant _HEX_SYMBOLS = "0123456789abcdef";
    uint8 private constant _ADDRESS_LENGTH = 20;

    uint16 public constant CHAIN_ID = 4; //3
    uint8 public constant CONSISTENCY_LEVEL = 1; //15
    uint32 nonce = 0;
    IWormhole _wormhole;
    mapping(bytes32 => bool) _completedMessages;
    bytes private current_msg;
    mapping(uint16 => bytes32) _applicationContracts;

    event DepositSol(bytes depositor, uint64 amount, uint32 nonce);
    event DepositToken(bytes depositor, bytes tokenMint, uint64 amount, uint32 nonce);
    
    event NativeStream(bytes sender, bytes receiver, uint64 amount, uint32 nonce);
    event TokenStream(bytes sender, bytes receiver, bytes tokenMint, uint64 amount, uint32 nonce);
   
    event NativeStreamUpdate(bytes sender, bytes receiver, uint64 amount, uint32 nonce);
    event TokenStreamUpdate(bytes sender, bytes receiver, bytes tokenMint, uint64 amount, uint32 nonce);
   
    event WithdrawStream(bytes withdrawer, uint32 nonce);
    event WithdrawToken(bytes withdrawer, bytes tokenMint, uint32 nonce);
    
    event PauseNativeStream(bytes receiver, uint32 nonce);
    event PauseTokenStream(bytes receiver, bytes tokenMint, uint32 nonce);
    
    event CancelNativeStream(bytes receiver, uint32 nonce);
    event CancelTokenStream(bytes receiver, bytes tokenMint, uint32 nonce);
    
    event InstantNativeTransfer(bytes receiver, uint64 amount, uint32 nonce);
    event InstantTokenTransfer(bytes receiver, bytes tokenMint, uint64 amount, uint32 nonce);

    event NativeWithdrawal(bytes withdrawer, uint64 amount, uint32 nonce);
    event TokenWithdrawal(bytes withdrawer, bytes tokenMint, uint64 amount, uint32 nonce);
    
    constructor() {
        // constructor(address wormholeAddress) {
        _wormhole = IWormhole(0x706abc4E45D419950511e474C7B9Ed348A4a716c);
    }

    function wormhole() public view returns (IWormhole) {
        return _wormhole;
    }

    function sendMsg(bytes memory str) public returns (uint64 sequence) {
        sequence = wormhole().publishMessage(nonce, str, 1);
        nonce = nonce + 1;
    }

    function process_deposit_sol(
        uint64 amount, 
        bytes memory depositor
    ) public payable returns (uint64 sequence){
        bytes memory sol_stream = Encoder.encode_process_deposit_sol(
            Messages.ProcessDeposit({
                amount: amount,
                toChain: CHAIN_ID,
                depositor: depositor
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            sol_stream,
            CONSISTENCY_LEVEL
        );
        emit DepositSol(depositor, amount, nonce);
        nonce = nonce + 1;
    }

    function process_deposit_token(
        uint64 amount, 
        bytes memory depositor,
        bytes memory token_mint
    ) public payable returns (uint64 sequence){
        bytes memory token_stream = Encoder.encode_process_deposit_token(
            Messages.ProcessDepositToken({
                amount: amount,
                toChain: CHAIN_ID,
                depositor: depositor,
                token_mint: token_mint
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            token_stream,
            CONSISTENCY_LEVEL
        );
        emit DepositToken(depositor, token_mint, amount, nonce);
        nonce = nonce + 1;
    }

    function process_native_stream(
        uint64 start_time,
        uint64 end_time,
        uint64 amount,
        bytes memory receiver,
        bytes memory sender,
        uint64 can_cancel,
        uint64 can_update
    ) public payable returns (uint64 sequence) {
        bytes memory sol_stream = Encoder.encode_native_stream(
            Messages.ProcessStream({
                start_time: start_time,
                end_time: end_time,
                amount: amount,
                toChain: CHAIN_ID,
                sender: sender,
                receiver: receiver,
                can_cancel: can_cancel,
                can_update: can_update
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            sol_stream,
            CONSISTENCY_LEVEL
        );
        emit NativeStream(sender, receiver, amount, nonce);
        nonce = nonce + 1;
    }

    function process_token_stream(
        uint64 start_time,
        uint64 end_time,
        uint64 amount,
        bytes memory receiver,
        bytes memory sender,
        uint64 can_cancel,
        uint64 can_update,
        bytes memory token_mint
    ) public payable returns (uint64 sequence) {
        bytes memory token_stream = Encoder.encode_token_stream(
            Messages.ProcessStreamToken({
                start_time: start_time,
                end_time: end_time,
                amount: amount,
                toChain: CHAIN_ID,
                sender: sender,
                receiver: receiver,
                can_cancel: can_cancel,
                can_update: can_update,
                token_mint: token_mint
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            token_stream,
            CONSISTENCY_LEVEL
        );
        emit TokenStream(sender, receiver, token_mint, amount, nonce);
        nonce = nonce + 1;
    }

    function process_native_stream_update(
        uint64 start_time,
        uint64 end_time,
        uint64 amount,
        bytes memory receiver,
        bytes memory sender
    ) public payable returns (uint64 sequence) {
        bytes memory sol_stream = Encoder.encode_native_stream_update(
            Messages.UpdateStream({
                start_time: start_time,
                end_time: end_time,
                amount: amount,
                toChain: CHAIN_ID,
                sender: sender,
                receiver: receiver
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            sol_stream,
            CONSISTENCY_LEVEL
        );
        emit NativeStreamUpdate(sender, receiver, amount, nonce);
        nonce = nonce + 1;
    }

    function process_token_stream_update(
        uint64 start_time,
        uint64 end_time,
        uint64 amount,
        bytes memory receiver,
        bytes memory sender,
        bytes memory token_mint
    ) public payable returns (uint64 sequence) {
        bytes memory sol_stream = Encoder.encode_token_stream_update(
            Messages.UpdateStreamToken({
                start_time: start_time,
                end_time: end_time,
                amount: amount,
                toChain: CHAIN_ID,
                sender: sender,
                receiver: receiver,
                token_mint: token_mint
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            sol_stream,
            CONSISTENCY_LEVEL
        );
        emit TokenStreamUpdate(sender, receiver, token_mint, amount, nonce);
        nonce = nonce + 1;
    }

    // receiver will withdraw using this
    function process_native_withdraw_stream(
        bytes memory withdrawer
    ) public payable returns (uint64 sequence){
        bytes memory sol_stream = Encoder.encode_native_withdraw_stream(
            Messages.ProcessWithdrawStream({
                toChain: CHAIN_ID,
                withdrawer: withdrawer
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            sol_stream,
            CONSISTENCY_LEVEL
        );
        emit WithdrawStream(withdrawer, nonce);
        nonce = nonce + 1;
    }

    function process_token_withdraw_stream(
        bytes memory withdrawer,
        bytes memory token_mint
    ) public payable returns (uint64 sequence) {
        bytes memory token_stream = Encoder.encode_token_withdraw_stream(
            Messages.ProcessWithdrawStreamToken({
                toChain: CHAIN_ID,
                withdrawer: withdrawer,
                token_mint: token_mint
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            token_stream,
            CONSISTENCY_LEVEL
        );
        emit WithdrawToken(withdrawer, token_mint, nonce);
        nonce = nonce + 1;
    }

    function process_pause_native_stream(
        bytes memory sender
    ) public payable returns (uint64 sequence) {
        bytes memory sol_stream = Encoder.encode_process_pause_native_stream(
            Messages.PauseStream({
                toChain: CHAIN_ID,
                sender: sender
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            sol_stream,
            CONSISTENCY_LEVEL
        );
        emit PauseNativeStream(sender, nonce);
        nonce = nonce + 1;
    }

   function process_pause_token_stream(
        bytes memory sender,
        bytes memory token_mint
    ) public payable returns (uint64 sequence) {
        bytes memory sol_stream = Encoder.encode_process_pause_token_stream(
            Messages.PauseStreamToken({
                toChain: CHAIN_ID,
                sender: sender,
                token_mint: token_mint
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            sol_stream,
            CONSISTENCY_LEVEL
        );
        emit PauseTokenStream(sender, token_mint, nonce);
        nonce = nonce + 1;
    }

    function process_cancel_native_stream(
        bytes memory sender
    ) public payable returns (uint64 sequence) {
        bytes memory sol_stream = Encoder.encode_process_cancel_native_stream(
            Messages.CancelStream({
                toChain: CHAIN_ID,
                sender: sender
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            sol_stream,
            CONSISTENCY_LEVEL
        );
        emit CancelNativeStream(sender, nonce);
        nonce = nonce + 1;
    }

    function process_cancel_token_stream(
        bytes memory sender,
        bytes memory token_mint
    ) public payable returns (uint64 sequence) {
        bytes memory sol_stream = Encoder.encode_process_cancel_token_stream(
            Messages.CancelStreamToken({
                toChain: CHAIN_ID,
                sender: sender,
                token_mint: token_mint
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            sol_stream,
            CONSISTENCY_LEVEL
        );
        emit CancelTokenStream(sender, token_mint, nonce);
        nonce = nonce + 1;
    }

    // sender will transfer to receiver
    function process_instant_native_transfer(
        uint64 amount, 
        bytes memory sender,
        bytes memory withdrawer
    ) public payable returns (uint64 sequence){
        bytes memory sol_stream = Encoder.encode_process_instant_native_transfer(
            Messages.ProcessTransfer({
                amount: amount,
                toChain: CHAIN_ID,
                withdrawer: withdrawer,
                sender: sender
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            sol_stream,
            CONSISTENCY_LEVEL
        );
        emit InstantNativeTransfer(sender, amount, nonce);
        nonce = nonce + 1;
    }

    // sender will transfer to receiver
    function process_instant_token_transfer(
        uint64 amount, 
        bytes memory sender,
        bytes memory withdrawer,
        bytes memory token_mint
    ) public payable returns (uint64 sequence){
        bytes memory token_stream = Encoder.encode_process_instant_token_transfer(
            Messages.ProcessTransferToken({
                amount: amount,
                toChain: CHAIN_ID,
                withdrawer: withdrawer,
                token_mint: token_mint,
                sender: sender
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            token_stream,
            CONSISTENCY_LEVEL
        );
        emit InstantTokenTransfer(sender, token_mint, amount, nonce);
        nonce = nonce + 1;
    }

    // sender will withdraw 
    function process_native_withdrawal(
        uint64 amount, 
        bytes memory sender
    ) public payable returns (uint64 sequence){
        bytes memory sol_stream = Encoder.encode_process_native_withdrawal(
            Messages.ProcessWithdraw({
                amount: amount,
                toChain: CHAIN_ID,
                withdrawer: sender
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            sol_stream,
            CONSISTENCY_LEVEL
        );
        emit NativeWithdrawal(sender, amount, nonce);
        nonce = nonce + 1;
    }

    // sender will withdraw 
    function process_token_withdrawal(
        uint64 amount, 
        bytes memory sender,
        bytes memory token_mint
    ) public payable returns (uint64 sequence){
        bytes memory token_stream = Encoder.encode_process_token_withdrawal(
            Messages.ProcessWithdrawToken({
                amount: amount,
                toChain: CHAIN_ID,
                withdrawer: sender,
                token_mint: token_mint
            })
        );
        sequence = wormhole().publishMessage(
            nonce,
            token_stream,
            CONSISTENCY_LEVEL
        );
        emit TokenWithdrawal(sender, token_mint, amount, nonce);
        nonce = nonce + 1;
    }

    /**
        Registers it's sibling applications on other chains as the only ones that can send this instance messages
     */
    function registerApplicationContracts(
        uint16 chainId,
        bytes32 applicationAddr
    ) public {
        // require(msg.sender == owner, "Only owner can register new chains!");
        _applicationContracts[chainId] = applicationAddr;
    }

    function receiveEncodedMsg(bytes memory encodedMsg) public {
        (IWormhole.VM memory vm, bool valid, string memory reason) = wormhole()
            .parseAndVerifyVM(encodedMsg);

        //1. Check Wormhole Guardian Signatures
        //  If the VM is NOT valid, will return the reason it's not valid
        //  If the VM IS valid, reason will be blank
        require(valid, reason);

        //2. Check if the Emitter Chain contract is registered
        require(
            _applicationContracts[vm.emitterChainId] == vm.emitterAddress,
            "Invalid Emitter Address!"
        );

        //3. Check that the message hasn't already been processed
        require(!_completedMessages[vm.hash], "Message already processed");
        _completedMessages[vm.hash] = true;

        //Do the thing
        current_msg = vm.payload;
    }

    function getCurrentMsg() public view returns (bytes memory) {
        return current_msg;
    }
}
