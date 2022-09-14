import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
import { SolanaProject } from "../target/types/solana_project";
import { keccak_256 } from 'js-sha3';
import { Zebec } from "../target/types/zebec";
import * as spl from "@solana/spl-token";
import fs from "fs";
import * as b from "byteify";
import { assert, expect } from "chai";

import {
  createMint,
  findZebecVault,
  feeVault,
  create_fee_account,
  withdrawData,
  getTokenBalance,
} from "./utils";

import {
  PREFIX_TOKEN,
  STREAM_TOKEN_SIZE,
} from "./Constants";
import { fee_collector_address } from "@certusone/wormhole-sdk/lib/cjs/solana/core/bridge_bg";
import { token } from "@project-serum/anchor/dist/cjs/utils";

let payer = new anchor.web3.Keypair();
describe("solana-project", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaProject as Program<SolanaProject>;
  const zebecProgram = anchor.workspace.Zebec as Program<Zebec>;

  let depositorHash;
  let chainIdHash;
  let receiverHash;
  const feePayer: anchor.web3.PublicKey = provider.wallet.publicKey;
  const fee_receiver: anchor.web3.Keypair = new anchor.web3.Keypair();
  const receiver: anchor.web3.Keypair = anchor.web3.Keypair.generate();

  let zebecEOA: anchor.web3.Keypair = anchor.web3.Keypair.generate();
  let pdaSigner: anchor.web3.PublicKey;
  let pdaReceiver: anchor.web3.PublicKey;
  let tokenMint: anchor.web3.PublicKey;
  let pdaSignerATA: anchor.web3.PublicKey;
  let receiverATA: anchor.web3.PublicKey;
  let zebecVault: anchor.web3.PublicKey;
  let zebecVaultATA: anchor.web3.PublicKey;
  let feeTokenAccountATA: anchor.web3.PublicKey;

  const dataAccount: anchor.web3.Keypair = anchor.web3.Keypair.generate();

  const fundWallet = async (user: anchor.web3.PublicKey, amount: number) => {
    let txFund = new anchor.web3.Transaction();
    txFund.add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: feePayer,
        toPubkey: user,
        lamports: amount * anchor.web3.LAMPORTS_PER_SOL,
      })
    );
    await provider.sendAndConfirm(txFund);
  };

  const initializeNft = async (): Promise<anchor.web3.PublicKey> => {
    const nftMint = new anchor.web3.Keypair();
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
        fromPubkey: feePayer,
        newAccountPubkey: nftMint.publicKey,
        lamports: lamportsForMint,
      })
    );
    // Allocate wallet account
    tx.add(
      spl.createInitializeMintInstruction(
        nftMint.publicKey,
        6,
        feePayer,
        feePayer,
        spl.TOKEN_PROGRAM_ID
      )
    );
    await provider.sendAndConfirm(tx, [nftMint]);
    return nftMint.publicKey;
  };

  const createUserAndAssociatedWallet = async (
    address: anchor.web3.PublicKey,
    mint?: anchor.web3.PublicKey
  ): Promise<anchor.web3.PublicKey | undefined> => {
    
    let userAssociatedTokenAccount: anchor.web3.PublicKey | undefined = undefined;
    // Fund zebecEOA with some SOL
    let txFund = new anchor.web3.Transaction();

    txFund.add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: feePayer,
        toPubkey: zebecEOA.publicKey,
        lamports: 5 * anchor.web3.LAMPORTS_PER_SOL,
      })
    );
    const sigTxFund = await provider.sendAndConfirm(txFund);
    if (mint) {
      // Create a token account for the address and mint some tokens
      userAssociatedTokenAccount = await spl.getAssociatedTokenAddress(
        mint,
        address,
        true,
        spl.TOKEN_PROGRAM_ID,
        spl.ASSOCIATED_TOKEN_PROGRAM_ID
      );
      const txFundTokenAccount = new anchor.web3.Transaction();
      txFundTokenAccount.add(
        spl.createAssociatedTokenAccountInstruction(
          zebecEOA.publicKey,
          userAssociatedTokenAccount,
          address,
          mint,
          spl.TOKEN_PROGRAM_ID,
          spl.ASSOCIATED_TOKEN_PROGRAM_ID
        )
      );
      txFundTokenAccount.add(
        spl.createMintToInstruction(
          mint,
          userAssociatedTokenAccount,
          feePayer,
          1337000000,
          [],
          spl.TOKEN_PROGRAM_ID
        )
      );
      try {
        const txFundTokenSig = await provider.sendAndConfirm(txFundTokenAccount, [zebecEOA]);
      } catch (error) {
        console.log(error);
      }
    }
    return userAssociatedTokenAccount;
  };

  it("PDA is initialized!", async () => {
    let chain_id_string = "4";
    depositorHash = Buffer.from(
      keccak_256('0xB0e53390e4697e65d6c2ed5213e49b8390da9853'),
      'hex'
    );
    chainIdHash = Buffer.from(
      keccak_256(chain_id_string),
      'hex'
    );
    const [pdaSignerTemp, nonce] = await anchor.web3.PublicKey.findProgramAddress(
      [depositorHash, chainIdHash],
      program.programId
    );
    pdaSigner = pdaSignerTemp;

    receiverHash = Buffer.from(
      keccak_256('0x30ca5c53ff960f16180aada7c38ab2572a597676'),
      'hex'
    );
    const [pdaReciverTemp, ] = await anchor.web3.PublicKey.findProgramAddress(
      [receiverHash, chainIdHash],
      program.programId
    );
    pdaReceiver = pdaReciverTemp;

    await fundWallet(payer.publicKey, 2);
    await fundWallet(pdaSignerTemp, 2);
    await fundWallet(pdaReciverTemp, 2);

    // await program.rpc.initilizePda( chainIdHash, depositorHash, {
    //   accounts: {
    //     payer: payer.publicKey,
    //     pdaSigner: pdaSigner,
    //     systemProgram: anchor.web3.SystemProgram.programId,
    //   },
    //   signers: [payer],
    // });


  });

  it("Create Set Vault", async () => {
    const fee_percentage = new anchor.BN(25);

    await fundWallet(fee_receiver.publicKey, 2);

    await zebecProgram.rpc.createFeeAccount(fee_percentage, {
      accounts: {
        feeVault: await feeVault(fee_receiver.publicKey, zebecProgram.programId),
        vaultData: await create_fee_account(fee_receiver.publicKey, zebecProgram.programId),
        owner: fee_receiver.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId      
      },
      signers: [fee_receiver],
    });
  });

  it("Inits SPL and mints", async () => {
    tokenMint = await initializeNft();

    pdaSignerATA = await createUserAndAssociatedWallet(
      pdaSigner,
      tokenMint
    );

    receiverATA = await createUserAndAssociatedWallet(
      pdaReceiver,
      tokenMint
    );
    zebecVault = await findZebecVault(pdaSigner, zebecProgram.programId);

    zebecVaultATA = await spl.getAssociatedTokenAddress(
      tokenMint,
      zebecVault,
      true,
      spl.TOKEN_PROGRAM_ID,
      spl.ASSOCIATED_TOKEN_PROGRAM_ID
    );

    feeTokenAccountATA = await spl.getAssociatedTokenAddress(
      tokenMint,
      await feeVault(fee_receiver.publicKey, zebecProgram.programId),
      true,
      spl.TOKEN_PROGRAM_ID,
      spl.ASSOCIATED_TOKEN_PROGRAM_ID
    );
  });

  it("Deposit token", async () => {
    const accounts = [
      {
        pubkey: zebecVault,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: pdaSigner,
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
        pubkey: pdaSignerATA,
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
    const amount =  new anchor.BN(10000000);
    const data = zebecProgram.coder.instruction.encode("depositToken", {
      amount: amount,
    });
    // const txSize = getTxSize(accounts, owners, false, 8);
    await fundWallet(zebecEOA.publicKey, 5);
    await fundWallet(pdaSigner, 5);
    const txSize = 1000;
    
    const tx = await program.rpc.createTransaction(
    zebecProgram.programId,
    accounts,
    data,
    {
      accounts: {
        transaction: transaction.publicKey,
        zebecEoa: zebecEOA.publicKey,
      },
      instructions: [
        await program.account.transaction.createInstruction(
          transaction,
          txSize
        )
      ],
      signers: [transaction, zebecEOA],
    }
    );

    const exeTxn = await program.rpc.executeTransaction(chainIdHash, depositorHash, {
      accounts: {
        pdaSigner: pdaSigner,
        transaction: transaction.publicKey,
      },
      remainingAccounts: accounts
        .map((t: any) => {
          if (t.pubkey.equals(pdaSigner)) {
            return { ...t, isSigner: false };
          }
          return t;
        })
        .concat({
          pubkey: zebecProgram.programId,
          isWritable: false,
          isSigner: false,
        }),
    });

    const tokenbalance = await getTokenBalance(
      provider.connection,
      zebecVaultATA
    );
    assert.equal(tokenbalance.toString(), amount.toString());      
  });

  it("Streams token", async () => {

    let withdrawDatatemp = await withdrawData(
      PREFIX_TOKEN,
      pdaSigner,
      zebecProgram.programId,
      tokenMint
    );

    let feeAccountTemp = await create_fee_account(fee_receiver.publicKey, zebecProgram.programId);

    let feeVaultTemp = await feeVault(fee_receiver.publicKey, zebecProgram.programId)
   
    const accounts = [
      {
        pubkey: dataAccount.publicKey,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: withdrawDatatemp,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: fee_receiver.publicKey,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: feeAccountTemp,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: feeVaultTemp,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: pdaSigner,
        isWritable: true,
        isSigner: true,
      },
      {
        pubkey: pdaReceiver,
        isWritable: false,
        isSigner: false,
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
        pubkey: tokenMint,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: anchor.web3.SYSVAR_RENT_PUBKEY,
        isWritable: false,
        isSigner: false,
      },
    ];
    let now = Math.floor(new Date().getTime() / 1000);
    const startTime = new anchor.BN(now);
    const endTime = new anchor.BN(now + 2000);
    const dataSize = STREAM_TOKEN_SIZE;
    const amount = new anchor.BN(1000000);
    const canCancel = true;
    const canUpdate = true;
    const data = zebecProgram.coder.instruction.encode("tokenStream", {
      startTime: startTime,
      endTime: endTime,
      amount: amount,
      canCancel,
      canUpdate,
    });
    const txSize = 1232;
    const transaction = anchor.web3.Keypair.generate();

    // console.log(transaction.publicKey.toBase58());
    
    await fundWallet(zebecEOA.publicKey, 5);
    await fundWallet(pdaSigner, 5);
    await fundWallet(fee_receiver.publicKey, 5);

    // 1467 WILL BE THE TXN SIZE IF CONFIRM MSG USED

    const tx = await program.rpc.createTransaction(
      zebecProgram.programId,
      accounts,
      data,
      {
        accounts: {
          transaction: transaction.publicKey,
          zebecEoa: zebecEOA.publicKey,
        },
        instructions: [
          await program.account.transaction.createInstruction(
            transaction,
            txSize
          ),
          await zebecProgram.account.streamToken.createInstruction(
            dataAccount,
            dataSize
          ),
        ],
        signers: [transaction, zebecEOA, dataAccount],
      }
    );

    const exeTxn = await program.rpc.executeTransaction(chainIdHash, depositorHash, {
      accounts: {
        pdaSigner: pdaSigner,
        transaction: transaction.publicKey,
      },
      remainingAccounts: accounts
        .map((t: any) => {
          if (t.pubkey.equals(pdaSigner)) {
            return { ...t, isSigner: false };
          }
          return t;
        })
        .concat({
          pubkey: zebecProgram.programId,
          isWritable: false,
          isSigner: false,
        }),
    });

    const data_account = await zebecProgram.account.streamToken.fetch(
      dataAccount.publicKey
    );

    assert.equal(data_account.startTime.toString(), startTime.toString());
    assert.equal(data_account.endTime.toString(), endTime.toString());
    assert.equal(data_account.amount.toString(), amount.toString());
    assert.equal(data_account.sender.toString(), pdaSigner.toString());
    assert.equal(
      data_account.receiver.toString(),
      pdaReceiver.toString()
    );
    assert.equal(data_account.paused.toString(), "0");

    const withdraw_info = await zebecProgram.account.tokenWithdraw.fetch(withdrawDatatemp);
    assert.equal(withdraw_info.amount.toString(), amount.toString());

      
  });

  /*
  it("Updates Streams", async () => {

    let withdrawDatatemp = await withdrawData(
      PREFIX_TOKEN,
      pdaSigner,
      zebecProgram.programId,
      tokenMint
    );

    const accounts = [
      {
        pubkey: dataAccount.publicKey,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: withdrawDatatemp,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: pdaSigner,
        isWritable: true,
        isSigner: true,
      },
      {
        pubkey: pdaReceiver,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: tokenMint,
        isWritable: false,
        isSigner: false,
      }
    ];

    let now = Math.floor(new Date().getTime() / 1000);
    let startTime = new anchor.BN(now - 1000);
    let endTime = new anchor.BN(now + 2000);
    const amount = new anchor.BN(1000000);
    const data = zebecProgram.coder.instruction.encode("tokenStreamUpdate", {
      startTime: startTime,
      endTime: endTime,
      amount: amount
    });
    const txSize = 1232;
    const transaction = anchor.web3.Keypair.generate();
    
    await fundWallet(zebecEOA.publicKey, 5);
    await fundWallet(pdaSigner, 5);

    // 1467 WILL BE THE TXN SIZE IF CONFIRM MSG USED
    const tx = await program.rpc.confirmMsg(
      zebecProgram.programId,
      accounts,
      data,
      chainIdHash,
      depositorHash,
      {
        accounts: {
          payer: zebecEOA.publicKey,
          // systemProgram: anchor.web3.SystemProgram.programId,
          // tokenMint: tokenMint,
          // processedVaa: processed_vaa_key,
          // emitterAcc: emitter_address_acc,
          // coreBridgeVaa: ,
          // config: ,
          transaction: transaction.publicKey,
          pdaSigner: pdaSigner,    
        },
        instructions: [
          await program.account.transaction.createInstruction(
            transaction,
            txSize
          ),
        ],
        signers: [transaction, zebecEOA],
        remainingAccounts: accounts
        .map((t: any) => {
          if (t.pubkey.equals(pdaSigner)) {
            return { ...t, isSigner: false };
          }
          return t;
        })
        .concat({
          pubkey: zebecProgram.programId,
          isWritable: false,
          isSigner: false,
        })
    });

    const data_account = await zebecProgram.account.streamToken.fetch(
      dataAccount.publicKey
    );

    assert.equal(data_account.startTime.toString(), startTime.toString());
    assert.equal(data_account.endTime.toString(), endTime.toString());
    assert.equal(data_account.amount.toString(), amount.toString());
    assert.equal(data_account.sender.toString(), pdaSigner.toString());
    assert.equal(
      data_account.receiver.toString(),
      pdaReceiver.toString()
    );
    assert.equal(data_account.paused.toString(), "0");

    const withdraw_info = await zebecProgram.account.tokenWithdraw.fetch(withdrawDatatemp);
    assert.equal(withdraw_info.amount.toString(), amount.toString());
  });
  
  it("Pause token stream from multisig", async () => {
    let withdrawDatatemp = await withdrawData(
      PREFIX_TOKEN,
      pdaSigner,
      zebecProgram.programId,
      tokenMint
    );

    const accounts = [
      {
        pubkey: pdaSigner,
        isWritable: true,
        isSigner: true,
      },
      {
        pubkey: pdaReceiver,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: dataAccount.publicKey,
        isWritable: true,
        isSigner: false,
      },
    ];
    const transaction = anchor.web3.Keypair.generate();
    const data = zebecProgram.coder.instruction.encode(
      "pauseResumeTokenStream",
      {}
    );
    const txSize = 1200;
    const tx = await program.rpc.confirmMsg(
      zebecProgram.programId,
      accounts,
      data,
      chainIdHash,
      depositorHash,
      {
        accounts: {
          payer: zebecEOA.publicKey,
          // systemProgram: anchor.web3.SystemProgram.programId,
          // tokenMint: tokenMint,
          // processedVaa: processed_vaa_key,
          // emitterAcc: emitter_address_acc,
          // coreBridgeVaa: ,
          // config: ,
          transaction: transaction.publicKey,
          pdaSigner: pdaSigner,    
        },
        instructions: [
          await program.account.transaction.createInstruction(
            transaction,
            txSize
          ),
        ],
        signers: [transaction, zebecEOA],
        remainingAccounts: accounts
        .map((t: any) => {
          if (t.pubkey.equals(pdaSigner)) {
            return { ...t, isSigner: false };
          }
          return t;
        })
        .concat({
          pubkey: zebecProgram.programId,
          isWritable: false,
          isSigner: false,
        })
    });

    const data_account = await zebecProgram.account.streamToken.fetch(
      dataAccount.publicKey
    );
    assert.equal(data_account.paused.toString(), "1");
  });

  it("Resume token stream from multisig", async () => {
    let withdrawDatatemp = await withdrawData(
      PREFIX_TOKEN,
      pdaSigner,
      zebecProgram.programId,
      tokenMint
    );

    const accounts = [
      {
        pubkey: pdaSigner,
        isWritable: true,
        isSigner: true,
      },
      {
        pubkey: pdaReceiver,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: dataAccount.publicKey,
        isWritable: true,
        isSigner: false,
      },
    ];
    const transaction = anchor.web3.Keypair.generate();
    const data = zebecProgram.coder.instruction.encode(
      "pauseResumeTokenStream",
      {}
    );
    const txSize = 1200;
    const tx = await program.rpc.confirmMsg(
      zebecProgram.programId,
      accounts,
      data,
      chainIdHash,
      depositorHash,
      {
        accounts: {
          payer: zebecEOA.publicKey,
          // systemProgram: anchor.web3.SystemProgram.programId,
          // tokenMint: tokenMint,
          // processedVaa: processed_vaa_key,
          // emitterAcc: emitter_address_acc,
          // coreBridgeVaa: ,
          // config: ,
          transaction: transaction.publicKey,
          pdaSigner: pdaSigner,    
        },
        instructions: [
          await program.account.transaction.createInstruction(
            transaction,
            txSize
          ),
        ],
        signers: [transaction, zebecEOA],
        remainingAccounts: accounts
        .map((t: any) => {
          if (t.pubkey.equals(pdaSigner)) {
            return { ...t, isSigner: false };
          }
          return t;
        })
        .concat({
          pubkey: zebecProgram.programId,
          isWritable: false,
          isSigner: false,
        })
    });

    const data_account = await zebecProgram.account.streamToken.fetch(
      dataAccount.publicKey
    );
    assert.equal(data_account.paused.toString(), "0");
  });
  

  it("Receiver Withdrawal Token", async () => {

    let withdrawDatatemp = await withdrawData(
      PREFIX_TOKEN,
      pdaSigner,
      zebecProgram.programId,
      tokenMint
    );

    let feeAccountTemp = await create_fee_account(fee_receiver.publicKey, zebecProgram.programId);

    let feeVaultTemp = await feeVault(fee_receiver.publicKey, zebecProgram.programId)

    const accounts = [
      {
        pubkey: zebecVault,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: pdaReceiver,
        isWritable: true,
        isSigner: true,
      },
      {
        pubkey: pdaSigner,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: fee_receiver.publicKey,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: feeAccountTemp,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: feeVaultTemp,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: dataAccount.publicKey,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: withdrawDatatemp,
        isWritable: true,
        isSigner: false,
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
        pubkey: zebecVaultATA,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: receiverATA,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: feeTokenAccountATA,
        isWritable: true,
        isSigner: false,
      },
    ];

    let now = Math.floor(new Date().getTime() / 1000);
    let startTime = new anchor.BN(now - 1000);
    let endTime = new anchor.BN(now + 2000);
    const amount = new anchor.BN(1000000);
    const data = zebecProgram.coder.instruction.encode("withdrawTokenStream", {});
    const txSize = 1232;
    const transaction = anchor.web3.Keypair.generate();
    
    await fundWallet(zebecEOA.publicKey, 5);
    await fundWallet(pdaSigner, 5);

    const tx = await program.rpc.createTransaction(
      zebecProgram.programId,
      accounts,
      data,
      {
        accounts: {
          transaction: transaction.publicKey,
          zebecEoa: zebecEOA.publicKey,
        },
        instructions: [
          await program.account.transaction.createInstruction(
            transaction,
            txSize
          ),
        ],
        signers: [transaction, zebecEOA],
      }
    );

    const tokenbalanceBefore = await getTokenBalance(
      provider.connection,
      receiverATA
    );

    const exeTxn = await program.rpc.executeTransaction(chainIdHash, receiverHash, {
      accounts: {
        pdaSigner: pdaReceiver,
        transaction: transaction.publicKey,
      },
      remainingAccounts: accounts
        .map((t: any) => {
          if (t.pubkey.equals(pdaReceiver)) {
            return { ...t, isSigner: false };
          }
          return t;
        })
        .concat({
          pubkey: zebecProgram.programId,
          isWritable: false,
          isSigner: false,
        }),
    });

    const tokenbalanceAfter = await getTokenBalance(
      provider.connection,
      receiverATA
    );
    // assert.equal(tokenbalance.toString(), amount.toString()); 
    expect(tokenbalanceBefore < tokenbalanceAfter);
        
  });

  
  it("Cancel token stream from multisig", async () => {

    const accounts = [
      {
        pubkey: zebecVault,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: pdaReceiver,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: pdaSigner,
        isWritable: true,
        isSigner: true,
      },
      {
        pubkey: fee_receiver.publicKey,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: await create_fee_account(fee_receiver.publicKey, zebecProgram.programId),
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: await feeVault(fee_receiver.publicKey, zebecProgram.programId),
        isWritable: false,
        isSigner: false,
      },

      {
        pubkey: dataAccount.publicKey,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: await withdrawData(
          PREFIX_TOKEN,
          pdaSigner,
          zebecProgram.programId,
          tokenMint
        ),
        isWritable: true,
        isSigner: false,
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
        pubkey: zebecVaultATA,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: receiverATA,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: feeTokenAccountATA,
        isWritable: true,
        isSigner: false,
      },
    ];
    const transaction = anchor.web3.Keypair.generate();
    const data = zebecProgram.coder.instruction.encode("cancelTokenStream", {});
    const txSize = 1200;

    const tx = await program.rpc.createTransaction(
      zebecProgram.programId,
      accounts,
      data,
      {
        accounts: {
          transaction: transaction.publicKey,
          zebecEoa: zebecEOA.publicKey,
        },
        instructions: [
          await program.account.transaction.createInstruction(
            transaction,
            txSize
          ),
        ],
        signers: [transaction, zebecEOA],
      }
    );

    const exeTxn = await program.rpc.executeTransaction(chainIdHash, depositorHash, {
      accounts: {
        pdaSigner: pdaSigner,
        transaction: transaction.publicKey,
      },
      remainingAccounts: accounts
        .map((t: any) => {
          if (t.pubkey.equals(pdaSigner)) {
            return { ...t, isSigner: false };
          }
          return t;
        })
        .concat({
          pubkey: zebecProgram.programId,
          isWritable: false,
          isSigner: false,
        }),
    });
  });

  
  it("Instant Transfer from multisig", async () => {
   
    const accounts = [
      {
        pubkey: zebecVault,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: pdaReceiver,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: pdaSigner,
        isWritable: true,
        isSigner: true,
      },
      {
        pubkey: await withdrawData(
          PREFIX_TOKEN,
          pdaSigner,
          zebecProgram.programId,
          tokenMint
        ),
        isWritable: true,
        isSigner: false,
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
        pubkey: zebecVaultATA,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: receiverATA,
        isWritable: true,
        isSigner: false,
      },
    ];
    const transaction = anchor.web3.Keypair.generate();
    // const txSize = getTxSize(accounts, owners, false, 8);
    const txSize = 1200;
    const data = zebecProgram.coder.instruction.encode("instantTokenTransfer", {
      amount: new anchor.BN(100),
    });

    const tx = await program.rpc.createTransaction(
      zebecProgram.programId,
      accounts,
      data,
      {
        accounts: {
          transaction: transaction.publicKey,
          zebecEoa: zebecEOA.publicKey,
        },
        instructions: [
          await program.account.transaction.createInstruction(
            transaction,
            txSize
          ),
        ],
        signers: [transaction, zebecEOA],
      }
    );

    const exeTxn = await program.rpc.executeTransaction(chainIdHash, depositorHash, {
      accounts: {
        pdaSigner: pdaSigner,
        transaction: transaction.publicKey,
      },
      remainingAccounts: accounts
        .map((t: any) => {
          if (t.pubkey.equals(pdaSigner)) {
            return { ...t, isSigner: false };
          }
          return t;
        })
        .concat({
          pubkey: zebecProgram.programId,
          isWritable: false,
          isSigner: false,
        }),
    });
  });
*/

  it("Withdraw Deposited Token from pdaSigner", async () => {
    const accounts = [
      {
        pubkey: zebecVault,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: await withdrawData(
          PREFIX_TOKEN,
          pdaSigner,
          zebecProgram.programId,
          tokenMint
        ),
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: pdaSigner,
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
        pubkey: pdaSignerATA,
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
    const txSize = 1200;
    const amount =  new anchor.BN(5000000);
    const data = zebecProgram.coder.instruction.encode("tokenWithdrawal", {
      amount: amount,
    });

    const pdaSignerBalanceBefore = await getTokenBalance(
      provider.connection,
      pdaSignerATA
    );

    // TXN too larger 1261> 1232

    fundWallet(pdaSigner, 10);
    const tx = await program.rpc.createTransaction(
      zebecProgram.programId,
      accounts,
      data,
      {
        accounts: {
          transaction: transaction.publicKey,
          zebecEoa: zebecEOA.publicKey,
        },
        instructions: [
          await program.account.transaction.createInstruction(
            transaction,
            txSize
          )
        ],
        signers: [transaction, zebecEOA],
      }
    );

    const exeTxn = await program.rpc.executeTransaction(chainIdHash, depositorHash, {
      accounts: {
        pdaSigner: pdaSigner,
        transaction: transaction.publicKey,
      },
      remainingAccounts: accounts
        .map((t: any) => {
          if (t.pubkey.equals(pdaSigner)) {
            return { ...t, isSigner: false };
          }
          return t;
        })
        .concat({
          pubkey: zebecProgram.programId,
          isWritable: false,
          isSigner: false,
        }),
    });

    const tokenbalance = await getTokenBalance(
      provider.connection,
      zebecVaultATA
    );
    assert.equal(tokenbalance.toString(), amount.toString()); 

    // const pdaSignerBalanceAfter = await getTokenBalance(
    //   provider.connection,
    //   pdaSignerATA
    // );
    // assert.equal(pdaSignerBalanceAfter.toString(), (pdaSignerBalanceBefore + amount).toString()); 
  });
});
