import * as bip39 from "bip39";
import * as b from "byteify";
import { derivePath } from "ed25519-hd-key";
import fs, { cpSync } from "fs";
import keccak256 from "keccak256";

import {
  CHAIN_ID_BSC,
  getEmitterAddressEth,
  importCoreWasm,
  postVaaSolanaWithRetry,
  setDefaultWasm,
  tryNativeToUint8Array,
  getBridgeFeeIx,
} from "@certusone/wormhole-sdk";
import * as anchor from "@project-serum/anchor";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
import * as spl from "@solana/spl-token";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  Transaction,
} from "@solana/web3.js";

import { SolanaProject as Messenger } from "../target/types/solana_project";

const PREFIX_TOKEN = "withdraw_token";
const STREAM_TOKEN_SIZE =
  8 + 8 + 8 + 8 + 8 + 8 + 32 + 32 + 32 + 8 + 8 + 32 + 8 + 1 + 1;
const SOLANA_CORE_BRIDGE_ADDRESS =
  "3u8hJUVTA4jH1wYAyUur7FFZVQ8H635K3tSHHF4ssjQ5";
const SOLANA_TOKEN_BRIDGE_ADDRESS =
  "DZnkkTmCiFWfYTfT41X3Rd1kDgozqzxWaHqsw6W4x2oe";
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

let chainIdHash = Buffer.from("4");
let depositorHash = tryNativeToUint8Array(
  "0x30Fbf353f4f7C37952e22a9709e04b7541D5A77F",
  CHAIN_ID_BSC
);
let receiverHash = tryNativeToUint8Array(
  "0xD8BeCE69d19837947b8d5963E505aed51C6F53Fa",
  CHAIN_ID_BSC
);
let dataStorage;
let bumps;

let tokenMintAddress = fs.readFileSync("StaticAddress/mint.txt").toString();
let tokenMint = new anchor.web3.PublicKey(tokenMintAddress);

let pdaSenderr = fs.readFileSync("StaticAddress/pdaSender.txt").toString();
let pdaSender = new anchor.web3.PublicKey(pdaSenderr);
// console.log("PDA Sender", pdaSender.toString());
// console.log("Depositor Hash", depositorHash.toString());

// const [pdaSender, nonce] = anchor.web3.PublicKey.findProgramAddressSync(
//   [depositorHash, Buffer.from(CHAIN_ID_BSC.toString())],
//   program.programId
// );

let pdaReceiverr = fs.readFileSync("StaticAddress/pdaReceiver.txt").toString();
let pdaReceiver = new anchor.web3.PublicKey(pdaReceiverr);

let pdaSenderATAA = fs
  .readFileSync("StaticAddress/pdaSenderATA.txt")
  .toString();
let pdaSenderATA = new anchor.web3.PublicKey(pdaSenderATAA);

let pdaReceiverATAA = fs
  .readFileSync("StaticAddress/pdaReceiverATA.txt")
  .toString();
let pdaReceiverATA = new anchor.web3.PublicKey(pdaReceiverATAA);

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

const getCurrentCount = async () => {
  let [txnCount, _] = await PublicKey.findProgramAddress(
    [anchor.utils.bytes.utf8.encode("txn_count"), depositorHash],
    program.programId
  );
  const current_counting = await program.account.count.fetch(txnCount);
  console.log(current_counting.count);
  return current_counting.count + 1;
};

