#! /bin/bash
# Deploy EVM
cd evm-project && npx hardhat run --network goerli scripts/deploy.ts && cd ../
# Typechain Generated 
# npx hardhat typechain
# Deploy Solana
solana config set --url devnet

#change the key pair
cd solana-project && anchor build && anchor deploy && cd../

#Register Solana Address on EVM
cd evm-project && npx hardhat run ./scripts/register_solana_address.ts && cd ../


#Initialize Solana contract
#TODO: Don't call this if the config account exists already
cd solana-project && npx ts-node ./scripts/initialize_messenger.ts && cd ../

#Send msg from EVM to Solana
cd evm-project && npx ts-node ./scripts/send_msg.ts && cd ../

#Register EVM Address on Solana
cd solana-project && npx ts-node ./scripts/register_eth_chain.ts && cd ../

#Init Zebec Contract 
cd solana-project && npx ts-node ./scripts/initialize_zebec.ts && cd ../

#Perform Test  
cd solana-project && npx ts-node ./scripts/perform_test.ts && cd ../