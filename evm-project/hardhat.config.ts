import * as dotenv from "dotenv";

import { HardhatUserConfig, task } from "hardhat/config";
import "@nomiclabs/hardhat-etherscan";
import "@nomiclabs/hardhat-waffle";
import "@typechain/hardhat";
import "hardhat-gas-reporter";
import "solidity-coverage";

dotenv.config();

// This is a sample Hardhat task. To learn how to create your own go to
// https://hardhat.org/guides/create-task.html
task("accounts", "Prints the list of accounts", async (taskArgs, hre) => {
    const accounts = await hre.ethers.getSigners();

    for (const account of accounts) {
        console.log(account.address);
    }
});

// You need to export an object to set up your config
// Go to https://hardhat.org/config/ to learn more

const config: HardhatUserConfig = {
    solidity: "0.8.4",
    networks: {
        ropsten: {
            url: process.env.ROPSTEN_URL || "",
            accounts:
                process.env.PRIVATE_KEY !== undefined
                    ? [process.env.PRIVATE_KEY]
                    : [],
        },
        goerli: {
            url: "https://eth-goerli.g.alchemy.com/v2/9wRFcxcjx3-SAM2OAFfTvS2GhsL1Yso0",
            accounts: [
                "7c56131ac2d675249d73fb032de5533f183f36b2aa2e82ab163e88ded1be3b39",
            ],
            // gas: 2100000,
            // gasPrice: 8000000000
        },
        smartChain: {
            url: "https://data-seed-prebsc-1-s1.binance.org:8545/",
            accounts: [
                "7c56131ac2d675249d73fb032de5533f183f36b2aa2e82ab163e88ded1be3b39",
            ],
            // gas: 2100000,
            // gasPrice: 8000000000
        },
        localhost: {
            url: "http://localhost:8545/",
            accounts: {
                mnemonic:
                    "myth like bonus scare over problem client lizard pioneer submit female collect",
            },
        },
        tilt: {
            url: `${process.env.TILT_RPC_IP}`,
            accounts: {
                mnemonic:
                    "myth like bonus scare over problem client lizard pioneer submit female collect",
            },
        },
    },
    gasReporter: {
        enabled: process.env.REPORT_GAS !== undefined,
        currency: "USD",
    },
    etherscan: {
        apiKey: "JQPUP8U38ICFHHKU5ZZVNRVWWCWI6KYK2R",
    },
};

export default config;
