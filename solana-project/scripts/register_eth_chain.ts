import * as anchor from '@project-serum/anchor';
import { SolanaProject as Messenger } from '../target/types/solana_project';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import fs from 'fs';
import { findProgramAddressSync } from '@project-serum/anchor/dist/cjs/utils/pubkey';
import * as b from 'byteify';

import {
  CHAIN_ID_BSC,
  getEmitterAddressEth,
  setDefaultWasm,
} from '@certusone/wormhole-sdk';

async function register_eth_address() {
  setDefaultWasm('node');
  const KEYPAIR = anchor.web3.Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync('hello.json').toString()))
  );
  const CONN_STRING = 'https://api.devnet.solana.com';
  const CONTRACT_ADDRESS = 'F56A1FPDGsNUrqHNjmHZ36txyDTY8VYA7UEWV4SwxQAF';
  const IDL = JSON.parse(
    fs.readFileSync('target/idl/solana_project.json').toString()
  );
  const program = new anchor.Program<Messenger>(
    IDL,
    CONTRACT_ADDRESS,
    new anchor.AnchorProvider(
      new anchor.web3.Connection(CONN_STRING),
      new NodeWallet(KEYPAIR),
      {}
    )
  );

  const ethAddress = getEmitterAddressEth(
    fs.readFileSync('../evm-project/eth-address.txt').toString()
  );

  await program.methods
    .registerChain(CHAIN_ID_BSC, ethAddress)
    .accounts({
      owner: KEYPAIR.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
      config: findProgramAddressSync(
        [Buffer.from('config')],
        program.programId
      )[0],
      emitterAcc: findProgramAddressSync(
        [Buffer.from('EmitterAddress'), b.serializeUint16(CHAIN_ID_BSC)],
        program.programId
      )[0],
    })
    .rpc();
}

register_eth_address();
