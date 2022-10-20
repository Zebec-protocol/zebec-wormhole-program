import hardhat from "hardhat";
import { ethers } from "ethers";
import fs from "fs";
// eslint-disable-next-line node/no-missing-import
import { Messenger } from "../typechain";
import {
    CHAIN_ID_SOLANA,
    CHAIN_ID_BSC,
    CHAIN_ID_ETH,
    getEmitterAddressSolana,
    setDefaultWasm,
} from "@certusone/wormhole-sdk";

const IS_ETH = false;
const CHAIN_ID = IS_ETH ? CHAIN_ID_ETH: CHAIN_ID_BSC;

async function main() {
    setDefaultWasm("node");
    const ethProvider = new ethers.providers.JsonRpcProvider("https://eth-goerli.g.alchemy.com/v2/9wRFcxcjx3-SAM2OAFfTvS2GhsL1Yso0");
    const bscProvider = new ethers.providers.JsonRpcProvider("https://data-seed-prebsc-1-s1.binance.org:8545/");
    const providers = IS_ETH ? ethProvider : bscProvider;
    const signer = ethers.Wallet.fromMnemonic(
        "vanish machine bid cycle text noble index moral comic music tornado sad"
    ).connect(providers); 
    const messengerAddress = fs.readFileSync("eth-address.txt").toString();

    const messenger = new ethers.Contract(
        messengerAddress,
        (
            await hardhat.artifacts.readArtifact(
                "contracts/Messenger.sol:Messenger"
            )
        ).abi,
        signer
    ) as Messenger;

    const solanaAddr = Buffer.from(
        await getEmitterAddressSolana(
            "F56A1FPDGsNUrqHNjmHZ36txyDTY8VYA7UEWV4SwxQAF"
        ),
        "hex"
    );
    messenger.registerApplicationContracts(CHAIN_ID_SOLANA, solanaAddr);

    // to check if properly set
    // let pid = await messenger.getMapping(CHAIN_ID_SOLANA);
    // console.log("solana program", pid, CHAIN_ID_SOLANA);
    // console.log(solanaAddr.toString('hex'));
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
});
