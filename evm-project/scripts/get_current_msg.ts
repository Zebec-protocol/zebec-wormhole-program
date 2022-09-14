import hardhat from "hardhat";
import { ethers } from "ethers";
import fs from "fs";
// eslint-disable-next-line node/no-missing-import
import { Messenger } from "../typechain";

export async function getCurrentMsg() {
    const signer = ethers.Wallet.fromMnemonic(
        "vanish machine bid cycle text noble index moral comic music tornado sad"
    ).connect(
        new ethers.providers.JsonRpcProvider(process.env.TILT_RPC_IP)
    );
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

    // console.log(await messenger.getSolanaMsg());
}

getCurrentMsg();
