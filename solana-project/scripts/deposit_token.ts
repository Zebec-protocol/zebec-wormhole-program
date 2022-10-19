import fs from "fs";
import * as anchor from "@project-serum/anchor";
import * as spl from "@solana/spl-token";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import * as bip39 from "bip39";
import { Keypair, PublicKey } from "@solana/web3.js";
import { derivePath } from "ed25519-hd-key";
import { keccak_256 } from "js-sha3";
import { SolanaProject as Messenger } from "../target/types/solana_project";
import { Zebec } from "../target/types/zebec";
import {
  setDefaultWasm,
  postVaaSolanaWithRetry,
  importCoreWasm,
  tryNativeToUint8Array,
  CHAIN_ID_BSC,
  CHAIN_ID_SOLANA,
  getEmitterAddressEth,
} from "@certusone/wormhole-sdk";
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
import * as b from "byteify";
import keccak256 from "keccak256";
import { assert, expect } from "chai";

const SOLANA_CORE_BRIDGE_ADDRESS =
  "3u8hJUVTA4jH1wYAyUur7FFZVQ8H635K3tSHHF4ssjQ5";
const OPERATE = "NewVaultOption";
const OPERATEDATA = "NewVaultOptionData";
const CONN_STRING = "https://api.devnet.solana.com";
let connection = new anchor.web3.Connection(CONN_STRING);

// Wallet that will server as payer and EOA
const KEYPAIR = anchor.web3.Keypair.fromSecretKey(
  Uint8Array.from(JSON.parse(fs.readFileSync("hello.json").toString()))
);
let provider = new anchor.AnchorProvider(
  connection,
  new NodeWallet(KEYPAIR),
  {}
);

// Proxy contract
const CONTRACT_ADDRESS = "F56A1FPDGsNUrqHNjmHZ36txyDTY8VYA7UEWV4SwxQAF";
const IDL = JSON.parse(
  fs.readFileSync("target/idl/solana_project.json").toString()
);
const program = new anchor.Program<Messenger>(IDL, CONTRACT_ADDRESS, provider);

// Zebec contract
const ZEBEC_PROGRAM_ID = new anchor.web3.PublicKey(
  "dSuyjPvmWdBr68FRG9Q433Py6YxeiTMZni7WiF74GQE"
);
const IDL2 = JSON.parse(fs.readFileSync("target/idl/zebec.json").toString());
const zebecProgram = new anchor.Program<Zebec>(
  IDL2,
  ZEBEC_PROGRAM_ID,
  provider
);

// Static account
const mnemonic =
  "pill tomorrow foster begin walnut borrow virtual kick shift mutual shoe scatter";
const seed = bip39.mnemonicToSeedSync(mnemonic, "");
const ACCOUNTS = [];
for (let i = 0; i < 10; i++) {
  const path = `m/44'/501'/${i}'/0'`;
  const keypair = Keypair.fromSeed(derivePath(path, seed.toString("hex")).key);
  ACCOUNTS.push(keypair);
}

let chainId = Buffer.from("4");
let depositorHash = tryNativeToUint8Array(
  "0x30Fbf353f4f7C37952e22a9709e04b7541D5A77F",
  CHAIN_ID_BSC
);
console.log(depositorHash);
let tokenMintAddress = fs.readFileSync("StaticAddress/mint.txt").toString();
let tokenMint = new anchor.web3.PublicKey(tokenMintAddress);

let pdaSenderr = fs.readFileSync("StaticAddress/pdaSender.txt").toString();
let pdaSender = new anchor.web3.PublicKey(pdaSenderr);

let pdaSenderATAA = fs
  .readFileSync("StaticAddress/pdaSenderATA.txt")
  .toString();
let pdaSenderATA = new anchor.web3.PublicKey(pdaSenderATAA);

let zebecVaultt = fs.readFileSync("StaticAddress/zebecVault.txt").toString();
let zebecVault = new anchor.web3.PublicKey(zebecVaultt);

let zebecVaultATAA = fs
  .readFileSync("StaticAddress/zebecVaultATA.txt")
  .toString();
let zebecVaultATA = new anchor.web3.PublicKey(zebecVaultATAA);

const fundWallet = async (user: anchor.web3.PublicKey, amount: number) => {
  let txFund = new anchor.web3.Transaction();
  txFund.add(
    anchor.web3.SystemProgram.transfer({
      fromPubkey: KEYPAIR.publicKey,
      toPubkey: user,
      lamports: amount * anchor.web3.LAMPORTS_PER_SOL,
    })
  );
  try {
    const txFundTokenSig = await provider.sendAndConfirm(txFund);
  } catch (error) {
    console.log(error);
  }
};

const getTokenBalance = async (
  tokenAccount: PublicKey
): Promise<bigint | undefined> => {
  const tokenAccountInfo = await provider.connection.getAccountInfo(
    tokenAccount
  );
  const data = Buffer.from(tokenAccountInfo.data);
  const accountInfo = spl.AccountLayout.decode(data);
  return accountInfo.amount;
};

