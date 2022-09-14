import hardhat from "hardhat";
import { BigNumber, ethers } from "ethers";
import fs from "fs";
// eslint-disable-next-line node/no-missing-import
import { Messenger } from "../typechain";
import {
    CHAIN_ID_ETH,
    CHAIN_ID_SOLANA,
    getEmitterAddressEth,
    parseSequenceFromLogEth,
    tryNativeToUint8Array,
} from "@certusone/wormhole-sdk";
import fetch from "node-fetch";
import { log } from "console";
import { sign } from "crypto";

const signer = ethers.Wallet.fromMnemonic(
    "vanish machine bid cycle text noble index moral comic music tornado sad"
).connect(new ethers.providers.JsonRpcProvider(process.env.TILT_RPC_IP));
const messengerAddress = fs.readFileSync("eth-address.txt").toString();

const startTime = 1;
const endTime = 2;
const amount = 3000;
const receiver = "0xD8BeCE69d19837947b8d5963E505aed51C6F53Fa";
const tokenMint = "B5Qfv5w6a7e46S8ZByprsAkNkqmV1BquZCkiT41YcE63";
const receiverUint8 = tryNativeToUint8Array(receiver, CHAIN_ID_ETH);
const mintUint8 = tryNativeToUint8Array(tokenMint, CHAIN_ID_SOLANA);
console.log(mintUint8);

export async function sendMsg() {
    const messenger = new ethers.Contract(
        messengerAddress,
        (
            await hardhat.artifacts.readArtifact(
                "contracts/Messenger.sol:Messenger"
            )
        ).abi,
        signer
    ) as Messenger;

    const sender = await signer.getAddress();
                
    const tx = await (
        await messenger.process_deposit_token(
            BigNumber.from(amount),
            tryNativeToUint8Array(sender, CHAIN_ID_ETH),
            mintUint8,
            { gasLimit: 100000 }
        )
    ).wait();

    // eslint-disable-next-line promise/param-names
    await new Promise((r) => setTimeout(r, 25000));
    const emitterAddr = getEmitterAddressEth(messenger.address);
    // this is goerili wormhole core bridge address
    const seq = parseSequenceFromLogEth(
        tx,
        "0x706abc4E45D419950511e474C7B9Ed348A4a716c"
    );

    const WH_DEVNET_REST = "https://wormhole-v2-testnet-api.certus.one";
    const vaaBytes = await (
        await fetch(
            `${WH_DEVNET_REST}/v1/signed_vaa/${CHAIN_ID_ETH}/${emitterAddr}/${seq}`
        )
    ).json();
    console.log(
        `${WH_DEVNET_REST}/v1/signed_vaa/${CHAIN_ID_ETH}/${emitterAddr}/${seq}`
    );

    // Submit on ETH
    fs.writeFileSync("vaa.txt", vaaBytes.vaaBytes);
}
sendMsg();
