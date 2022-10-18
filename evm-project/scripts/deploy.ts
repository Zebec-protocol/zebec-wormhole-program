// We require the Hardhat Runtime Environment explicitly here. This is optional
// but useful for running the script in a standalone fashion through `node <script>`.
//
// When running the script with `npx hardhat run <script>` you'll find the Hardhat
// Runtime Environment's members available in the global scope.
import { ethers } from "hardhat";
import fs from "fs";

async function main() {
    // Hardhat always runs the compile task when running scripts with its command
    // line interface.
    //
    // If this script is run directly using `node` you may want to call compile
    // manually to make sure everything is compiled
    // await hre.run('compile');

    // We get the contract to deploy

    const Messenger = await ethers.getContractFactory("Messenger");
    //BSC
    const messenger = await Messenger.deploy("0x68605AD7b15c732a30b1BbC62BE8F2A509D74b4D", "100","0xae13d989dac2f0debff460ac112a837c89baa7cd");
    //ETH
    // const messenger = await Messenger.deploy("0x706abc4E45D419950511e474C7B9Ed348A4a716c", "100","0xB4FBF271143F4FBf7B91A5ded31805e42b2208d6");
    await messenger.deployed();
    console.log("Messenger deployed to address: ", messenger.address);
    fs.writeFileSync("eth-address.txt", messenger.address);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
});
