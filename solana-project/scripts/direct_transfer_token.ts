import fs from "fs";
import * as anchor from "@project-serum/anchor";
import * as spl from "@solana/spl-token";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import * as bip39 from "bip39";
import { Keypair, PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { derivePath } from "ed25519-hd-key";
import { keccak_256 } from "js-sha3";
import { SolanaProject as Messenger } from "../target/types/solana_project";
import { Zebec } from "../target/types/zebec";
import {
  setDefaultWasm,
  postVaaSolanaWithRetry,
  importCoreWasm,
  getClaimAddressSolana,
  getEmitterAddressEth,
  tryNativeToUint8Array,
  CHAIN_ID_ETH,
  CHAIN_ID_BSC,
} from "@certusone/wormhole-sdk";
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
import * as b from "byteify";
import keccak256 from "keccak256";
import { assert, expect } from "chai";

const PREFIX_TOKEN = "withdraw_token";
const STREAM_TOKEN_SIZE =
  8 + 8 + 8 + 8 + 8 + 8 + 32 + 32 + 32 + 8 + 8 + 32 + 8 + 1 + 1;
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
  // '2V16ssMQJ1EmezfGfcNDybWtczDFEqJ73qvXE5w9FSZE8D'
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

let transaction: anchor.web3.Keypair;

let chainIdHash = Buffer.from("4");
let depositorHash = tryNativeToUint8Array(
  "0x30Fbf353f4f7C37952e22a9709e04b7541D5A77F",
  CHAIN_ID_ETH
);
let receiverHash = tryNativeToUint8Array(
  "0x30ca5c53ff960f16180aada7c38ab2572a597676",
  CHAIN_ID_ETH
);
let dataStorage;

let tokenMintAddress = fs.readFileSync("StaticAddress/mint.txt").toString();
let tokenMint = new anchor.web3.PublicKey(tokenMintAddress);

let pdaSenderr = fs.readFileSync("StaticAddress/pdaSender.txt").toString();
let pdaSender = new anchor.web3.PublicKey(pdaSenderr);

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

let feeVaultATAA = fs.readFileSync("StaticAddress/feeVaultATA.txt").toString();
let feeVaultATA = new anchor.web3.PublicKey(feeVaultATAA);

let zebecVaultt = fs.readFileSync("StaticAddress/zebecVault.txt").toString();
let zebecVault = new anchor.web3.PublicKey(zebecVaultt);

let zebecVaultATAA = fs
  .readFileSync("StaticAddress/zebecVaultATA.txt")
  .toString();
let zebecVaultATA = new anchor.web3.PublicKey(zebecVaultATAA);

//feeOwner == feeReceiver
let feeReceiverr = fs.readFileSync("StaticAddress/feeReceiver.txt").toString();
let feeReceiver = new anchor.web3.PublicKey(feeReceiverr);

//feeAccountTemp == vaultData
// let vaultDataa = fs.readFileSync('StaticAddress/vaultData.txt').toString();
// let vaultData = new anchor.web3.PublicKey(
//     vaultDataa
// );

//feeVaultTemp == feeVault
// let feeVaultt = fs.readFileSync('StaticAddress/feeVault.txt').toString();
// let feeVault = new anchor.web3.PublicKey(
//     feeVaultt
// );

const feeVault = async (
  fee_receiver: PublicKey,
  zebecProgramID: PublicKey
): Promise<anchor.web3.PublicKey> => {
  const [fee_vault, _un] = await PublicKey.findProgramAddress(
    [fee_receiver.toBuffer(), anchor.utils.bytes.utf8.encode(OPERATE)],
    zebecProgramID
  );
  return fee_vault;
};

const create_fee_account = async (
  fee_receiver: PublicKey,
  zebecProgramID: PublicKey
): Promise<anchor.web3.PublicKey> => {
  const fee_vault = await feeVault(fee_receiver, zebecProgramID);
  const [create_fee_account, _] = await PublicKey.findProgramAddress(
    [
      fee_receiver.toBuffer(),
      anchor.utils.bytes.utf8.encode(OPERATEDATA),
      fee_vault.toBuffer(),
    ],
    zebecProgramID
  );
  return create_fee_account;
};

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

const withdrawData = async (
  prefix: string,
  sender: PublicKey,
  zebecProgramID: PublicKey,
  mint?: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> => {
  if (mint) {
    const [withdrawData, bumps] = await PublicKey.findProgramAddress(
      [
        anchor.utils.bytes.utf8.encode(prefix),
        sender.toBuffer(),
        mint.toBuffer(),
      ],
      zebecProgramID
    );
    return withdrawData;
  } else {
    const [withdrawData, bumps] = await PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode(prefix), sender.toBuffer()],
      zebecProgramID
    );
    return withdrawData;
  }
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

  let current_count = 1;
  let [dataStorage] = await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from("data_store"),
      depositorHash,
      Buffer.from(current_count.toString()),
    ],
    program.programId
  );

  let [txnCount] = await PublicKey.findProgramAddress(
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

const create_and_execute = async () => {
  let dataStorage = new anchor.web3.PublicKey(
    fs.readFileSync("StaticAddress/dataStorage.txt").toString()
  );

  let txnCount = new anchor.web3.PublicKey(
    fs.readFileSync("StaticAddress/txnCount.txt").toString()
  );

  let dataAccount = new anchor.web3.PublicKey(
    fs.readFileSync("StaticAddress/dataAccount.txt").toString()
  );

  let config = new anchor.web3.PublicKey(
    fs.readFileSync("StaticAddress/config.txt").toString()
  );

  let withdrawDatatemp = await withdrawData(
    PREFIX_TOKEN,
    pdaSender,
    zebecProgram.programId,
    tokenMint
  );

  let feeAccountTemp = await create_fee_account(
    feeReceiver,
    zebecProgram.programId
  );

  let feeVaultTemp = await feeVault(feeReceiver, zebecProgram.programId);

  const accounts = [
    //source account
    {
      pubkey: pdaSender,
      isWritable: true,
      isSigner: true,
    },
    //destination account
    {
      pubkey: pdaReceiver,
      isWritable: true,
      isSigner: false,
    },
    //systemProgram
    {
      pubkey: anchor.web3.SystemProgram.programId,
      isWritable: false,
      isSigner: false,
    },
    //tokenProgram
    {
      pubkey: spl.TOKEN_PROGRAM_ID,
      isWritable: false,
      isSigner: false,
    },
    //associatedTokenProgram
    {
      pubkey: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
      isWritable: false,
      isSigner: false,
    },
    //rent
    {
      pubkey: anchor.web3.SYSVAR_RENT_PUBKEY,
      isWritable: false,
      isSigner: false,
    },
    //mint
    {
      pubkey: tokenMint,
      isWritable: false,
      isSigner: false,
    },
    //sourceTokenAccount
    {
      pubkey: pdaSenderATA,
      isWritable: true,
      isSigner: false,
    },
    //destinationTokenAccount
    {
      pubkey: pdaReceiverATA,
      isWritable: true,
      isSigner: false,
    },
  ];

  const data = zebecProgram.coder.instruction.encode("sendTokenDirectly", {});
  const txSize = 1232;
  const transaction = anchor.web3.Keypair.generate();
  let current_count_receiver = 1;

  const tokenbalanceBefore = await getTokenBalance(pdaReceiverATA);

  const tx = await program.rpc.transactionDirectTransfer(
    zebecProgram.programId,
    accounts,
    data,
    current_count_receiver,
    Buffer.from(CHAIN_ID_BSC.toString()),
    CHAIN_ID_BSC,
    new anchor.BN("100"),
    {
      accounts: {
        transaction: transaction.publicKey,
        zebecEoa: KEYPAIR.publicKey,
        dataStorage: dataStorage,
        txnCount: txnCount,
        pdaSigner: pdaSender,
        config: config,
        //portal config,
        //from
        //mint,
        //portal_custody
        //portal_authority_signer
        //portal_custody_signer,
        //bridge_config,
        //portal_message,
        //portal_emitter,
        //portal_sequence,
        //bridge_fee_collector,
        //clock,
        //rent,
        //system_program,
        //portal_bridge_program,
        //core_bridge_program,
        //token_program
        systemProgram: anchor.web3.SystemProgram.programId,
      },
      instructions: [
        await program.account.transaction.createInstruction(
          transaction,
          txSize
        ),
      ],
      signers: [transaction, KEYPAIR],
    }
  );
};

const doTheThing = async () => {
  await store_msg_direct_transfer();

  await create_and_execute();
};

doTheThing();