const store_msg_direct_transfer = async () => {
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

  let current_count = await getCurrentCount();
  let [dataStorage] = await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from("data_store"),
      depositorHash,
      Buffer.from(current_count.toString()),
    ],
    program.programId
  );

  let [txnCount, _] = await PublicKey.findProgramAddress(
    [anchor.utils.bytes.utf8.encode("txn_count"), depositorHash],
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

const direct_transfer_native = async () => {
  let current_count = await getCurrentCount();
  let [dataStorage] = await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from("data_store"),
      depositorHash,
      Buffer.from((current_count - 1).toString()),
    ],
    program.programId
  );

  let txnCount = new anchor.web3.PublicKey(
    fs.readFileSync("StaticAddress/txnCount.txt").toString()
  );
  // let config = new anchor.web3.PublicKey(
  //   fs.readFileSync("StaticAddress/config.txt").toString()
  // );

  const [config] = PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    program.programId
  );

  // const countInfo = await provider.connection.getAccountInfo(
  //   txnCount,
  //   "confirmed"
  // );

  // const tokenbalanceBefore = await getTokenBalance(pdaReceiverATA);
  const mint = tokenMint;
  const pdaSigner = pdaSender;
  const from = getAssociatedTokenAddressSync(mint, pdaSigner, true);
  const coreBridgeProgram = new PublicKey(SOLANA_CORE_BRIDGE_ADDRESS);
  const portalBridgeProgram = new PublicKey(SOLANA_TOKEN_BRIDGE_ADDRESS);
  const [bridgeConfig] = PublicKey.findProgramAddressSync(
    [Buffer.from("Bridge")],
    coreBridgeProgram
  );
  const [bridgeFeeCollector] = PublicKey.findProgramAddressSync(
    [Buffer.from("fee_collector")],
    coreBridgeProgram
  );
  const [portalAuthoritySigner] = PublicKey.findProgramAddressSync(
    [Buffer.from("authority_signer")],
    portalBridgeProgram
  );
  const [portalConfig] = PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    portalBridgeProgram
  );
  const [portalCustody] = PublicKey.findProgramAddressSync(
    [mint.toBuffer()],
    portalBridgeProgram
  );
  const [portalCustodySigner] = PublicKey.findProgramAddressSync(
    [Buffer.from("custody_signer")],
    portalBridgeProgram
  );
  const [portalEmitter] = PublicKey.findProgramAddressSync(
    [Buffer.from("emitter")],
    portalBridgeProgram
  );
  const messageKeypair = Keypair.generate();
  const portalMessage = messageKeypair.publicKey;
  const [portalSequence] = PublicKey.findProgramAddressSync(
    [Buffer.from("Sequence"), portalEmitter.toBuffer()],
    coreBridgeProgram
  );
  const zebecEoa = KEYPAIR.publicKey;

  console.log("Pda Sender", pdaSender.toBase58());
  console.log("config:", config.toString());
  console.log("dataStorage:", dataStorage.toString());
  console.log("coreBridgeProgram:", coreBridgeProgram.toString());
  console.log("portalBridgeProgram", portalBridgeProgram.toString());
  console.log("bridgeConfig", bridgeConfig.toString());
  console.log("bridgeFeeCollector", bridgeFeeCollector.toString());
  console.log("portalAuthoritySigner", portalAuthoritySigner.toString());
  console.log("portalConfig", portalConfig.toString());
  console.log("portalCustody", portalCustody.toString());
  console.log("portalCustodySigner", portalCustodySigner.toString());
  console.log("portalEmitter", portalEmitter.toString());
  console.log("portalMessage", portalMessage.toString());
  console.log("portalSequence", portalSequence.toString());
  console.log("senderATA", from.toString());

  const transaction = new Transaction();

  setDefaultWasm("node");

  const transferFeeIxn = await getBridgeFeeIx(
    provider.connection,
    SOLANA_CORE_BRIDGE_ADDRESS,
    zebecEoa.toString()
  );

  console.log("transferFeeIxn", transferFeeIxn);

  const tx = await program.methods
    .transactionDirectTransferNative(
      Buffer.from(depositorHash),
      Buffer.from(CHAIN_ID_BSC.toString()),
      current_count - 1,
      CHAIN_ID_BSC,
      new anchor.BN("10000")
    )
    .accounts({
      zebecEoa: zebecEoa,
      dataStorage: dataStorage,
      txnCount: txnCount,
      pdaSigner: pdaSigner,
      config: config,
      portalConfig: portalConfig,
      from: from,
      mint: mint,
      portalCustody: portalCustody,
      portalAuthoritySigner: portalAuthoritySigner,
      portalCustodySigner: portalCustodySigner,
      bridgeConfig: bridgeConfig,
      portalMessage: portalMessage,
      portalEmitter: portalEmitter,
      portalSequence: portalSequence,
      bridgeFeeCollector: bridgeFeeCollector,
      clock: SYSVAR_CLOCK_PUBKEY,
      rent: SYSVAR_RENT_PUBKEY,
      systemProgram: SystemProgram.programId,
      portalBridgeProgram: portalBridgeProgram,
      coreBridgeProgram: coreBridgeProgram,
      tokenProgram: spl.TOKEN_PROGRAM_ID,
    })
    // .signers([KEYPAIR, messageKeypair])
    .instruction();

  transaction.add(transferFeeIxn, tx);
  const { blockhash, lastValidBlockHeight } =
    await provider.connection.getLatestBlockhash();

  transaction.recentBlockhash = blockhash;
  transaction.lastValidBlockHeight = lastValidBlockHeight;
  transaction.feePayer = zebecEoa;
  transaction.partialSign(messageKeypair);
  await provider.wallet.signTransaction(transaction);

  const signature = await provider.connection.sendRawTransaction(
    transaction.serialize()
  );

  const confirmation = await provider.connection.confirmTransaction({
    signature,
    blockhash,
    lastValidBlockHeight,
  });
};

const doTheThing = async () => {
  const count = await getCurrentCount();
  console.log("current_count", count);

  // await store_msg_direct_transfer();

  await direct_transfer_native();
};

doTheThing();