const store_msg_deposit = async () => {
  setDefaultWasm("node");

  const { parse_vaa } = await importCoreWasm();
  const vaa = fs.readFileSync("../evm-project/vaa.txt").toString();
  const vaaBytes = Buffer.from(vaa, "base64");

  //Submit to Core Bridge
  await postVaaSolanaWithRetry(
    new anchor.web3.Connection(CONN_STRING, "confirmed"),
    async (tx) => {
      tx.partialSign(KEYPAIR);
      return tx;
    },
    SOLANA_CORE_BRIDGE_ADDRESS,
    KEYPAIR.publicKey.toString(),
    vaaBytes,
    10
  );
  await new Promise((r) => setTimeout(r, 5000));

  const parsed_vaa = parse_vaa(vaaBytes);
  let emitter_address_acc = findProgramAddressSync(
    [
      Buffer.from("EmitterAddress"),
      b.serializeUint16(parsed_vaa.emitter_chain),
    ],
    program.programId
  )[0];

  let processed_vaa_key = findProgramAddressSync(
    [
      Buffer.from(
        getEmitterAddressEth(
          fs.readFileSync("../evm-project/eth-address.txt").toString()
        ),
        "hex"
      ),
      b.serializeUint16(parsed_vaa.emitter_chain),
      b.serializeUint64(parsed_vaa.sequence),
    ],
    program.programId
  )[0];

  //Create VAA Hash to use in core bridge key
  let buffer_array = [];
  buffer_array.push(b.serializeUint32(parsed_vaa.timestamp));
  buffer_array.push(b.serializeUint32(parsed_vaa.nonce));
  buffer_array.push(b.serializeUint16(parsed_vaa.emitter_chain));
  buffer_array.push(Uint8Array.from(parsed_vaa.emitter_address));
  buffer_array.push(b.serializeUint64(parsed_vaa.sequence));
  buffer_array.push(b.serializeUint8(parsed_vaa.consistency_level));
  buffer_array.push(Uint8Array.from(parsed_vaa.payload));
  const hash = keccak256(Buffer.concat(buffer_array));

  let core_bridge_vaa_key = findProgramAddressSync(
    [Buffer.from("PostedVAA"), hash],
    new anchor.web3.PublicKey(SOLANA_CORE_BRIDGE_ADDRESS)
  )[0];
  console.log("Core Bridge VAA Key: ", core_bridge_vaa_key.toString());

  let current_count = 5;
  let [dataStorage] = await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from("data_store"),
      Buffer.from(depositorHash),
      Buffer.from(current_count.toString()),
    ],
    program.programId
  );

  let [txnCount] = await PublicKey.findProgramAddress(
    [Buffer.from("txn_count"), Buffer.from(depositorHash)],
    program.programId
  );

  fs.writeFileSync("StaticAddress/dataStorage.txt", dataStorage.toBase58());
  fs.writeFileSync("StaticAddress/txnCount.txt", txnCount.toBase58());

  const tx = await program.methods
    .storeMsg(current_count, Buffer.from(depositorHash))
    .accounts({
      payer: KEYPAIR.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
      processedVaa: processed_vaa_key,
      emitterAcc: emitter_address_acc,
      coreBridgeVaa: core_bridge_vaa_key,
      dataStorage: dataStorage,
      txnCount: txnCount,
    })
    .signers([KEYPAIR])
    .rpc();
};

const readDataStorage = async (current_count) => {
  let [dataStorage] = await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from("data_store"),
      Buffer.from(depositorHash),
      Buffer.from(current_count.toString()),
    ],
    program.programId
  );

  console.log(Buffer.from(depositorHash));
  let transactionData = await program.account.transactionData.fetch(
    dataStorage
  );
  console.log(transactionData.amount.toString());
  console.log(transactionData.sender);
  console.log(transactionData.fromChainId.toString());
};

const create_and_execute = async () => {
  let dataStorage = new anchor.web3.PublicKey(
    fs.readFileSync("StaticAddress/dataStorage.txt").toString()
  );

  let txnCount = new anchor.web3.PublicKey(
    fs.readFileSync("StaticAddress/txnCount.txt").toString()
  );

  const accounts = [
    {
      pubkey: zebecVault,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: pdaSender,
      isWritable: true,
      isSigner: true,
    },
    {
      pubkey: anchor.web3.SystemProgram.programId,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: spl.TOKEN_PROGRAM_ID,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: anchor.web3.SYSVAR_RENT_PUBKEY,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: tokenMint,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: pdaSenderATA,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: zebecVaultATA,
      isWritable: true,
      isSigner: false,
    },
  ];
  const transaction = anchor.web3.Keypair.generate();
  const amount = new anchor.BN(3000);
  const data = zebecProgram.coder.instruction.encode("depositToken", {
    amount: amount,
  });

  // const txSize = getTxSize(accounts, owners, false, 8);
  // await fundWallet(zebecEOA.publicKey, 5);
  // await fundWallet(pdaSender, 2);
  const txSize = 450;
  let current_count = 5;

  await program.rpc.transactionDeposit(
    zebecProgram.programId,
    accounts,
    data,
    current_count,
    chainId,
    Buffer.from(depositorHash),
    {
      accounts: {
        transaction: transaction.publicKey,
        zebecEoa: KEYPAIR.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        dataStorage: dataStorage,
        txnCount: txnCount,
        pdaSigner: pdaSender,
      },
      instructions: [
        await program.account.transaction.createInstruction(
          transaction,
          txSize
        ),
      ],
      signers: [transaction, KEYPAIR],
      remainingAccounts: accounts
        .map((t: any) => {
          if (t.pubkey.equals(pdaSender)) {
            return { ...t, isSigner: false };
          }
          return t;
        })
        .concat({
          pubkey: zebecProgram.programId,
          isWritable: false,
          isSigner: false,
        }),
    }
  );

  const tokenbalance = await getTokenBalance(zebecVaultATA);
  assert.equal(tokenbalance.toString(), amount.toString());
};

const doTheThing = async () => {
  // await store_msg_deposit();

  await create_and_execute();
  // readDataStorage(3);
};

doTheThing();
