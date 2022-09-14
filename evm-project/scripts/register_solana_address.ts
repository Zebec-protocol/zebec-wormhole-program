import hardhat from "hardhat";
import { ethers } from "ethers";
import fs from "fs";
// eslint-disable-next-line node/no-missing-import
import { Messenger } from "../typechain";
import {
    CHAIN_ID_SOLANA,
    getEmitterAddressSolana,
    setDefaultWasm,
} from "@certusone/wormhole-sdk";

async function main() {
    setDefaultWasm("node");
    const signer = ethers.Wallet.fromMnemonic(
        "vanish machine bid cycle text noble index moral comic music tornado sad"
    ).connect(new ethers.providers.JsonRpcProvider(process.env.TILT_RPC_IP));
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
            "ExoGSfFpysvXgA75oKaBf5i8cqn2DYBCf4mdi36jja5u"
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
