import * as bip39 from "bip39";
import { derivePath } from "ed25519-hd-key";
import fs from "fs";

import { CHAIN_ID_BSC, tryNativeToUint8Array } from "@certusone/wormhole-sdk";
import * as anchor from "@project-serum/anchor";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import * as spl from "@solana/spl-token";
import { Keypair, PublicKey } from "@solana/web3.js";

import { SolanaProject as Messenger } from "../target/types/solana_project";
import { Zebec } from "../target/types/zebec";

const OPERATE = "NewVaultOption";
const OPERATEDATA = "NewVaultOptionData";
const CONN_STRING = "https://api.devnet.solana.com";
let connection = new anchor.web3.Connection(CONN_STRING);

// Wallet that will server as payer and EOA
const KEYPAIR = anchor.web3.Keypair.fromSecretKey(
  Uint8Array.from(JSON.parse(fs.readFileSync("./hello.json").toString()))
);
let provider = new anchor.AnchorProvider(
  connection,
  new NodeWallet(KEYPAIR),
  {}
);

// Proxy contract
const CONTRACT_ADDRESS = "F56A1FPDGsNUrqHNjmHZ36txyDTY8VYA7UEWV4SwxQAF";
const IDL = JSON.parse(
  fs.readFileSync("./target/idl/solana_project.json").toString()
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
let fee_receiver = ACCOUNTS[2];

let depositorHash;
let chainId;
let receiverHash;
let tokenMintAddress = fs.readFileSync("StaticAddress/mint.txt").toString();
let tokenMint = new anchor.web3.PublicKey(tokenMintAddress);

const initializeToken = async (): Promise<anchor.web3.Keypair> => {
  const tokenMint = new anchor.web3.Keypair();
  const lamportsForMint =
    await provider.connection.getMinimumBalanceForRentExemption(
      spl.MintLayout.span
    );
  let tx = new anchor.web3.Transaction();

  // Allocate mint
  tx.add(
    anchor.web3.SystemProgram.createAccount({
      programId: spl.TOKEN_PROGRAM_ID,
      space: spl.MintLayout.span,
      fromPubkey: KEYPAIR.publicKey,
      newAccountPubkey: tokenMint.publicKey,
      lamports: lamportsForMint,
    })
  );
  // Allocate wallet account
  tx.add(
    spl.createInitializeMintInstruction(
      tokenMint.publicKey,
      6,
      KEYPAIR.publicKey,
      KEYPAIR.publicKey,
      spl.TOKEN_PROGRAM_ID
    )
  );
  await provider.sendAndConfirm(tx, [tokenMint]);
  return tokenMint;
};

const init_mint = async () => {
  let tokenMint = await initializeToken();
  console.log("tokenMint is ", tokenMint.publicKey.toBase58());
  fs.writeFileSync("StaticAddress/mint.txt", tokenMint.publicKey.toBase58());
};

const createUserAssociatedWalletAndMint = async (
  address: anchor.web3.PublicKey,
  mint?: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey | undefined> => {
  let userAssociatedTokenAccount: anchor.web3.PublicKey | undefined = undefined;
  if (mint) {
    // Create a token account for the address and mint some tokens
    userAssociatedTokenAccount = await spl.getAssociatedTokenAddress(
      mint,
      address,
      true,
      spl.TOKEN_PROGRAM_ID,
      spl.ASSOCIATED_TOKEN_PROGRAM_ID
    );
    console.log(
      "userAssociatedTokenAccount is ",
      userAssociatedTokenAccount.toBase58()
    );

    const txFundTokenAccount = new anchor.web3.Transaction();
    txFundTokenAccount.add(
      spl.createAssociatedTokenAccountInstruction(
        KEYPAIR.publicKey,
        userAssociatedTokenAccount,
        address,
        mint,
        spl.TOKEN_PROGRAM_ID,
        spl.ASSOCIATED_TOKEN_PROGRAM_ID
      )
    );
    // txFundTokenAccount.add(
    //   spl.createMintToInstruction(
    //     mint,
    //     userAssociatedTokenAccount,
    //     KEYPAIR.publicKey,
    //     5000000000,
    //     [],
    //     spl.TOKEN_PROGRAM_ID
    //   )
    // );
    try {
      const txFundTokenSig = await provider.sendAndConfirm(txFundTokenAccount, [
        KEYPAIR,
      ]);
    } catch (error) {
      console.log(error);
    }
  }
  return userAssociatedTokenAccount;
};

const fundWallet = async (user: anchor.web3.PublicKey, amount: number) => {
  console.log(KEYPAIR.publicKey.toString());
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

const readInfo = async () => {
  const tokenInfo = await provider.connection.getAccountInfo(tokenMint);
  const data = Buffer.from(tokenInfo.data);
  const accountInfo = spl.MintLayout.decode(data);
  console.log(accountInfo);
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

const config_acc = async () => {
  const [config, bumps] = await PublicKey.findProgramAddress(
    [Buffer.from("config")],
    program.programId
  );

  const configData = await program.account.config.fetch(config);
  console.log("Owner", configData.owner.toBase58());
  fs.writeFileSync("StaticAddress/config.txt", config.toBase58());

  return config;
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

const init_pda_ata = async () => {
  chainId = Buffer.from(CHAIN_ID_BSC.toString());

  depositorHash = tryNativeToUint8Array(
    "0x30Fbf353f4f7C37952e22a9709e04b7541D5A77F",
    CHAIN_ID_BSC
  );
  receiverHash = tryNativeToUint8Array(
    "0x30ca5c53ff960f16180aada7c38ab2572a597676",
    CHAIN_ID_BSC
  );

  console.log(depositorHash);
  console.log(chainId);

  // const [pdaSignerTemp, nonce] = await anchor.web3.PublicKey.findProgramAddress(
  //   [depositorHash, chainId],
  //   program.programId
  // );
  const [pdaSender, nonce] = anchor.web3.PublicKey.findProgramAddressSync(
    [depositorHash, Buffer.from(CHAIN_ID_BSC.toString())],
    program.programId
  );
  console.log("PDA Signer: " + pdaSender);

  let pdaSigner = pdaSender;

  const [pdaReciverTemp] = await anchor.web3.PublicKey.findProgramAddress(
    [receiverHash, chainId],
    program.programId
  );
  let pdaReceiver = pdaReciverTemp;

  console.log("Pda receiver", pdaReceiver.toBase58());
  console.log("Pda signer", pdaSigner.toBase58());

  let signerATA = await createUserAssociatedWalletAndMint(pdaSigner, tokenMint);

  let receiverATA = await spl.getAssociatedTokenAddress(
    tokenMint,
    pdaReceiver,
    true,
    spl.TOKEN_PROGRAM_ID,
    spl.ASSOCIATED_TOKEN_PROGRAM_ID
  );

  let [zebecVault] = await anchor.web3.PublicKey.findProgramAddress(
    [pdaSigner.toBuffer()],
    zebecProgram.programId
  );

  let zebecVaultATA = await spl.getAssociatedTokenAddress(
    tokenMint,
    zebecVault,
    true,
    spl.TOKEN_PROGRAM_ID,
    spl.ASSOCIATED_TOKEN_PROGRAM_ID
  );

  await fundWallet(pdaSigner, 2);
  await fundWallet(pdaReciverTemp, 2);

  fs.writeFileSync("StaticAddress/pdaSender.txt", pdaSigner.toBase58());
  fs.writeFileSync("StaticAddress/pdaSenderATA.txt", signerATA.toBase58());
  fs.writeFileSync("StaticAddress/pdaReceiver.txt", pdaReceiver.toBase58());
  fs.writeFileSync("StaticAddress/pdaReceiverATA.txt", receiverATA.toBase58());
  fs.writeFileSync("StaticAddress/zebecVault.txt", zebecVault.toBase58());
  fs.writeFileSync("StaticAddress/zebecVaultATA.txt", zebecVaultATA.toBase58());

  let withdrawDatatemp = await withdrawData(
    "withdraw_token",
    pdaSigner,
    zebecProgram.programId,
    tokenMint
  );
  fs.writeFileSync(
    "StaticAddress/withdrawData.txt",
    withdrawDatatemp.toBase58()
  );
};

const init_fee_vault_ata = async () => {
  let fee_vault = new anchor.web3.PublicKey(
    fs.readFileSync("StaticAddress/feeVault.txt").toString()
  );

  let feeVaultPDAATA = await spl.getAssociatedTokenAddress(
    tokenMint,
    fee_vault,
    true,
    spl.TOKEN_PROGRAM_ID,
    spl.ASSOCIATED_TOKEN_PROGRAM_ID
  );

  fs.writeFileSync("StaticAddress/feeVaultATA.txt", feeVaultPDAATA.toBase58());
};

const init_fee_account_zebec = async () => {
  // await fundWallet(fee_receiver.publicKey, 2);

  const [fee_vault, __] = await anchor.web3.PublicKey.findProgramAddress(
    [
      fee_receiver.publicKey.toBuffer(),
      anchor.utils.bytes.utf8.encode(OPERATE),
    ],
    zebecProgram.programId
  );

  const [vault_data, _] = await anchor.web3.PublicKey.findProgramAddress(
    [
      fee_receiver.publicKey.toBuffer(),
      anchor.utils.bytes.utf8.encode(OPERATEDATA),
      fee_vault.toBuffer(),
    ],
    zebecProgram.programId
  );

  fs.writeFileSync(
    "StaticAddress/feeReceiver.txt",
    fee_receiver.publicKey.toBase58()
  );
  fs.writeFileSync("StaticAddress/feeVault.txt", fee_vault.toBase58());
  fs.writeFileSync("StaticAddress/vaultData.txt", vault_data.toBase58());
};

const initZEBECFeeVault = async () => {
  const fee_percentage = new anchor.BN(25);

  let fee_vault = new anchor.web3.PublicKey(
    fs.readFileSync("StaticAddress/feeVault.txt").toString()
  );

  let vault_data = new anchor.web3.PublicKey(
    fs.readFileSync("StaticAddress/vaultData.txt").toString()
  );

  console.log("Fee Vault");
  await zebecProgram.rpc.createFeeAccount(fee_percentage, {
    accounts: {
      feeVault: fee_vault,
      feeVaultData: vault_data,
      feeOwner: fee_receiver.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    },
    signers: [fee_receiver],
  });
};

const doTheThing = async () => {
  // await init_mint();
  await init_pda_ata();
  // await readInfo();
  await init_fee_account_zebec();
  // await initZEBECFeeVault();
  await init_fee_vault_ata();

  // await config_acc();
};
doTheThing();
