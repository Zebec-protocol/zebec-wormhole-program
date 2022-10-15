// SPDX-License-Identifier: Apache 2
pragma solidity ^0.8.0;

import "./interfaces/IWormhole.sol";
import "./interfaces/IWETH.sol";
import "./interfaces/IERC20.sol";
import "./Encoder.sol";

contract Messenger is Encoder {
    bytes16 private constant _HEX_SYMBOLS = "0123456789abcdef";
    uint8 private constant _ADDRESS_LENGTH = 20;

    address public owner;
    uint8 public constant CONSISTENCY_LEVEL = 1; //15
    uint32 nonce = 0;

    IWormhole public _wormhole;
    IWETH public _weth;
    uint256 public _wormhole_fee;
    
    mapping(uint16 => bytes32) public _applicationContracts;

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
    
    event DirectTransfer(bytes sender, bytes receiver, bytes tokenMint, uint64 amount, uint32 nonce);

    constructor(address wormholeAddress, uint256 wormholeFee, address weth) {
        _wormhole = IWormhole(wormholeAddress); //0x706abc4E45D419950511e474C7B9Ed348A4a716c
        _weth = IWETH(weth); //0xB4FBF271143F4FBf7B91A5ded31805e42b2208d6
        _wormhole_fee = wormholeFee;
        owner = msg.sender;
    }

    function process_deposit_sol(
        uint64 amount, 
        bytes memory depositor,
        uint256 arbiter_fee
    ) public payable {
        nonce++;
        bytes memory encoded_data = Encoder.encode_process_deposit_sol(
            Messages.ProcessDeposit({
                amount: amount,
                toChain: getChainId(),
                depositor: depositor
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit DepositSol(depositor, amount, nonce);
    }

    function process_deposit_token(
        uint64 amount, 
        bytes memory depositor,
        bytes memory token_mint,
        uint256 arbiter_fee
    ) public payable {
        nonce++;
        bytes memory encoded_data = Encoder.encode_process_deposit_token(
            Messages.ProcessDepositToken({
                amount: amount,
                toChain: getChainId(),
                depositor: depositor,
                token_mint: token_mint
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit DepositToken(depositor, token_mint, amount, nonce);
    }

    function process_native_stream(
        uint64 start_time,
        uint64 end_time,
        uint64 amount,
        bytes memory receiver,
        bytes memory sender,
        uint64 can_cancel,
        uint64 can_update,
        uint256 arbiter_fee
    ) public payable  {
        nonce++;
        bytes memory encoded_data = Encoder.encode_native_stream(
            Messages.ProcessStream({
                start_time: start_time,
                end_time: end_time,
                amount: amount,
                toChain: getChainId(),
                sender: sender,
                receiver: receiver,
                can_cancel: can_cancel,
                can_update: can_update
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit NativeStream(sender, receiver, amount, nonce);
    }

    function process_token_stream(
        uint64 start_time,
        uint64 end_time,
        uint64 amount,
        bytes memory receiver,
        bytes memory sender,
        uint64 can_cancel,
        uint64 can_update,
        bytes memory token_mint,
        uint256 arbiter_fee
    ) public payable  {
        nonce++;
        bytes memory encoded_data = Encoder.encode_token_stream(
            Messages.ProcessStreamToken({
                start_time: start_time,
                end_time: end_time,
                amount: amount,
                toChain: getChainId(),
                sender: sender,
                receiver: receiver,
                can_cancel: can_cancel,
                can_update: can_update,
                token_mint: token_mint
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit TokenStream(sender, receiver, token_mint, amount, nonce);
    }

    function process_native_stream_update(
        uint64 start_time,
        uint64 end_time,
        uint64 amount,
        bytes memory receiver,
        bytes memory sender,
        uint256 arbiter_fee
    ) public payable  {
        nonce++;
        bytes memory encoded_data = Encoder.encode_native_stream_update(
            Messages.UpdateStream({
                start_time: start_time,
                end_time: end_time,
                amount: amount,
                toChain: getChainId(),
                sender: sender,
                receiver: receiver
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit NativeStreamUpdate(sender, receiver, amount, nonce);
    }

    function process_token_stream_update(
        uint64 start_time,
        uint64 end_time,
        uint64 amount,
        bytes memory receiver,
        bytes memory sender,
        bytes memory token_mint,
        bytes memory data_account_address,
        uint256 arbiter_fee
    ) public payable  {
        nonce++;
        bytes memory encoded_data = Encoder.encode_token_stream_update(
            Messages.UpdateStreamToken({
                start_time: start_time,
                end_time: end_time,
                amount: amount,
                toChain: getChainId(),
                sender: sender,
                receiver: receiver,
                token_mint: token_mint,
                data_account_address: data_account_address
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit TokenStreamUpdate(sender, receiver, token_mint, amount, nonce);
    }

    // receiver will withdraw using this
    function process_native_withdraw_stream(
        bytes memory withdrawer,
        uint256 arbiter_fee
    ) public payable {
        nonce++;
        bytes memory encoded_data = Encoder.encode_native_withdraw_stream(
            Messages.ProcessWithdrawStream({
                toChain: getChainId(),
                withdrawer: withdrawer
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit WithdrawStream(withdrawer, nonce);
    }

    function process_token_withdraw_stream(
        bytes memory withdrawer,
        bytes memory token_mint,
        bytes memory sender_address,
        bytes memory data_account_address,
        uint256 arbiter_fee
    ) public payable  {
        nonce++;
        bytes memory encoded_data = Encoder.encode_token_withdraw_stream(
            Messages.ProcessWithdrawStreamToken({
                toChain: getChainId(),
                withdrawer: withdrawer,
                token_mint: token_mint,
                sender_address: sender_address,
                data_account_address: data_account_address
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit WithdrawToken(withdrawer, token_mint, nonce);
    }

    function process_pause_native_stream(
        bytes memory sender,
        uint256 arbiter_fee
    ) public payable  {
        nonce++;
        bytes memory encoded_data = Encoder.encode_process_pause_native_stream(
            Messages.PauseStream({
                toChain: getChainId(),
                sender: sender
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit PauseNativeStream(sender, nonce);
    }

   function process_pause_token_stream(
        bytes memory sender,
        bytes memory token_mint,
        bytes memory reciever_address,
        bytes memory data_account_address,
        uint256 arbiter_fee
    ) public payable  {
        nonce++;
        bytes memory encoded_data = Encoder.encode_process_pause_token_stream(
            Messages.PauseStreamToken({
                toChain: getChainId(),
                sender: sender,
                token_mint: token_mint,
                reciever_address: reciever_address,
                data_account_address: data_account_address
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit PauseTokenStream(sender, token_mint, nonce);
    }

    function process_cancel_native_stream(
        bytes memory sender,
        uint256 arbiter_fee

    ) public payable  {
        nonce++;
        bytes memory encoded_data = Encoder.encode_process_cancel_native_stream(
            Messages.CancelStream({
                toChain: getChainId(),
                sender: sender
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit CancelNativeStream(sender, nonce);
    }

    function process_cancel_token_stream(
        bytes memory sender,
        bytes memory token_mint,
        bytes memory reciever_address,
        bytes memory data_account_address,
        uint256 arbiter_fee
    ) public payable  {
        nonce++;
        bytes memory encoded_data = Encoder.encode_process_cancel_token_stream(
            Messages.CancelStreamToken({
                toChain: getChainId(),
                sender: sender,
                token_mint: token_mint,
                reciever_address: reciever_address,
                data_account_address: data_account_address
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit CancelTokenStream(sender, token_mint, nonce);
    }

    // sender will transfer to receiver
    function process_instant_native_transfer(
        uint64 amount, 
        bytes memory sender,
        bytes memory withdrawer,
        uint256 arbiter_fee
    ) public payable {
        nonce++;
        bytes memory encoded_data = Encoder.encode_process_instant_native_transfer(
            Messages.ProcessTransfer({
                amount: amount,
                toChain: getChainId(),
                withdrawer: withdrawer,
                sender: sender
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit InstantNativeTransfer(sender, amount, nonce);
    }

    // sender will transfer to receiver
    function process_instant_token_transfer(
        uint64 amount, 
        bytes memory sender,
        bytes memory withdrawer,
        bytes memory token_mint,
        uint256 arbiter_fee
    ) public payable {
        nonce++;
        bytes memory encoded_data = Encoder.encode_process_instant_token_transfer(
            Messages.ProcessTransferToken({
                amount: amount,
                toChain: getChainId(),
                receiver: withdrawer,
                token_mint: token_mint,
                sender: sender
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit InstantTokenTransfer(sender, token_mint, amount, nonce);
    }

    // sender will withdraw 
    function process_native_withdrawal(
        uint64 amount, 
        bytes memory sender,
        uint256 arbiter_fee
    ) public payable {
        nonce++;
        bytes memory encoded_data = Encoder.encode_process_native_withdrawal(
            Messages.ProcessWithdraw({
                amount: amount,
                toChain: getChainId(),
                withdrawer: sender
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit NativeWithdrawal(sender, amount, nonce);
    }

    // sender will withdraw 
    function process_token_withdrawal(
        uint64 amount, 
        bytes memory sender,
        bytes memory token_mint,
        uint256 arbiter_fee
    ) public payable {
        nonce++;
        bytes memory encoded_data = Encoder.encode_process_token_withdrawal(
            Messages.ProcessWithdrawToken({
                amount: amount,
                toChain: getChainId(),
                withdrawer: sender,
                token_mint: token_mint
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit TokenWithdrawal(sender, token_mint, amount, nonce);
    }

    function process_direct_transfer(
        uint64 amount, 
        bytes memory sender,
        bytes memory token_mint,
        bytes memory receiver,
        uint256 arbiter_fee
    ) public payable {
        nonce++;
        bytes memory encoded_data = Encoder.encode_process_direct_transfer(
            Messages.ProcessTransferToken({
                amount: amount,
                toChain: getChainId(),
                receiver: receiver,
                token_mint: token_mint,
                sender: sender
            })
        );
         _bridgeInstructionInWormhole(
            nonce,
            encoded_data,
            arbiter_fee
        );
        emit DirectTransfer(sender, receiver, token_mint, amount, nonce);
    }

    function _bridgeInstructionInWormhole(uint32 nonceValue, bytes memory stream, uint256 arbiterFee) internal returns(uint64 sequence){

        uint256 wormholeFee = _wormhole.messageFee();

        require(wormholeFee < msg.value, "value is smaller than wormhole fee");

        uint256 amount = msg.value - wormholeFee;

        require(arbiterFee <= amount, "fee is bigger than amount minus wormhole fee");

        uint256 normalizedAmount = normalizeAmount(amount, 18);
        uint256 normalizedArbiterFee = normalizeAmount(arbiterFee, 18);

        // refund dust
        uint dust = amount - deNormalizeAmount(normalizedAmount, 18) - deNormalizeAmount(normalizedArbiterFee, 18);
        if (dust > 0) {
            payable(msg.sender).transfer(dust);
        }

        // deposit into WETH
        _weth.deposit{
            value : amount - dust
        }();

        sequence = _wormhole.publishMessage(nonceValue, stream, CONSISTENCY_LEVEL);
    }

    function normalizeAmount(uint256 amount, uint8 decimals) internal pure returns(uint256){
        if (decimals > 8) {
            amount /= 10 ** (decimals - 8);
        }
        return amount;
    }

    function deNormalizeAmount(uint256 amount, uint8 decimals) internal pure returns(uint256){
        if (decimals > 8) {
            amount *= 10 ** (decimals - 8);
        }
        return amount;
    }

    receive() external payable {}

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

     function getChainId() internal view returns (uint256) {
        uint256 chainId;
        assembly { chainId := chainid() }
        return chainId;
    }

    function changeAdmin(address _owner) public {
        require(msg.sender == owner, "Only owner can change admin!");
        owner = _owner;
    }

    function claimEthAmount() public {
        require(msg.sender == owner, "Only owner can withdraw funds!");
        uint256 _contractBalance = address(this).balance;
        require( _contractBalance > 0 , "No ETH accumulated");

        (bool _sent,) = owner.call{value: _contractBalance}("");
        require(_sent, "Failed to send Ether");
    }

    function claimWETHAmount(uint256 amount) public {
        require(msg.sender == owner, "Only owner can withdraw funds!");

        _weth.withdraw(amount);
        (bool _sent,) = owner.call{value: amount}("");
        require(_sent, "Failed to send Ether");
    }
}