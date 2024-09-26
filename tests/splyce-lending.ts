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

const wrappedSolMint = new PublicKey("So11111111111111111111111111111111111111112");

// Define 'key' for reserve
const key = new anchor.BN(1);

const keyBuffer = key.toArrayLike(Buffer, 'le', 8);

console.log("Client Key Buffer (Hex):", keyBuffer.toString('hex'));


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

  console.log("Client Program ID:", program.programId.toString());

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
    //airdrop some SOL to the new owner
    await airdropSol(newOwner.publicKey, 1);
    const liquidator = Keypair.generate();
    const riskAuthority = Keypair.generate();

    // Initialize the transaction for setting the lending market owner and config
    const tx = await program.methods
      .setLendingMarketOwnerAndConfig(
        newOwner.publicKey,
        rateLimiterConfig,
        liquidator.publicKey,
        riskAuthority.publicKey,
        payer // orginal owner
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
    
    const tx2 = await program.methods
    .setLendingMarketOwnerAndConfig(
      payer,
      rateLimiterConfig,
      liquidator.publicKey,
      riskAuthority.publicKey,
      payer // orginal owner
    )
    .accounts({
      lendingMarket: lendingMarketPDA,
      signer: newOwner.publicKey,
    })
    .signers([newOwner])
    .rpc();

    const lendingMarketAccount2 = await program.account.lendingMarket.fetch(
      lendingMarketPDA
    );
    // Assert owner public key is the new owner
    assert.equal(
      lendingMarketAccount2.owner.toBase58(),
      payer.toBase58(),
      "Owner should be set back to the original owner public key"
    );
  });

  it("init_reserve", async () => {
    try {
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
  
      console.log("Token Account Info:", tokenAccountInfo);
  
      // 6) Init MockPythPriceFeed
      const initialPriceOfSolInLamports = new anchor.BN(100 * LAMPORTS_PER_SOL); // $100
      const expo = new anchor.BN(9); // Exponent for price feed
  
      // Derive the PDA for the MockPythPriceFeed using the signer's key and initial price
      const seeds = [
        provider.wallet.publicKey.toBuffer(),
        initialPriceOfSolInLamports.toArrayLike(Buffer, "le", 8),
      ];
      const [mockPythPriceFeedPDA, bump] = await PublicKey.findProgramAddress(
        seeds,
        program.programId
      );
  
      // Initialize the mock Pyth price feed
      await program.methods
        .initMockPythFeed(
          initialPriceOfSolInLamports,
          expo,
        )
        .accounts({
          mockPythFeed: mockPythPriceFeedPDA,
          signer: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
  
      console.log("Initialized MockPythPriceFeed at:", mockPythPriceFeedPDA.toBase58());
  
      const mockPythPriceFeedAccount = await program.account.mockPythPriceFeed.fetch(
        mockPythPriceFeedPDA
      );
      console.log("MockPythPriceFeed Account:", mockPythPriceFeedAccount);
  
      // 7) Prepare for init_reserve
  
      // Define 'key' for reserve
      const key = new anchor.BN(1);
  
      const keyBuffer = key.toArrayLike(Buffer, 'le', 8);
  
      console.log("Client Key Buffer (Hex):", keyBuffer.toString('hex'));
  
      // Derive reserve PDA
      const [reservePDA, reserveBump] = await PublicKey.findProgramAddress(
        [
          Buffer.from("reserve"),
          // key.toArrayLike(Buffer, 'le', 8), // TODO: Investigate why this is different from the program.
          provider.wallet.publicKey.toBuffer(),
        ],
        program.programId
      );
  
      // Get lendingMarketPDA (from previous test)
      const [lendingMarketPDA, lendingMarketBump] = await PublicKey.findProgramAddress(
        [provider.wallet.publicKey.toBuffer()],
        program.programId
      );
  
      // Create a Keypair for the collateral mint account (LP token mint)
      // This account will be initialized in the instruction with the mint authority set to the lending market
      const collateralMintKeypair = Keypair.generate();
      console.log("Collateral Mint Account:", collateralMintKeypair.publicKey.toBase58());
      const liquidityFeeAccountOwner = Keypair.generate();
      console.log("Liquidity Fee Account Owner:", liquidityFeeAccountOwner.publicKey.toBase58());
      await airdropSol(liquidityFeeAccountOwner.publicKey, 1); // Airdrop 1 SOL
      console.log("Collateral Mint Account:", collateralMintKeypair.publicKey.toBase58());
      const defaultSigner = provider.wallet.publicKey;
      console.log("Default Signer:", defaultSigner.toBase58());
  
      // Airdrop SOL to collateralMintKeypair to cover rent for the mint account
      await airdropSol(collateralMintKeypair.publicKey, 1); // Airdrop 1 SOL
      console.log("LendingMarketPDA:", lendingMarketPDA.toBase58());
  
      // Collateral Reserve Account (Associated Token Account for collateral mint, owned by lendingMarketPDA)
      const collateralReserveAccount = await getAssociatedTokenAddress(
        collateralMintKeypair.publicKey, // Collateral mint (LP token mint)
        lendingMarketPDA,                // Owner of the account (PDA)
        true,                            // Allow owner off curve
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
  
      // Collateral User Account (Associated Token Account for collateral mint, owned by provider.wallet.publicKey)
      const collateralUserAccount = await getAssociatedTokenAddress(
        collateralMintKeypair.publicKey, // Collateral mint (LP token mint)
        provider.wallet.publicKey,       // Owner of the account (on-curve)
        false,
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
  
      // Liquidity Reserve Account (Associated Token Account for WSOL, owned by lendingMarketPDA)
      const liquidityReserveAccount = await getAssociatedTokenAddress(
        wrappedSolMint,                  // WSOL mint
        lendingMarketPDA,                // Owner of the account (PDA)
        true,                            // Allow owner off curve
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
  
      // Liquidity Fee Account (Associated Token Account for WSOL, owned by another keypair)
      const liquidityFeeAccount = await getAssociatedTokenAddress(
        wrappedSolMint,                  // WSOL mint
        liquidityFeeAccountOwner.publicKey,        // Owner of the account (on-curve)
        false,
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
  
      // Liquidity User Account is the WSOL token account of the provider.wallet
      // This is where the WSOL comes from (provided by the default signer)
      const liquidityUserAccount = wsolTokenAccount;
  
      // Prepare ReserveConfig
      const reserveConfig = {
        optimalUtilizationRate: 80,
        maxUtilizationRate: 90,
        loanToValueRatio: 50,
        liquidationBonus: 5,
        maxLiquidationBonus: 10,
        liquidationThreshold: 60,
        maxLiquidationThreshold: 70,
        minBorrowRate: 0,
        optimalBorrowRate: 4,
        maxBorrowRate: 8,
        superMaxBorrowRate: new anchor.BN(12),
        fees: {
          borrowFeeWad: new anchor.BN(0),
          flashLoanFeeWad: new anchor.BN(0),
          hostFeePercentage: 0,
        },
        depositLimit: new anchor.BN(1000 * LAMPORTS_PER_SOL),
        borrowLimit: new anchor.BN(500 * LAMPORTS_PER_SOL),
        feeReceiver: liquidityFeeAccount, // Ensure feeReceiver is correctly handled if needed
        protocolLiquidationFee: 1,
        protocolTakeRate: 1,
        addedBorrowWeightBps: new anchor.BN(0),
        reserveType: { regular: {} },
        scaledPriceOffsetBps: new anchor.BN(0),
        extraOraclePubkey: null,
        attributedBorrowLimitOpen: new anchor.BN(0),
        attributedBorrowLimitClose: new anchor.BN(0),
        reserveSetterProgramId: null,
      };
  
      // Define liquidity amount to deposit (0.5 SOL in lamports)
      const liquidityAmount = new anchor.BN(0.5 * LAMPORTS_PER_SOL); // Deposit 0.5 SOL
  
      // Get feed_id from the mock Pyth price feed
      const feedId = mockPythPriceFeedPDA.toBuffer();
  
      // Ensure feedId is 32 bytes
      if (feedId.length !== 32) {
        throw new Error('feedId must be 32 bytes');
      }
  
      // console.log("collateralReserveAccount", collateralReserveAccount.toBase58());
      // console.log("collateralUserAccount", collateralUserAccount.toBase58());
      // console.log("liquidityReserveAccount", liquidityReserveAccount.toBase58());
      // console.log("liquidityFeeAccount", liquidityFeeAccount.toBase58());
      // console.log("liquidityUserAccount", liquidityUserAccount.toBase58());
      // console.log("reservePDA", reservePDA.toBase58());
      // console.log("lendingMarketPDA", lendingMarketPDA.toBase58());
  
      // Now, call the instruction to initialize the reserve within try-catch
      const tx = await program.methods
        .initReserve(
          liquidityAmount,
          key,
          Array.from(feedId),
          reserveConfig,
          true  // is_test
        )
        .accounts({
          reserve: reservePDA,
          lendingMarket: lendingMarketPDA,
          collateralMintAccount: collateralMintKeypair.publicKey,
          collateralReserveAccount: collateralReserveAccount,
          collateralUserAccount: collateralUserAccount,
          liquidityMintAccount: wrappedSolMint,
          liquidityReserveAccount: liquidityReserveAccount,
          liquidityFeeAccount: liquidityFeeAccount,
          liquidityUserAccount: liquidityUserAccount,
          signer: provider.wallet.publicKey,
          feeAccountOwner: liquidityFeeAccountOwner.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          mockPythFeed: mockPythPriceFeedPDA,
        })
        .signers([
          collateralMintKeypair,
          liquidityFeeAccountOwner
        ])
        .rpc();
  
      // If the transaction is successful, you can proceed with further assertions
      console.log("initReserve transaction signature:", tx);
  
      // Fetch the reserve account and check its fields
      const reserveAccount = await program.account.reserve.fetch(reservePDA);
      console.log("Reserve Account:", reserveAccount);
  
      // Assert that the reserve account has been initialized correctly
      assert.equal(reserveAccount.version, 1, "Reserve version should be 1");
      assert.equal(reserveAccount.lendingMarket.toBase58(), lendingMarketPDA.toBase58(), "Lending market should match");
      assert.equal(reserveAccount.key.toString(), key.toString(), "Reserve key should match");
  
      // Additional assertions can be added here as needed
  
    } catch (error) {
      // Check if the error has logs
      if (error.logs) {
        console.error("Transaction failed with logs:", error.logs);
      } else {
        console.error("Transaction failed without logs:", error);
      }
  
      // If using Anchor's SendTransactionError, you can access the logs like this:
      if (error instanceof anchor.AnchorError) {
        console.error("Anchor Error:", error.error);
        console.error("Error Logs:", error.logs);
      }
  
      // Optionally, rethrow the error if you want the test to fail
      throw error;
    }
  });

});