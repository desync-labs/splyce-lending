import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SplyceLending } from "../target/types/splyce_lending";
import {
  TOKEN_PROGRAM_ID,
  createSyncNativeInstruction,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
console.log("TOKEN_PROGRAM_ID:", TOKEN_PROGRAM_ID.toBase58());
console.log("ASSOCIATED_TOKEN_PROGRAM_ID:", ASSOCIATED_TOKEN_PROGRAM_ID.toBase58());
import { Connection, Keypair, PublicKey, SystemProgram, Transaction, LAMPORTS_PER_SOL } from "@solana/web3.js";

import { assert } from "chai";


async function airdropSol(publicKey, amount) {
  let airdropTx = await anchor.getProvider().connection.requestAirdrop(publicKey, amount * anchor.web3.LAMPORTS_PER_SOL);
  await confirmTransaction(airdropTx);
}

async function confirmTransaction(tx) {
  const latestBlockHash = await anchor.getProvider().connection.getLatestBlockhash();
  await anchor.getProvider().connection.confirmTransaction({
    blockhash: latestBlockHash.blockhash,
    lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
    signature: tx,
  });
}

/**
 * Wraps SOL into WSOL by creating an associated token account, transferring lamports, and synchronizing the account.
 *
 * @param provider - The Anchor provider containing the connection and wallet.
 * @param amount - The amount of SOL to wrap into WSOL.
 * @returns The PublicKey of the WSOL associated token account.
 */
async function wrapSOL(provider: anchor.AnchorProvider, amount: number): Promise<PublicKey> {
  const connection = provider.connection;
  const wallet = provider.wallet;

  // WSOL Mint Address (constant for Solana)
  const wrappedSolMint = new PublicKey("So11111111111111111111111111111111111111112");

  // Calculate the amount in lamports
  const lamports = Math.round(amount * LAMPORTS_PER_SOL);
  console.log(`Wrapping ${amount} SOL (${lamports} lamports)`);

  // Derive the associated token account for WSOL
  const wsolTokenAccount = await getAssociatedTokenAddress(
    wrappedSolMint,
    wallet.publicKey,
    false, // Allow owner off curve
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  console.log(`WSOL Associated Token Account: ${wsolTokenAccount.toBase58()}`);

  // Initialize a transaction
  const transaction = new Transaction();

  // Check if the associated token account already exists
  const accountInfo = await connection.getAccountInfo(wsolTokenAccount);
  if (!accountInfo) {
    console.log("Associated token account does not exist. Creating...");
    // Create the associated token account for WSOL
    transaction.add(
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,       // Payer
        wsolTokenAccount,       // Associated token account
        wallet.publicKey,       // Owner
        wrappedSolMint          // Mint
      )
    );
  } else {
    console.log("Associated token account already exists.");
  }

  // Transfer lamports to the WSOL token account to wrap SOL
  console.log(`Transferring ${lamports} lamports to WSOL token account...`);
  transaction.add(
    SystemProgram.transfer({
      fromPubkey: wallet.publicKey,
      toPubkey: wsolTokenAccount,
      lamports: lamports,
    })
  );

  // Synchronize the native balance of the WSOL account
  console.log("Synchronizing native balance of WSOL account...");
  transaction.add(
    createSyncNativeInstruction(
      wsolTokenAccount,
      TOKEN_PROGRAM_ID
    )
  );

  // Set the fee payer
  transaction.feePayer = wallet.publicKey;

  // Fetch a recent blockhash
  const { blockhash } = await connection.getLatestBlockhash("confirmed");
  transaction.recentBlockhash = blockhash;

  console.log(`Transaction feePayer set to: ${transaction.feePayer.toBase58()}`);
  console.log(`Transaction recentBlockhash set to: ${transaction.recentBlockhash}`);

  // Sign the transaction using the wallet and send it
  try {
    // Sign the transaction using the wallet
    const signedTransaction = await wallet.signTransaction(transaction);

    // Verify that the transaction has a recentBlockhash
    if (!signedTransaction.recentBlockhash) {
      throw new Error("Signed transaction is missing a recent blockhash.");
    }

    console.log(`Signed transaction blockhash: ${signedTransaction.recentBlockhash}`);

    // Send the signed transaction
    const signature = await connection.sendRawTransaction(signedTransaction.serialize(), { commitment: "confirmed" });

    // Confirm the transaction
    await confirmTransaction(signature);

    console.log(`Wrapped ${amount} SOL into WSOL at ${wsolTokenAccount.toString()} with signature ${signature}`);
    return wsolTokenAccount;
  } catch (error) {
    console.error("Error wrapping SOL into WSOL:", error);
    throw error;
  }
}


describe("splyce-lending", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SplyceLending as Program<SplyceLending>;

  it("init_lending_market", async () => {

    // Set quote currency to "USD" padded with null bytes (32 bytes total)
    const quoteCurrency = Buffer.from(
      "USD" + "\0".repeat(29), // "USD" (3 characters) + 29 null bytes
      "utf-8"
    );

    // Derive the PDA for lending market using the signer's key (payer)
    const [lendingMarketPDA, bump] = await PublicKey.findProgramAddress(
      [provider.wallet.publicKey.toBuffer()],
      program.programId
    );

    // Get the payer account (signer)
    const payer = provider.wallet.publicKey;

    // Initialize the transaction for initializing the lending market
    const tx = await program.methods
      .initLendingMarket(quoteCurrency)
      .accounts({
        lendingMarket: lendingMarketPDA,
        signer: payer,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log("Transaction signature:", tx);

    const lendingMarketAccount = await program.account.lendingMarket.fetch(
      lendingMarketPDA
    );
    console.log("Lending Market Account:", lendingMarketAccount);

    // Assert lending market account fields
    assert.equal(lendingMarketAccount.version, 1, "Version should be 1");
    assert.equal(lendingMarketAccount.bumpSeed, bump, "Bump seed should match the derived bump");

    // Assert owner public key
    assert.equal(
      lendingMarketAccount.owner.toBase58(),
      payer.toBase58(),
      "Owner should be the payer public key"
    );

    // Assert quote currency (USD)
    const expectedQuoteCurrency = Buffer.from(
      "USD" + "\0".repeat(29), 
      "utf-8"
    );
    assert.deepEqual(
      lendingMarketAccount.quoteCurrency,
      [...expectedQuoteCurrency],
      "Quote currency should be USD padded with null bytes"
    );

    // Assert token program ID
    assert.equal(
      lendingMarketAccount.tokenProgramId.toBase58(),
      TOKEN_PROGRAM_ID.toBase58(),
      "Token Program ID should match the SPL Token Program ID"
    );

    // Assert rate limiter config
    assert.equal(
      lendingMarketAccount.rateLimiter.config.windowDuration.toString(),
      "1",
      "Rate limiter window duration should be 1"
    );
    assert.equal(
      lendingMarketAccount.rateLimiter.config.maxOutflow.toString(),
      "340282366920938463463374607431768211455",
      "Rate limiter max outflow should be max value (u128)"
    );

    // Assert whitelisted liquidator is null
    assert.isNull(lendingMarketAccount.whitelistedLiquidator, "Whitelisted liquidator should be null");

    // Assert risk authority
    assert.equal(
      lendingMarketAccount.riskAuthority.toBase58(),
      payer.toBase58(),
      "Risk authority should be the payer public key"
    );
  });

  it("set_lending_market_owner_and_config", async () => {
    // Derive the PDA for lending market using the signer's key (payer)
    const [lendingMarketPDA, bump] = await PublicKey.findProgramAddress(
      [provider.wallet.publicKey.toBuffer()],
      program.programId
    );

    interface RateLimiterConfig {
      windowDuration: anchor.BN;  // u64
      maxOutflow: anchor.BN;      // u128
    }

    const rateLimiterConfig: RateLimiterConfig = {
      windowDuration: new anchor.BN(10),  // window size of 100 slots
      maxOutflow: new anchor.BN("1000000000000000000")  // max outflow of 1e18 tokens
    };

    // Get the payer account (signer)
    const payer = provider.wallet.publicKey;

    const newOwner = Keypair.generate();
    const liquidator = Keypair.generate();
    const riskAuthority = Keypair.generate();

    // Initialize the transaction for setting the lending market owner and config
    const tx = await program.methods
      .setLendingMarketOwnerAndConfig(
        newOwner.publicKey,
        rateLimiterConfig,
        liquidator.publicKey,
        riskAuthority.publicKey
      )
      .accounts({
        lendingMarket: lendingMarketPDA,
        signer: payer,
      })
      .rpc();

    console.log("Transaction signature:", tx);

    const lendingMarketAccount = await program.account.lendingMarket.fetch(
      lendingMarketPDA
    );

    console.log("Lending Market Account:", lendingMarketAccount);

      // Assert that the version is still 1
    assert.equal(lendingMarketAccount.version, 1, "Version should be 1");

    // Assert bumpSeed matches the derived bump
    assert.equal(lendingMarketAccount.bumpSeed, bump, "Bump seed should match the derived bump");

    // Assert owner public key is the new owner
    assert.equal(
      lendingMarketAccount.owner.toBase58(),
      newOwner.publicKey.toBase58(),
      "Owner should be the newly set owner public key"
    );

    // Assert quote currency (USD), remains the same
    const expectedQuoteCurrency = Buffer.from(
      "USD" + "\0".repeat(29),
      "utf-8"
    );
    assert.deepEqual(
      lendingMarketAccount.quoteCurrency,
      [...expectedQuoteCurrency],
      "Quote currency should still be USD padded with null bytes"
    );

    // Assert token program ID hasn't changed
    assert.equal(
      lendingMarketAccount.tokenProgramId.toBase58(),
      TOKEN_PROGRAM_ID.toBase58(),
      "Token Program ID should remain the same"
    );

    // Assert rate limiter config
    assert.equal(
      lendingMarketAccount.rateLimiter.config.windowDuration.toString(),
      rateLimiterConfig.windowDuration.toString(),
      "Rate limiter window duration should be set to 10"
    );
    assert.equal(
      lendingMarketAccount.rateLimiter.config.maxOutflow.toString(),
      rateLimiterConfig.maxOutflow.toString(),
      "Rate limiter max outflow should be 1e18 tokens"
    );

    // Assert whitelisted liquidator public key matches the new liquidator
    assert.equal(
      lendingMarketAccount.whitelistedLiquidator.toBase58(),
      liquidator.publicKey.toBase58(),
      "Whitelisted liquidator should be the newly set liquidator"
    );

    // Assert risk authority public key matches the new risk authority
    assert.equal(
      lendingMarketAccount.riskAuthority.toBase58(),
      riskAuthority.publicKey.toBase58(),
      "Risk authority should be the newly set risk authority"
    );
  });

  it("init_reserve", async () => {
      // 1) Check SOL balance before airdrop
      const balanceBefore = await provider.connection.getBalance(provider.wallet.publicKey);
      console.log("Balance before airdrop:", balanceBefore / LAMPORTS_PER_SOL, "SOL");

      // 2) Airdrop SOL (ensure sufficient SOL for wrapping and transactions)
      const airdropAmount = 2; // Airdrop 2 SOL to cover wrapping and rent
      await airdropSol(provider.wallet.publicKey, airdropAmount);

      // 3) Check SOL balance after airdrop
      const balanceAfter = await provider.connection.getBalance(provider.wallet.publicKey);
      console.log("Balance after airdrop:", balanceAfter / LAMPORTS_PER_SOL, "SOL");

      // 4) Wrap SOL into WSOL
      const wrapAmount = 1; // Amount of SOL to wrap
      const wsolTokenAccount = await wrapSOL(provider, wrapAmount);

      // 5) Verify WSOL balance
      const tokenAccountInfo = await provider.connection.getTokenAccountBalance(wsolTokenAccount);
      console.log(`WSOL Token Account Balance: ${tokenAccountInfo.value.uiAmount} WSOL`);

      assert.equal(tokenAccountInfo.value.uiAmount, wrapAmount, "WSOL balance should match the wrapped amount");

      console.log("logging Token Account Info", tokenAccountInfo);

      // 6) Init MockPythPriceFeed
      // Derive the PDA for lending market using the signer's key (payer)
      const initialPriceOfSolInLamport = (100 * LAMPORTS_PER_SOL ); // $100
      const initialPriceOfSol = new anchor.BN(initialPriceOfSolInLamport); // $100
      // const initialPriceOfSol = anchor.BN(100); // $100

      const seeds = [
        provider.wallet.publicKey.toBuffer(),
        initialPriceOfSol.toArrayLike(Buffer, "le", 8)
        ];
      const [MockPythPriceFeedPDA, bump] = await PublicKey.findProgramAddress(
        seeds,
        program.programId
      );

    // Initialize the transaction for initializing the lending market
    const tx1 = await program.methods
      .initMockPythFeed(
        initialPriceOfSol,
        new anchor.BN(9),
      )
      .accounts({
        mockPythFeed: MockPythPriceFeedPDA,
      })
      .rpc();

    console.log("Transaction signature:", tx1);

    // const programAccount = await program.account;
    // console.log("Program Account:", programAccount);
    const mockPythPriceFeedPDA = await program.account.mockPythPriceFeed.fetch(
      MockPythPriceFeedPDA
    );
    console.log("mockPythPriceFeedPDA Account:", mockPythPriceFeedPDA);


  });

});