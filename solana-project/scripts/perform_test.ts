import fs from 'fs';
import * as anchor from '@project-serum/anchor';
import * as spl from '@solana/spl-token';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import * as bip39 from 'bip39';
import { Keypair, PublicKey, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { derivePath } from 'ed25519-hd-key';
import { keccak_256 } from 'js-sha3';
import { SolanaProject as Messenger } from '../target/types/solana_project';
import { Zebec } from '../target/types/zebec';
import {
  setDefaultWasm,
  postVaaSolanaWithRetry,
  importCoreWasm,
  getClaimAddressSolana,
  getEmitterAddressEth,
} from '@certusone/wormhole-sdk';
import { findProgramAddressSync } from '@project-serum/anchor/dist/cjs/utils/pubkey';
import * as b from 'byteify';
import keccak256 from 'keccak256';

const SOLANA_CORE_BRIDGE_ADDRESS =
  '3u8hJUVTA4jH1wYAyUur7FFZVQ8H635K3tSHHF4ssjQ5';
const OPERATE = 'NewVaultOption';
const OPERATEDATA = 'NewVaultOptionData';
const CONN_STRING = 'https://api.devnet.solana.com';
let connection = new anchor.web3.Connection(CONN_STRING);

// EOA Wallet
const KEYPAIR = anchor.web3.Keypair.fromSecretKey(
  Uint8Array.from(JSON.parse(fs.readFileSync('hello.json').toString()))
);
let provider = new anchor.AnchorProvider(
  connection,
  new NodeWallet(KEYPAIR),
  {}
);

const CONTRACT_ADDRESS = '3DwkDP1FrPSgvM2hXJ6PSwP3rm4nq54HX16gU41ckWGd';
const IDL = JSON.parse(
  fs.readFileSync('target/idl/solana_project.json').toString()
);
const program = new anchor.Program<Messenger>(IDL, CONTRACT_ADDRESS, provider);

const ZEBEC_PROGRAM_ID = new anchor.web3.PublicKey(
  'dSuyjPvmWdBr68FRG9Q433Py6YxeiTMZni7WiF74GQE'
);
const IDL2 = JSON.parse(fs.readFileSync('target/idl/zebec.json').toString());
const zebecProgram = new anchor.Program<Zebec>(
  IDL2,
  ZEBEC_PROGRAM_ID,
  provider
);

let transaction: anchor.web3.Keypair;
let depositorHash = Buffer.from(
  keccak_256('0xB0e53390e4697e65d6c2ed5213e49b8390da9853'),
  'hex'
);

let chainIdHash = Buffer.from(keccak_256('4'), 'hex');
let receiverHash;

let feeReceiver = new anchor.web3.PublicKey(
  fs.readFileSync('StaticAddress/feeReceiver.txt').toString()
);
let tokenMint = new anchor.web3.PublicKey(
  fs.readFileSync('StaticAddress/mint.txt').toString()
);
let pdaSender = new anchor.web3.PublicKey(
  fs.readFileSync('StaticAddress/pdaSender.txt').toString()
);

let pdaSenderATA = new anchor.web3.PublicKey(
  fs.readFileSync('StaticAddress/pdaSenderATA.txt').toString()
);
let zebecVault = new anchor.web3.PublicKey(
  fs.readFileSync('StaticAddress/zebecVault.txt').toString()
);
let zebecVaultATA = new anchor.web3.PublicKey(
  fs.readFileSync('StaticAddress/zebecVaultATA.txt').toString()
);
let pdaReceiver = new anchor.web3.PublicKey(
  fs.readFileSync('StaticAddress/pdaReceiver.txt').toString()
);
let feeVault = new anchor.web3.PublicKey(
  fs.readFileSync('StaticAddress/feeVault.txt').toString()
);
let vaultData = new anchor.web3.PublicKey(
  fs.readFileSync('StaticAddress/vaultData.txt').toString()
);
let dataAccount = new anchor.web3.PublicKey(
  fs.readFileSync('StaticAddress/dataAccount.txt').toString()
);
let feeReceiverTokenAccount = new anchor.web3.PublicKey(
  fs.readFileSync('StaticAddress/feeVaultATA.txt').toString()
);
let pdaReceiverATA = new anchor.web3.PublicKey(
  fs.readFileSync('StaticAddress/pdaReceiverATA.txt').toString()
);

let withdrawData = new anchor.web3.PublicKey(
  fs.readFileSync('StaticAddress/withdrawData.txt').toString()
);

const PREFIX_TOKEN = 'withdraw_token';
const STREAM_TOKEN_SIZE =
  8 + 8 + 8 + 8 + 8 + 8 + 32 + 32 + 32 + 8 + 8 + 32 + 8 + 1 + 1;

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

const getDataStoragePda = async (
  sender: Buffer,
  currentCount: anchor.BN
): Promise<PublicKey> => {
  //PDA
  let [dataStorage, bump] = await PublicKey.findProgramAddress(
    [Buffer.from('data_store'), sender, currentCount.toArrayLike(Buffer)],
    program.programId
  );
  return dataStorage;
};

const getTxnCountPda = async (sender: Buffer): Promise<PublicKey> => {
  //PDA
  let [count, bump] = await PublicKey.findProgramAddress(
    [Buffer.from('txn_count'), sender],
    program.programId
  );

  return count;
};

const submit_vaa = async () => {
  setDefaultWasm('node');

  const { parse_vaa } = await importCoreWasm();
  const vaa = fs.readFileSync('../evm-project/vaa.txt').toString();
  const vaaBytes = Buffer.from(vaa, 'base64');

  //Submit to Core Bridge
  await postVaaSolanaWithRetry(
    new anchor.web3.Connection(CONN_STRING, 'confirmed'),
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
      Buffer.from('EmitterAddress'),
      b.serializeUint16(parsed_vaa.emitter_chain),
    ],
    program.programId
  )[0];

  let processed_vaa_key = findProgramAddressSync(
    [
      Buffer.from(
        getEmitterAddressEth(
          fs.readFileSync('../evm-project/eth-address.txt').toString()
        ),
        'hex'
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
    [Buffer.from('PostedVAA'), hash],
    new anchor.web3.PublicKey(SOLANA_CORE_BRIDGE_ADDRESS)
  )[0];
  console.log('Core Bridge VAA Key: ', core_bridge_vaa_key.toString());

  //Current Depositor Data Storage PDA
  let currentCount = 0;
  let dataStorage = await getDataStoragePda(
    depositorHash,
    new anchor.BN(currentCount)
  );

  // //Count Storage PDA
  let count = await getTxnCountPda(depositorHash);
  console.log('Count', count);
  const tx = await program.methods
    .storeMsg(currentCount, depositorHash)
    .accounts({
      payer: KEYPAIR.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
      processedVaa: processed_vaa_key,
      emitterAcc: emitter_address_acc,
      coreBridgeVaa: core_bridge_vaa_key,
      dataStorage: dataStorage,
      txnCount: count,
    })
    .signers([KEYPAIR])
    .rpc();

  const hashData = await program.account.transactionData.fetch(dataStorage);
  console.log('Hash Txn Data', hashData.transactionHash);
};

const deposit = async () => {
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
  const amount = new anchor.BN(10000000);
  const data = zebecProgram.coder.instruction.encode('depositToken', {
    amount: amount,
  });

  const txSize = 450;

  let transactionHash = Buffer.from('Test_Hash');

  const tx = await program.rpc.createTransaction(
    zebecProgram.programId,
    accounts,
    data,
    new anchor.BN(0),

    depositorHash,
    transactionHash,
    {
      accounts: {
        transaction: transaction.publicKey,
        zebecEoa: KEYPAIR.publicKey,
        dataStorage: await getDataStoragePda(depositorHash, new anchor.BN(0)),
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

  const exeTxn = await program.rpc.executeTransaction(
    chainIdHash,
    depositorHash,
    {
      accounts: {
        pdaSigner: pdaSender,
        transaction: transaction.publicKey,
      },
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
  console.log(tokenbalance.toString());
};

const stream = async () => {
  let dataAccount = new anchor.web3.Keypair();

  fs.writeFileSync(
    'StaticAddress/dataAccount.txt',
    dataAccount.publicKey.toBase58()
  );

  let withdrawDatatemp = await withdrawData(
    PREFIX_TOKEN,
    pdaSender,
    zebecProgram.programId,
    tokenMint
  );
  fs.writeFileSync(
    'StaticAddress/withdrawData.txt',
    withdrawDatatemp.toBase58()
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
      pubkey: feeReceiver,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: vaultData,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: feeVault,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: pdaSender,
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

  const data = zebecProgram.coder.instruction.encode('tokenStream', {
    startTime: startTime,
    endTime: endTime,
    amount: amount,
    canCancel,
    canUpdate,
  });

  const txSize = 700;
  const transaction = anchor.web3.Keypair.generate();

  let transactionHash = Buffer.from('Test_Hash');

  const tx = await program.rpc.createTransaction(
    zebecProgram.programId,
    accounts,
    data,
    new anchor.BN(1),

    depositorHash,
    transactionHash,
    {
      accounts: {
        transaction: transaction.publicKey,
        zebecEoa: KEYPAIR.publicKey,
        dataStorage: await getDataStoragePda(depositorHash, new anchor.BN(1)),
        systemProgram: anchor.web3.SystemProgram.programId,
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
      signers: [transaction, KEYPAIR, dataAccount],
    }
  );

  const exeTxn = await program.rpc.executeTransaction(
    chainIdHash,
    depositorHash,
    {
      accounts: {
        pdaSigner: pdaSender,
        transaction: transaction.publicKey,
      },
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

  const data_account = await zebecProgram.account.streamToken.fetch(
    dataAccount.publicKey
  );

  const withdraw_info = await zebecProgram.account.tokenWithdraw.fetch(
    withdrawDatatemp
  );
};

const update_stream = async () => {
  let withdrawDatatemp = await withdrawData(
    PREFIX_TOKEN,
    pdaSender,
    zebecProgram.programId,
    tokenMint
  );

  const accounts = [
    {
      pubkey: dataAccount,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: withdrawDatatemp,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: pdaSender,
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
    },
  ];

  let now = Math.floor(new Date().getTime() / 1000);

  const startTime = new anchor.BN(now);
  const endTime = new anchor.BN(now + 2000);
  const amount = new anchor.BN(1000000);

  const data = zebecProgram.coder.instruction.encode('tokenStreamUpdate', {
    startTime: startTime,
    endTime: endTime,
    amount: amount,
  });

  const txSize = 700;
  const transaction = anchor.web3.Keypair.generate();

  let transactionHash = Buffer.from('Test_Hash');

  const tx = await program.rpc.createTransaction(
    zebecProgram.programId,
    accounts,
    data,
    new anchor.BN(2),

    depositorHash,
    transactionHash,
    {
      accounts: {
        transaction: transaction.publicKey,
        zebecEoa: KEYPAIR.publicKey,
        dataStorage: await getDataStoragePda(depositorHash, new anchor.BN(2)),
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

  const exeTxn = await program.rpc.executeTransaction(
    chainIdHash,
    depositorHash,
    {
      accounts: {
        pdaSigner: pdaSender,
        transaction: transaction.publicKey,
      },
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
};

const pause_stream = async () => {
  const accounts = [
    {
      pubkey: pdaSender,
      isWritable: true,
      isSigner: true,
    },
    {
      pubkey: pdaReceiver,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: dataAccount,
      isWritable: true,
      isSigner: false,
    },
  ];

  const data = zebecProgram.coder.instruction.encode(
    'pauseResumeTokenStream',
    {}
  );

  const txSize = 700;
  const transaction = anchor.web3.Keypair.generate();

  let transactionHash = Buffer.from('Test_Hash');

  const tx = await program.rpc.createTransaction(
    zebecProgram.programId,
    accounts,
    data,
    new anchor.BN(3),

    depositorHash,
    transactionHash,
    {
      accounts: {
        transaction: transaction.publicKey,
        zebecEoa: KEYPAIR.publicKey,
        dataStorage: await getDataStoragePda(depositorHash, new anchor.BN(3)),
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

  const exeTxn = await program.rpc.executeTransaction(
    chainIdHash,
    depositorHash,
    {
      accounts: {
        pdaSigner: pdaSender,
        transaction: transaction.publicKey,
      },
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
};

const cancel_stream = async () => {
  let withdrawDatatemp = await withdrawData(
    PREFIX_TOKEN,
    pdaSender,
    zebecProgram.programId,
    tokenMint
  );

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
      pubkey: pdaSender,
      isWritable: true,
      isSigner: true,
    },
    {
      pubkey: feeReceiver,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: vaultData,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: feeVault,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: dataAccount,
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
      pubkey: pdaReceiverATA,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: feeReceiverTokenAccount,
      isWritable: true,
      isSigner: false,
    },
  ];

  const data = zebecProgram.coder.instruction.encode('cancelTokenStream', {});

  const txSize = 700;
  const transaction = anchor.web3.Keypair.generate();

  let transactionHash = Buffer.from('Test_Hash');

  const tx = await program.rpc.createTransaction(
    zebecProgram.programId,
    accounts,
    data,
    new anchor.BN(4),

    depositorHash,
    transactionHash,
    {
      accounts: {
        transaction: transaction.publicKey,
        zebecEoa: KEYPAIR.publicKey,
        dataStorage: await getDataStoragePda(depositorHash, new anchor.BN(4)),
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

  const exeTxn = await program.rpc.executeTransaction(
    chainIdHash,
    depositorHash,
    {
      accounts: {
        pdaSigner: pdaSender,
        transaction: transaction.publicKey,
      },
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
};

const instant_transfer = async () => {
  let withdrawDatatemp = await withdrawData(
    PREFIX_TOKEN,
    pdaSender,
    zebecProgram.programId,
    tokenMint
  );

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
      pubkey: pdaSender,
      isWritable: true,
      isSigner: true,
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
      pubkey: pdaReceiverATA,
      isWritable: true,
      isSigner: false,
    },
  ];

  const amount = new anchor.BN(10000000);

  const data = zebecProgram.coder.instruction.encode('instantTokenTransfer', {
    amount: amount,
  });

  const txSize = 700;
  const transaction = anchor.web3.Keypair.generate();

  let transactionHash = Buffer.from('Test_Hash');

  const tx = await program.rpc.createTransaction(
    zebecProgram.programId,
    accounts,
    data,
    new anchor.BN(5),

    depositorHash,
    transactionHash,
    {
      accounts: {
        transaction: transaction.publicKey,
        zebecEoa: KEYPAIR.publicKey,
        dataStorage: await getDataStoragePda(depositorHash, new anchor.BN(5)),
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

  const exeTxn = await program.rpc.executeTransaction(
    chainIdHash,
    depositorHash,
    {
      accounts: {
        pdaSigner: pdaSender,
        transaction: transaction.publicKey,
      },
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
};

const token_withdrawl = async () => {
  const accounts = [
    {
      pubkey: zebecVault,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: withdrawData,
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
  const txSize = 700;
  const amount = new anchor.BN(10000000);
  const data = zebecProgram.coder.instruction.encode('tokenWithdrawal', {
    amount: amount,
  });
  depositorHash = Buffer.from(
    keccak_256('0xB0e53390e4697e65d6c2ed5213e49b8390da9853'),
    'hex'
  );
  chainIdHash = Buffer.from(keccak_256('4'), 'hex');

  let transactionHash = Buffer.from('Test_Hash');

  const tx = await program.rpc.createTransaction(
    zebecProgram.programId,
    accounts,
    data,
    new anchor.BN(6),

    depositorHash,
    transactionHash,
    {
      accounts: {
        transaction: transaction.publicKey,
        zebecEoa: KEYPAIR.publicKey,
        dataStorage: await getDataStoragePda(depositorHash, new anchor.BN(6)),
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

  const exeTxn = await program.rpc.executeTransaction(
    chainIdHash,
    depositorHash,
    {
      accounts: {
        pdaSigner: pdaSender,
        transaction: transaction.publicKey,
      },
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
};

const token_receiver_withdrawl = async () => {
  let feeTokenAccountATA = await spl.getAssociatedTokenAddress(
    tokenMint,
    feeVault,
    true,
    spl.TOKEN_PROGRAM_ID,
    spl.ASSOCIATED_TOKEN_PROGRAM_ID
  );
  fs.writeFileSync(
    'StaticAddress/feeAccountATA.txt',
    feeTokenAccountATA.toBase58()
  );

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
      pubkey: pdaSender,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: feeReceiver,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: vaultData,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: feeVault,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: dataAccount,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: withdrawData,
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
      pubkey: pdaReceiverATA,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: feeTokenAccountATA,
      isWritable: true,
      isSigner: false,
    },
  ];

  const data = zebecProgram.coder.instruction.encode('withdrawTokenStream', {});
  const txSize = 700;
  const transaction = anchor.web3.Keypair.generate();
  receiverHash = Buffer.from(
    keccak_256('0x30ca5c53ff960f16180aada7c38ab2572a597676'),
    'hex'
  );
  chainIdHash = Buffer.from(keccak_256('4'), 'hex');

  let transactionHash = Buffer.from('Test_Hash');

  const tx = await program.rpc.createTransaction(
    zebecProgram.programId,
    accounts,
    data,
    new anchor.BN(7),

    depositorHash,
    transactionHash,
    {
      accounts: {
        transaction: transaction.publicKey,
        zebecEoa: KEYPAIR.publicKey,
        dataStorage: await getDataStoragePda(depositorHash, new anchor.BN(7)),
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

  const exeTxn = await program.rpc.executeTransaction(
    chainIdHash,
    receiverHash,
    {
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
    }
  );
};

const doTest = async () => {
  await submit_vaa();

  //Execute One By One
  await deposit();
  // await stream();
  // await update_stream();
  // await pause_stream();
  // await pause_stream();
  // await cancel_stream();
  // await instant_transfer();
  // await token_withdrawl();
  // await token_receiver_withdrawl();
};

doTest();
