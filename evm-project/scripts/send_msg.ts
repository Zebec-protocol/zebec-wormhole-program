import hardhat from "hardhat";
import { BigNumber, ethers } from "ethers";
import fs from "fs";
// eslint-disable-next-line node/no-missing-import
import { Messenger } from "../typechain";
import {
    CHAIN_ID_SOLANA,
    CHAIN_ID_BSC,
    getEmitterAddressEth,
    parseSequenceFromLogEth,
    tryNativeToUint8Array,
    CHAIN_ID_ETH,
} from "@certusone/wormhole-sdk";
import fetch from "node-fetch";
import { log } from "console";
import { sign } from "crypto";

const IS_ETH = false;
const CHAIN_ID = IS_ETH ? CHAIN_ID_ETH : CHAIN_ID_BSC;

const messengerAddress = fs.readFileSync("eth-address.txt").toString();

const startTime = 1;
const endTime = 2;
const amount = 3000;
const receiver = "0xD8BeCE69d19837947b8d5963E505aed51C6F53Fa";

let tokenMint = fs
    .readFileSync("../solana-project/StaticAddress/mint.txt")
    .toString();

const receiverUint8 = tryNativeToUint8Array(receiver, CHAIN_ID);
const mintUint8 = tryNativeToUint8Array(tokenMint, CHAIN_ID_SOLANA);

export async function sendMsg() {
    const ethProvider = new ethers.providers.JsonRpcProvider(
        "https://eth-goerli.g.alchemy.com/v2/9wRFcxcjx3-SAM2OAFfTvS2GhsL1Yso0"
    );
    const bscProvider = new ethers.providers.JsonRpcProvider(
        "https://data-seed-prebsc-1-s1.binance.org:8545/"
    );
    const providers = IS_ETH ? ethProvider : bscProvider;
    const signer = ethers.Wallet.fromMnemonic(
        "vanish machine bid cycle text noble index moral comic music tornado sad"
    ).connect(providers);

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
    console.log(tryNativeToUint8Array(sender, CHAIN_ID));
    console.log(sender);
    const tx = await (
        await messenger.process_deposit_token(
            BigNumber.from(amount),
            tryNativeToUint8Array(sender, CHAIN_ID),
            mintUint8,
            BigNumber.from("10"),
            {
                gasLimit: BigNumber.from("10000000"),
                value: BigNumber.from("100"),
            }
        )
    ).wait();

    // eslint-disable-next-line promise/param-names
    await new Promise((r) => setTimeout(r, 25000));
    const emitterAddr = getEmitterAddressEth(messenger.address);
    // this is bsc testnet wormhole core bridge address
    const seq = parseSequenceFromLogEth(
        tx,
        "0x68605AD7b15c732a30b1BbC62BE8F2A509D74b4D"
        // "0x706abc4E45D419950511e474C7B9Ed348A4a716c"
    );

    const WH_DEVNET_REST = "https://wormhole-v2-testnet-api.certus.one";
    const vaaBytes = await (
        await fetch(
            `${WH_DEVNET_REST}/v1/signed_vaa/${CHAIN_ID}/${emitterAddr}/${seq}`
        )
    ).json();
    console.log(
        `${WH_DEVNET_REST}/v1/signed_vaa/${CHAIN_ID}/${emitterAddr}/${seq}`
    );

    // Submit on ETH
    fs.writeFileSync("vaa.txt", vaaBytes.vaaBytes);
}
sendMsg();
