import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SplyceLending } from "../target/types/splyce_lending";
import {
  TOKEN_PROGRAM_ID,
  createSyncNativeInstruction,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getMint
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

async function wrapSOL2(
  provider: anchor.AnchorProvider,
  payer: Keypair,
  amount: number
): Promise<PublicKey> {
  const connection = provider.connection;
  const lamports = amount * LAMPORTS_PER_SOL;
  console.log(`Wrapping ${amount} SOL (${lamports} lamports)`);

  const wsolTokenAccount = await getAssociatedTokenAddress(
    wrappedSolMint,
    payer.publicKey,
    false,
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  console.log(`WSOL Associated Token Account: ${wsolTokenAccount.toBase58()}`);

  // Check if the associated token account already exists
  const accountInfo = await connection.getAccountInfo(wsolTokenAccount);
  const transaction = new Transaction();
  if (!accountInfo) {
    console.log("Associated token account does not exist. Creating...");
    transaction.add(
      createAssociatedTokenAccountInstruction(
        payer.publicKey,    // Payer
        wsolTokenAccount,   // Associated token account
        payer.publicKey,    // Owner
        wrappedSolMint      // Mint
      )
    );
  } else {
    console.log("Associated token account already exists.");
  }

  // Transfer lamports to the WSOL token account to wrap SOL
  console.log(`Transferring ${lamports} lamports to WSOL token account...`);
  transaction.add(
    SystemProgram.transfer({
      fromPubkey: payer.publicKey,
      toPubkey: wsolTokenAccount,
      lamports,
    })
  );

  // Synchronize the native balance of the WSOL account
  console.log("Synchronizing native balance of WSOL account...");
  transaction.add(createSyncNativeInstruction(wsolTokenAccount));

  // Set the fee payer
  transaction.feePayer = payer.publicKey;

  // Fetch recent blockhash
  const { blockhash } = await connection.getLatestBlockhash("confirmed");
  transaction.recentBlockhash = blockhash;

  // Sign the transaction with the payer's Keypair
  transaction.sign(payer);

  // Send the transaction
  const txid = await connection.sendRawTransaction(transaction.serialize());
  await connection.confirmTransaction(txid);

  console.log(`Wrapped ${amount} SOL into WSOL at ${wsolTokenAccount.toBase58()} with signature ${txid}`);
  return wsolTokenAccount;
}


describe("splyce-lending", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SplyceLending as Program<SplyceLending>;

  console.log("Client Program ID:", program.programId.toString());
  let riskAuthorityGlobal;

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
      const balanceBeforeAirdrop = await provider.connection.getBalance(provider.wallet.publicKey);
      console.log("Balance before airdrop:", balanceBeforeAirdrop / LAMPORTS_PER_SOL, "SOL");
  
      // 2) Airdrop SOL (ensure sufficient SOL for wrapping and transactions)
      const airdropAmount = 2; // Airdrop 2 SOL to cover wrapping and rent
      await airdropSol(provider.wallet.publicKey, airdropAmount);
  
      // 3) Check SOL balance after airdrop
      const balanceAfterAirdrop = await provider.connection.getBalance(provider.wallet.publicKey);
      console.log("Balance after airdrop:", balanceAfterAirdrop / LAMPORTS_PER_SOL, "SOL");
  
      // 4) Wrap SOL into WSOL
      const wrapAmount = 1; // Amount of SOL to wrap
      const wsolTokenAccount = await wrapSOL(provider, wrapAmount);
  
      // 5) Verify WSOL balance
      const tokenAccountInfoBefore = await provider.connection.getTokenAccountBalance(wsolTokenAccount);
      console.log(`WSOL Token Account Balance before initReserve: ${tokenAccountInfoBefore.value.uiAmount} WSOL`);
  
      assert.equal(
        tokenAccountInfoBefore.value.uiAmount,
        wrapAmount,
        "WSOL balance should match the wrapped amount before initReserve"
      );
  
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
          keyBuffer,
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
  
      // 8) **Fetch and Store Balances Before initReserve**
  
      // Fetch WSOL balance before initReserve
      const wsolBalanceBeforeInit = await provider.connection.getTokenAccountBalance(liquidityUserAccount);
      const wsolBalanceBefore = parseFloat(wsolBalanceBeforeInit.value.uiAmountString || "0");
      console.log(`WSOL Balance before initReserve: ${wsolBalanceBefore} WSOL`);
  
      // 9) **Call the `initReserve` Instruction**
  
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
  
      // 10) **Fetch and Store Balances After initReserve**
  
      // Fetch WSOL balance after initReserve
      const wsolBalanceAfterInit = await provider.connection.getTokenAccountBalance(liquidityUserAccount);
      const wsolBalanceAfter = parseFloat(wsolBalanceAfterInit.value.uiAmountString || "0");
      console.log(`WSOL Balance after initReserve: ${wsolBalanceAfter} WSOL`);
  
      // Fetch Collateral Token balance after initReserve
      const collateralBalanceAfterInit = await provider.connection.getTokenAccountBalance(collateralUserAccount);
      const collateralBalanceAfter = parseFloat(collateralBalanceAfterInit.value.uiAmountString || "0");
      console.log(`Collateral Token Balance after initReserve: ${collateralBalanceAfter} CTokens`);
  
      // 11) **Assert the Balance Changes**
  
      // Calculate expected WSOL balance
      const expectedWsolBalance = wsolBalanceBefore - (liquidityAmount.toNumber() / LAMPORTS_PER_SOL);
      assert.closeTo(
        wsolBalanceAfter,
        expectedWsolBalance,
        0.0001,
        `WSOL balance should decrease by ${liquidityAmount.toNumber() / LAMPORTS_PER_SOL} WSOL`
      );
  
      // Fetch the reserve account to determine the collateral amount minted
      const reserveAccount = await program.account.reserve.fetch(reservePDA);
      console.log("Reserve Account:", reserveAccount);
      const collateralAmountMinted = reserveAccount.collateral.mintTotalSupply.toNumber() / 1e9; // Assuming smoothedMarketPrice reflects the collateral minted in this test
  
      // Calculate expected Collateral Token balance
      const expectedCollateralBalance = collateralAmountMinted;
      assert.closeTo(
        collateralBalanceAfter,
        expectedCollateralBalance,
        0.0001,
        `Collateral Token balance should increase by ${collateralAmountMinted} CTokens`
      );
  
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

  it("update_reserve_config as BERNANKE (success, when not the lending_market.owner)", async () => {
        // Get the payer account (signer)
        const payer = provider.wallet.publicKey;
        // Setting lending market owner to someone else
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
    
        //done setting lending market owner to someone else
  
    const key = new anchor.BN(1);
    const keyBuffer = key.toArrayLike(Buffer, 'le', 8);
  
    const [reservePDA, reserveBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from("reserve"),
        keyBuffer,
        provider.wallet.publicKey.toBuffer(),
      ],
      program.programId
    );
  
    // Fetch the reserve account
    const reserveAccountBefore = await program.account.reserve.fetch(reservePDA);
    const currentConfig = reserveAccountBefore.config;
  
    // Prepare new config with changes allowed for BERNANKE
    const newConfig = { ...currentConfig };
    newConfig.fees.borrowFeeWad = new anchor.BN(1000); // Set borrow fee to 0.1%
    newConfig.protocolLiquidationFee = 5; // Set protocol liquidation fee to 0.5%
    newConfig.protocolTakeRate = 2; // Set protocol take rate to 2%
    // Assuming feeReceiver is an existing public key
    newConfig.feeReceiver = provider.wallet.publicKey;
  
    // Call update_reserve_config as BERNANKE
    await program.methods
      .initUpdateReserveConfig(
        newConfig,
        rateLimiterConfig,
        true // is_test
      )
      .accounts({
        reserve: reservePDA,
        lendingMarket: lendingMarketPDA,
        signer: provider.wallet.publicKey,
        lendingMarketOwner: provider.wallet.publicKey, // Assuming BERNANKE is not the owner
        mockPythFeed: reserveAccountBefore.mockPythFeed,
      })
      .rpc();
  
    // Fetch the reserve account after update
    const reserveAccountAfter = await program.account.reserve.fetch(reservePDA);
  
    // Verify that the config has been updated
    assert.equal(reserveAccountAfter.config.fees.borrowFeeWad.toString(), newConfig.fees.borrowFeeWad.toString(), "Borrow fee should be updated");
    assert.equal(reserveAccountAfter.config.protocolLiquidationFee, newConfig.protocolLiquidationFee, "Protocol liquidation fee should be updated");
    assert.equal(reserveAccountAfter.config.protocolTakeRate, newConfig.protocolTakeRate, "Protocol take rate should be updated");
    assert.equal(reserveAccountAfter.config.feeReceiver.toBase58(), newConfig.feeReceiver.toBase58(), "Fee receiver should be updated");
  
    // Verify that 'stale' is set to true
    assert.isTrue(reserveAccountAfter.lastUpdate.stale, "Reserve last update should be marked as stale");

    // Return the lending market owner to the original owner
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
  });

  it("update_reserve_config as BERNANKE and market owner (success)", async () => {
    // Get the payer account (signer)
    const payer = provider.wallet.publicKey;
  
    // Get the lending market PDA
    const [lendingMarketPDA, bump] = await PublicKey.findProgramAddress(
      [provider.wallet.publicKey.toBuffer()],
      program.programId
    );
  
    // Fetch the reserve PDA
    const key = new anchor.BN(1);
    const keyBuffer = key.toArrayLike(Buffer, 'le', 8);
  
    const [reservePDA, reserveBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from("reserve"),
        keyBuffer,
        provider.wallet.publicKey.toBuffer(),
      ],
      program.programId
    );
  
    // Fetch the reserve account
    const reserveAccountBefore = await program.account.reserve.fetch(reservePDA);
    const currentConfig = reserveAccountBefore.config;
  
    // Prepare new config with allowed changes
    const newConfig = { ...currentConfig };
    newConfig.fees.borrowFeeWad = new anchor.BN(2000); // Set borrow fee to 0.2%
    newConfig.protocolLiquidationFee = 10; // Set protocol liquidation fee to 1.0%
    newConfig.protocolTakeRate = 4; // Set protocol take rate to 2%

          // 6) Init MockPythPriceFeed
          const initialPriceOfSolInLamports = new anchor.BN(110 * LAMPORTS_PER_SOL); // $100
          const expo = new anchor.BN(9); // Exponent for price feed
      
          // Derive the PDA for the MockPythPriceFeed using the signer's key and initial price
          const seeds = [
            provider.wallet.publicKey.toBuffer(),
            initialPriceOfSolInLamports.toArrayLike(Buffer, "le", 8),
          ];
          const [mockPythPriceFeedPDA, bumpMockPyth] = await PublicKey.findProgramAddress(
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
          console.log("Reserve Account Before MockPythPDA:", reserveAccountBefore.mockPythFeed);

    await program.methods
      .initUpdateReserveConfig(
        newConfig,
        reserveAccountBefore.rateLimiter.config,
        true // is_test
      )
      .accounts({
        reserve: reservePDA,
        lendingMarket: lendingMarketPDA,
        signer: payer,
        lendingMarketOwner: payer,
        mockPythFeed: mockPythPriceFeedPDA,
      })
      .rpc();
  
    // Fetch the reserve account after update
    const reserveAccountAfter = await program.account.reserve.fetch(reservePDA);
  
    // Verify that the config has been updated
    assert.equal(reserveAccountAfter.config.fees.borrowFeeWad.toString(), newConfig.fees.borrowFeeWad.toString(), "Borrow fee should be updated");
    assert.equal(reserveAccountAfter.config.protocolLiquidationFee, newConfig.protocolLiquidationFee, "Protocol liquidation fee should be updated");
    assert.equal(reserveAccountAfter.config.protocolTakeRate, newConfig.protocolTakeRate, "Protocol take rate should be updated");
    console.log("Reserve Account After MockPythPDA:", reserveAccountAfter.mockPythFeed);
    assert.equal(reserveAccountAfter.mockPythFeed, mockPythPriceFeedPDA.toBase58(), "New mockPythPriceFeed should be updated");

    // Verify that 'stale' is set to true
    assert.isTrue(
      reserveAccountAfter.lastUpdate.stale,
      "Reserve last update should be marked as stale"
    );
  });
  
  it("update_reserve_config as lending market owner (fail when changing forbidden fields)", async () => {
    // Generate a new owner keypair
    const newOwner = Keypair.generate();
    await airdropSol(newOwner.publicKey, 1);
    
    const payer = provider.wallet.publicKey;
    const liquidator = Keypair.generate();
    const riskAuthority = Keypair.generate();
    riskAuthorityGlobal = riskAuthority;
    
    const rateLimiterConfig = {
      windowDuration: new anchor.BN(10),  // window size of 10 slots
      maxOutflow: new anchor.BN("1000000000000000000")  // max outflow of 1e18 tokens
    };
  
    // Derive the lending market PDA
    const [lendingMarketPDA, bump] = await PublicKey.findProgramAddress(
      [provider.wallet.publicKey.toBuffer()],
      program.programId
    );
  
    // Set the lending market owner to newOwner
    const tx = await program.methods
      .setLendingMarketOwnerAndConfig(
        newOwner.publicKey,
        rateLimiterConfig,
        liquidator.publicKey,
        riskAuthority.publicKey,
        payer // original owner
      )
      .accounts({
        lendingMarket: lendingMarketPDA,
        signer: payer,
      })
      .rpc();
  
    // console.log("Transaction signature:", tx);
  
    // Define 'key' for reserve
    const key = new anchor.BN(1);
    const keyBuffer = key.toArrayLike(Buffer, 'le', 8);
  
    // Derive the reserve PDA
    const [reservePDA, reserveBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from("reserve"),
        keyBuffer,
        provider.wallet.publicKey.toBuffer(),
      ],
      program.programId
    );
  
    // Fetch the reserve account
    const reserveAccountBefore = await program.account.reserve.fetch(reservePDA);
    const currentConfig = reserveAccountBefore.config;
  
    // Prepare new config with forbidden changes
    const newConfig = { ...currentConfig };
    
    // Attempt to change forbidden fields
    newConfig.fees.borrowFeeWad = new anchor.BN(1000); // Change fees
    newConfig.protocolLiquidationFee = 5; // Change protocol liquidation fee
    newConfig.protocolTakeRate = 2; // Change protocol take rate
    newConfig.feeReceiver = Keypair.generate().publicKey; // Change fee receiver
  
    try {
      // Call update_reserve_config as the new owner
      await program.methods
        .initUpdateReserveConfig(
          newConfig,
          reserveAccountBefore.rateLimiter.config,
          true // is_test
        )
        .accounts({
          reserve: reservePDA,
          lendingMarket: lendingMarketPDA,
          signer: newOwner.publicKey, // Update signer to newOwner
          lendingMarketOwner: newOwner.publicKey, // Update owner to newOwner
          mockPythFeed: reserveAccountBefore.mockPythFeed,
        })
        .signers([newOwner])
        .rpc();
  
      // If the above does not throw, then the test fails
      assert.fail("Expected transaction to fail when changing forbidden fields");
    } catch (error) {
      // Log the error to understand its structure
      console.log("Caught error:", error);
  
      // Adjust error assertion based on actual error structure
      if ('error' in error && 'errorCode' in error.error) {
        // If error is wrapped inside AnchorError
        assert.equal(
          error.error.errorCode.code,
          "NotBernanke",
          `Expected error code: NotBernanke`
        );
      } else if ('message' in error && error.message.includes("NotBernanke")) {
        // Alternatively, check the message directly
        assert.include(
          error.message,
          "NotBernanke",
          `Expected error message to include "NotBernanke", got "${error.message}"`
        );
      } else {
        // If the error doesn't match expected structure, fail the test
        assert.fail("Expected an AnchorError with code NotBernanke");
      }
    }
  });
  
  it("update_reserve_config as unauthorized user (should fail)", async () => {
    // Create an unauthorized user
    const unauthorizedUser = Keypair.generate();
    await airdropSol(unauthorizedUser.publicKey, 1); // Airdrop 1 SOL
  
    // Get the lending market PDA
    const [lendingMarketPDA, bump] = await PublicKey.findProgramAddress(
      [provider.wallet.publicKey.toBuffer()],
      program.programId
    );
  
    // Fetch the reserve PDA
    const key = new anchor.BN(1);
    const keyBuffer = key.toArrayLike(Buffer, 'le', 8);
  
    const [reservePDA, reserveBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from("reserve"),
        keyBuffer,
        provider.wallet.publicKey.toBuffer(),
      ],
      program.programId
    );
  
    // Fetch the reserve account
    const reserveAccountBefore = await program.account.reserve.fetch(reservePDA);
    const currentConfig = reserveAccountBefore.config;
  
    // Prepare a new config (even same as before)
    const newConfig = { ...currentConfig };
  
    try {
      // Call update_reserve_config as the unauthorized user
      await program.methods
        .initUpdateReserveConfig(
          newConfig,
          reserveAccountBefore.rateLimiter.config,
          true // is_test
        )
        .accounts({
          reserve: reservePDA,
          lendingMarket: lendingMarketPDA,
          signer: unauthorizedUser.publicKey,
          lendingMarketOwner: provider.wallet.publicKey,
          mockPythFeed: reserveAccountBefore.mockPythFeed,
        })
        .signers([unauthorizedUser])
        .rpc();
      // If the above does not throw, then the test fails
      assert.fail(
        "Expected transaction to fail when unauthorized user attempts to update reserve config"
      );
    } catch (error) {
      // Check that the error is the expected one
      assert.ok(error instanceof anchor.AnchorError, "Expected an AnchorError");
      const errMsg = "Unauthorized";
      assert.equal(
        error.error.errorCode.code,
        "Unauthorized",
        `Expected error code: Unauthorized`
      );
    }
  });
  
  it("update_reserve_config as risk authority (success when decreasing limits and disabling outflows)", async () => {
        // Fetch the reserve account
        const key = new anchor.BN(1);
        const keyBuffer = key.toArrayLike(Buffer, 'le', 8);
    const [reservePDA, reserveBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from("reserve"),
        keyBuffer,
        provider.wallet.publicKey.toBuffer(),
      ],
      program.programId
    );
    // Create a risk authority keypair
    const riskAuthority = riskAuthorityGlobal;
    await airdropSol(riskAuthority.publicKey, 1);
    const reserveAccountBefore = await program.account.reserve.fetch(reservePDA);

  
    // Set the risk authority in the lending market
    const [lendingMarketPDA, bump] = await PublicKey.findProgramAddress(
      [provider.wallet.publicKey.toBuffer()],
      program.programId
    );
  
    // Set the lending market owner and config to set the risk authority
    // await program.methods
    //   .setLendingMarketOwnerAndConfig(
    //     provider.wallet.publicKey, // Keep owner as payer
    //     reserveAccountBefore.rateLimiter.config,
    //     null, // No whitelisted liquidator
    //     riskAuthority.publicKey,
    //     provider.wallet.publicKey // original owner
    //   )
    //   .accounts({
    //     lendingMarket: lendingMarketPDA,
    //     signer: provider.wallet.publicKey,
    //   })
    //   .rpc();
  

  

  
    // const reserveAccountBefore = await program.account.reserve.fetch(reservePDA);
    const currentConfig = reserveAccountBefore.config;
  
    // Prepare new config with decreased limits
    const newConfig = { ...currentConfig };
    newConfig.borrowLimit = currentConfig.borrowLimit.sub(
      new anchor.BN(100 * LAMPORTS_PER_SOL)
    ); // Decrease borrow limit
    newConfig.depositLimit = currentConfig.depositLimit.sub(
      new anchor.BN(100 * LAMPORTS_PER_SOL)
    ); // Decrease deposit limit
  
    // Prepare rate limiter config to disable outflows
    const newRateLimiterConfig = { ...reserveAccountBefore.rateLimiter.config };
    newRateLimiterConfig.maxOutflow = new anchor.BN(0); // Disable outflows
  
    // Call update_reserve_config as risk authority
    await program.methods
      .initUpdateReserveConfig(
        newConfig,
        newRateLimiterConfig,
        true // is_test
      )
      .accounts({
        reserve: reservePDA,
        lendingMarket: lendingMarketPDA,
        signer: riskAuthority.publicKey,
        lendingMarketOwner: provider.wallet.publicKey, // Owner is still provider.wallet
        mockPythFeed: reserveAccountBefore.mockPythFeed,
      })
      .signers([riskAuthority])
      .rpc();
  
    // Fetch the reserve account after update
    const reserveAccountAfter = await program.account.reserve.fetch(reservePDA);
  
    // Verify that borrowLimit and depositLimit have decreased
    assert.ok(
      reserveAccountAfter.config.borrowLimit.lt(currentConfig.borrowLimit),
      "Borrow limit should be decreased"
    );
    assert.ok(
      reserveAccountAfter.config.depositLimit.lt(currentConfig.depositLimit),
      "Deposit limit should be decreased"
    );
  
    // Verify that rate limiter maxOutflow is zero
    assert.equal(
      reserveAccountAfter.rateLimiter.config.maxOutflow.toString(),
      "0",
      "Max outflow should be zero"
    );
  
    // Verify that 'stale' is set to true
    assert.isTrue(
      reserveAccountAfter.lastUpdate.stale,
      "Reserve last update should be marked as stale"
    );
  });

  it("redeem_reserve_collateral", async () => {
    try {
      // 1) Fetch necessary accounts and PDAs
      const payer = provider.wallet.publicKey;
  
      // Derive the lending market PDA
      const [lendingMarketPDA, lendingMarketBump] = await PublicKey.findProgramAddress(
        [provider.wallet.publicKey.toBuffer()],
        program.programId
      );
  
      // Define 'key' for reserve
      const key = new anchor.BN(1);
      const keyBuffer = key.toArrayLike(Buffer, 'le', 8);
  
      // Derive the reserve PDA
      const [reservePDA, reserveBump] = await PublicKey.findProgramAddress(
        [
          Buffer.from("reserve"),
          keyBuffer,
          provider.wallet.publicKey.toBuffer(),
        ],
        program.programId
      );
  
      // Fetch the reserve account **before** redemption
      let reserveAccountBefore = await program.account.reserve.fetch(reservePDA);
      console.log("Reserve Account Before Redemption:", reserveAccountBefore);
  
      // Collateral Mint Account
      const collateralMintPubkey = reserveAccountBefore.collateral.mintPubkey;
  
      // Collateral User Account (where the user's collateral tokens are)
      const collateralUserAccount = await getAssociatedTokenAddress(
        collateralMintPubkey,
        provider.wallet.publicKey,
        false,
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
  
      // Liquidity Mint Account (e.g., WSOL)
      const liquidityMintPubkey = reserveAccountBefore.liquidity.mintPubkey;
  
      // Liquidity User Account (where the user's liquidity tokens will be received)
      const liquidityUserAccount = await getAssociatedTokenAddress(
        liquidityMintPubkey,
        provider.wallet.publicKey,
        false,
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
  
      // Liquidity Reserve Account (where the liquidity is stored in the reserve)
      const liquidityReserveAccount = reserveAccountBefore.liquidity.supplyPubkey;
  
      // Signer (the user)
      const signer = provider.wallet.publicKey;
  
      // 2) Ensure Liquidity User Account exists
      const liquidityUserAccountInfo = await provider.connection.getAccountInfo(liquidityUserAccount);
      if (!liquidityUserAccountInfo) {
        // Create the associated token account for liquidity token (e.g., WSOL)
        const createLiquidityUserAccountIx = createAssociatedTokenAccountInstruction(
          signer,                  // Payer
          liquidityUserAccount,    // Associated token account
          signer,                  // Owner
          liquidityMintPubkey      // Mint
        );
  
        const tx = new Transaction().add(createLiquidityUserAccountIx);
        await provider.sendAndConfirm(tx, []);
      }
  
      // 3) Fetch and Store Balances Before Redemption
  
      // Fetch collateral token balance before redemption
      const collateralBalanceBefore = await provider.connection.getTokenAccountBalance(collateralUserAccount);
      const collateralBalanceBeforeAmount = parseFloat(collateralBalanceBefore.value.uiAmountString || "0");
      console.log(`Collateral Token Balance before redemption: ${collateralBalanceBeforeAmount} CTokens`);
  
      // Fetch liquidity token balance before redemption
      const liquidityBalanceBefore = await provider.connection.getTokenAccountBalance(liquidityUserAccount);
      const liquidityBalanceBeforeAmount = parseFloat(liquidityBalanceBefore.value.uiAmountString || "0");
      console.log(`Liquidity Token Balance before redemption: ${liquidityBalanceBeforeAmount} Tokens`);
  
      // 4) Determine the amount of collateral to redeem
      const collateralAmountToRedeem = new anchor.BN(collateralBalanceBefore.value.amount).div(new anchor.BN(2)); // Redeem half
  
      // Ensure we are redeeming at least some amount
      if (collateralAmountToRedeem.lte(new anchor.BN(0))) {
        throw new Error("Insufficient collateral to redeem.");
      }
  
      console.log(`Redeeming ${collateralAmountToRedeem.toString()} collateral tokens`);
  
      console.log("Reserve PDA:", reservePDA.toBase58());
      console.log("Lending Market PDA:", lendingMarketPDA.toBase58());
      console.log("Collateral Mint Pubkey:", collateralMintPubkey.toBase58());
      console.log("Collateral User Account:", collateralUserAccount.toBase58());
      console.log("Liquidity Mint Pubkey:", liquidityMintPubkey.toBase58());
      console.log("Liquidity User Account:", liquidityUserAccount.toBase58());
      console.log("Liquidity Reserve Account:", liquidityReserveAccount.toBase58());
      console.log("Signer:", signer.toBase58());
  
      // Fetch the collateral mint account to get the decimals
      const collateralMintAccount = await getMint(provider.connection, collateralMintPubkey);
      const collateralDecimals = collateralMintAccount.decimals;
      console.log(`Collateral Decimals: ${collateralDecimals}`);
  
      // Fetch the liquidity mint account to get the decimals
      const liquidityMintAccount = await getMint(provider.connection, liquidityMintPubkey);
      const liquidityDecimals = liquidityMintAccount.decimals;
      console.log(`Liquidity Decimals: ${liquidityDecimals}`);
  
      // 5) **Calculate the Exchange Rate and Liquidity Amount Received**
  
      // Calculate the exchange rate manually using reserveAccountBefore
      const liquidityAvailableAmountBefore = new anchor.BN(reserveAccountBefore.liquidity.availableAmount.toString());
      console.log(`Liquidity Available Amount Before Redemption: ${liquidityAvailableAmountBefore.toString()}`);
      const collateralMintTotalSupplyBefore = new anchor.BN(reserveAccountBefore.collateral.mintTotalSupply.toString());
  
      const WAD = new anchor.BN(1e9); // Assuming WAD is 1e9
  
      let collateralExchangeRate;
      if (collateralMintTotalSupplyBefore.eq(new anchor.BN(0))) {
        collateralExchangeRate = WAD;
      } else {
        collateralExchangeRate = liquidityAvailableAmountBefore.mul(WAD).div(collateralMintTotalSupplyBefore);
      }
  
      console.log(`Collateral Exchange Rate: ${collateralExchangeRate.toString()}`);
  
      const liquidityAmountReceived = collateralAmountToRedeem.mul(collateralExchangeRate).div(WAD);
      console.log(`Liquidity Amount Received: ${liquidityAmountReceived.toString()}`);
      const liquidityAmountReceivedDecimal = liquidityAmountReceived.toNumber() / Math.pow(10, liquidityDecimals);
      console.log(`Liquidity Amount Received (Decimal): ${liquidityAmountReceivedDecimal}`);
  
      // Check if the reserve is stale before refreshing
      const reserveAccountBeforeRefresh = await program.account.reserve.fetch(reservePDA);
      console.log("Is reserve stale before refresh:", reserveAccountBeforeRefresh.lastUpdate.stale);

      // Create a transaction that includes both refresh and redeem instructions
      const transaction = new anchor.web3.Transaction();

      // Add refresh_reserve instruction
      const refreshIx = await program.methods
        .refreshReserve(true) // is_test
        .accounts({
          reserve: reservePDA,
          signer: provider.wallet.publicKey,
          mockPythFeed: reserveAccountBeforeRefresh.mockPythFeed,
        })
        .instruction();

      transaction.add(refreshIx);

      // Add redeem_reserve_collateral instruction
      const redeemIx = await program.methods
        .redeemReserveCollateral(collateralAmountToRedeem)
        .accounts({
          reserve: reservePDA,
          lendingMarket: lendingMarketPDA,
          collateralMintAccount: collateralMintPubkey,
          collateralUserAccount: collateralUserAccount,
          liquidityUserAccount: liquidityUserAccount,
          liquidityReserveAccount: liquidityReserveAccount,
          signer: signer,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .instruction();
      
      transaction.add(redeemIx);
      // Send and confirm the transaction
      const txSignature = await provider.sendAndConfirm(transaction);
      console.log("Transaction successful. Signature:", txSignature);

      // Check if the reserve is stale after redemption
      const reserveAccountAfterRedemption = await program.account.reserve.fetch(reservePDA);
      console.log("Is reserve stale after redemption:", reserveAccountAfterRedemption.lastUpdate.stale);
      // 7) Fetch and Store Balances After Redemption
  
      // Fetch collateral token balance after redemption
      const collateralBalanceAfter = await provider.connection.getTokenAccountBalance(collateralUserAccount);
      const collateralBalanceAfterAmount = parseFloat(collateralBalanceAfter.value.uiAmountString || "0");
      console.log(`Collateral Token Balance after redemption: ${collateralBalanceAfterAmount} CTokens`);
  
      // Fetch liquidity token balance after redemption
      const liquidityBalanceAfter = await provider.connection.getTokenAccountBalance(liquidityUserAccount);
      const liquidityBalanceAfterAmount = parseFloat(liquidityBalanceAfter.value.uiAmountString || "0");
      console.log(`Liquidity Token Balance after redemption: ${liquidityBalanceAfterAmount} Tokens`);
  
      // 8) Assert the Balance Changes
  
      // Calculate the collateral amount to redeem in decimal form
      const collateralAmountToRedeemDecimal = collateralAmountToRedeem.toNumber() / Math.pow(10, collateralDecimals);
      const expectedCollateralBalance = collateralBalanceBeforeAmount - collateralAmountToRedeemDecimal;
  
      console.log(`Collateral Amount to Redeem (Decimal): ${collateralAmountToRedeemDecimal}`);
      console.log(`Expected Collateral Balance: ${expectedCollateralBalance}`);
  
      // Collateral balance should decrease by the redeemed amount
      assert.closeTo(
        collateralBalanceAfterAmount,
        expectedCollateralBalance,
        0.0001,
        `Collateral balance should decrease by the redeemed amount`
      );
  
      // Calculate expected liquidity balance after redemption
      const expectedLiquidityBalance = liquidityBalanceBeforeAmount + liquidityAmountReceivedDecimal;
      console.log(`Expected Liquidity Balance After Redemption: ${expectedLiquidityBalance}`);
  
      // Liquidity balance should increase by the redeemed amount
      assert.closeTo(
        liquidityBalanceAfterAmount,
        expectedLiquidityBalance,
        0.0001,
        `Liquidity balance should increase by the redeemed amount`
      );
  
      // 9) Fetch the reserve account **after** redemption
      const reserveAccountAfter = await program.account.reserve.fetch(reservePDA);
  
      // 10) Verify that the reserve's available liquidity decreased accordingly
      const liquidityAvailableAmountAfter = new anchor.BN(reserveAccountAfter.liquidity.availableAmount.toString());
      console.log(`Liquidity Available Amount After Redemption: ${liquidityAvailableAmountAfter.toString()}`);
  
      const expectedAvailableAmount = liquidityAvailableAmountBefore.sub(liquidityAmountReceived);
  
      console.log("Check here!");
      console.log(`Actual Available Liquidity After Redemption: ${liquidityAvailableAmountAfter.toString()}`);
      console.log(`Expected Available Liquidity After Redemption: ${expectedAvailableAmount.toString()}`);
  
      assert.equal(
        liquidityAvailableAmountAfter.toString(),
        expectedAvailableAmount.toString(),
        "Reserve's available liquidity should decrease by the redeemed amount"
      );
  
      // 11) Verify that 'stale' is set to true after redemption
      assert.isTrue(
        reserveAccountAfter.lastUpdate.stale,
        "Reserve last update should be marked as stale after redemption"
      );
    } catch (error) {
      // Handle errors
      console.error("Error during redeem_reserve_collateral test:", error);
      throw error;
    }
  });

  it("deposit_reserve_liquidity as a secondary user", async () => {
    try {
      // 1. Initialize Secondary User
      const secondaryUser = Keypair.generate();
      await airdropSol(secondaryUser.publicKey, 2); // Airdrop 2 SOL to the secondary user
      console.log("Secondary User PublicKey:", secondaryUser.publicKey.toBase58());
  
      // 2. Wrap SOL into WSOL for Secondary User
      const wrapAmount = 1; // Amount of SOL to wrap
      const wsolTokenAccount = await wrapSOL2(provider, secondaryUser, wrapAmount);
      console.log(`WSOL Account for Secondary User: ${wsolTokenAccount.toBase58()}`);
  
      // 3. Derive Necessary PDAs
      const [lendingMarketPDA, _lendingMarketBump] = await PublicKey.findProgramAddress(
        [provider.wallet.publicKey.toBuffer()],
        program.programId
      );
  
      const key = new anchor.BN(1);
      const keyBuffer = key.toArrayLike(Buffer, 'le', 8);
  
      const [reservePDA, _reserveBump] = await PublicKey.findProgramAddress(
        [
          Buffer.from("reserve"),
          keyBuffer,
          provider.wallet.publicKey.toBuffer(), // Assuming the reserve is tied to the default signer
        ],
        program.programId
      );
  
      // 4. Create Collateral User Account for Secondary User
      // Assume collateral mint is already initialized in init_reserve
      const reserveAccount = await program.account.reserve.fetch(reservePDA);
      const collateralMintPubkey = reserveAccount.collateral.mintPubkey;
  
      // Derive the collateral user's associated token account (ATA) for LP tokens
      const collateralUserAccount = await getAssociatedTokenAddress(
        collateralMintPubkey,
        secondaryUser.publicKey,
        false, // Allow owner off curve
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
  
      // Create the collateral ATA if it doesn't exist
      const collateralATAInfo = await provider.connection.getAccountInfo(collateralUserAccount);
      if (!collateralATAInfo) {
        console.log("Collateral ATA does not exist. Creating...");
        const createCollateralATAIx = createAssociatedTokenAccountInstruction(
          secondaryUser.publicKey, // Payer
          collateralUserAccount,   // ATA
          secondaryUser.publicKey, // Owner
          collateralMintPubkey     // Mint
        );

        const tx = new Transaction().add(createCollateralATAIx);
        // Send the transaction signed by the secondary user
        await provider.sendAndConfirm(tx, [secondaryUser]);
        console.log("Collateral ATA created for Secondary User.");
      } else {
        console.log("Collateral ATA already exists for Secondary User.");
      }
      console.log("Collateral ATA for Secondary User:", collateralUserAccount.toBase58());
      const collateralATAInfoAfterCreation = await provider.connection.getAccountInfo(collateralUserAccount);
      console.log("Collateral ATA Info After Creation:", collateralATAInfoAfterCreation);

      // 5. Provide WSOL as Liquidity
      // Define the liquidity amount to deposit (e.g., 0.5 SOL)
      const liquidityAmount = new anchor.BN(0.5 * LAMPORTS_PER_SOL); // 0.5 SOL in lamports

      // Fetch the reserve account before refresh
      const reserveAccountBeforeRefresh = await program.account.reserve.fetch(reservePDA);
    
      // Check if the reserve is stale before refresh
      const isStaleBeforeRefresh = reserveAccountBeforeRefresh.lastUpdate.stale;
      console.log("Is reserve stale before refresh:", isStaleBeforeRefresh);

      // Refresh Reserve before depositing liquidity
      await program.methods
        .refreshReserve(
          true // is_test
        )
        .accounts({
          reserve: reservePDA,
          signer: provider.wallet.publicKey,
          mockPythFeed: reserveAccount.mockPythFeed,
      })
      .rpc();

      console.log("Called refresh_reserve before deposit");

      // Fetch the reserve account after refresh
      const reserveAccountAfterRefresh = await program.account.reserve.fetch(reservePDA);
    
      // Check if the reserve is stale after refresh
      const isStaleAfterRefresh = reserveAccountAfterRefresh.lastUpdate.stale;
      console.log("Is reserve stale after refresh:", isStaleAfterRefresh);

      // Assert that the stale status has changed (it should be false after refresh)
      assert.notEqual(isStaleBeforeRefresh, isStaleAfterRefresh, "Stale status should change after refresh");
      assert.isFalse(isStaleAfterRefresh, "Reserve should not be stale after refresh");

      // Call the deposit_reserve_liquidity instruction as the secondary user
      await program.methods
        .depositReserveLiquidity(
          liquidityAmount
        )
        .accounts({
          liquidityUserAccount: wsolTokenAccount,             // Secondary user's WSOL account
          collateralUserAccount: collateralUserAccount,       // Secondary user's Collateral (LP) Token account
          reserve: reservePDA,
          liquidityReserveAccount: reserveAccount.liquidity.supplyPubkey,
          collateralMintAccount: collateralMintPubkey,
          lendingMarket: lendingMarketPDA,
          signer: secondaryUser.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([secondaryUser]) // Include secondary user in signers
        .rpc();
  
      console.log("Deposit Reserve Liquidity Transaction successful.");
  
      // 6. Fetch and Assert Balances After Deposit
      // Fetch collateral token balance after deposit
      const collateralBalanceAfter = await provider.connection.getTokenAccountBalance(collateralUserAccount);
      const collateralBalanceAfterAmount = parseFloat(collateralBalanceAfter.value.uiAmountString || "0");
      console.log(`Collateral Token Balance after deposit: ${collateralBalanceAfterAmount} CTokens`);
  
      // Assert that collateral tokens have been minted
      assert.isAbove(
        collateralBalanceAfterAmount,
        0,
        "Collateral Token balance should be greater than 0 after deposit."
      );
  
      // Optionally, fetch reserve account and perform additional assertions
      const reserveAccountAfter = await program.account.reserve.fetch(reservePDA);
      console.log("Reserve Account After Deposit:", reserveAccountAfter);
  
      // Verify that the reserve's available liquidity has increased
      assert.equal(
        reserveAccountAfter.liquidity.availableAmount.toNumber(),
        reserveAccount.liquidity.availableAmount.toNumber() + liquidityAmount.toNumber(),
        "Reserve's available liquidity should increase by the deposited amount."
      );
  
      // Verify that 'stale' is set to true
      assert.isTrue(
        reserveAccountAfter.lastUpdate.stale,
        "Reserve last update should be marked as stale after deposit."
      );

      const wsolBalanceAfter = await provider.connection.getTokenAccountBalance(wsolTokenAccount);
      console.log("WSOL Balance After Deposit:", wsolBalanceAfter);
  
    } catch (error) {
      console.error("Error during deposit_reserve_liquidity as secondary user test:", error);
      throw error;
    }
  });

  xit("TODO-deposit_reserve_liquidity reverts as it reaches limit", async () => {
    //TODO: Implement this test
  }); 

  it("refresh_reserve updates liquidity.market_price when mock_pyth_feed price changes", async () => {
    // Fetch the reserve PDA
    const key = new anchor.BN(1);
    const keyBuffer = key.toArrayLike(Buffer, 'le', 8);
  
    const [reservePDA, reserveBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from("reserve"),
        keyBuffer,
        provider.wallet.publicKey.toBuffer(),
      ],
      program.programId
    );
  
    // Fetch the reserve account before the price update
    const reserveAccountBefore = await program.account.reserve.fetch(reservePDA);
  
    // Get the mock_pyth_feed address from the reserve
    const mockPythFeedPubkey = reserveAccountBefore.mockPythFeed;
  
    // Fetch the mock_pyth_feed account
    const mockPythFeedAccount = await program.account.mockPythPriceFeed.fetch(mockPythFeedPubkey);
  
    // Fetch the current price from mock_pyth_feed
    const currentPriceBefore = mockPythFeedAccount.price.toNumber();
    const currentExpoBefore = mockPythFeedAccount.expo;
  
    console.log(`MockPythFeed Price before update: ${currentPriceBefore}, Exponent: ${currentExpoBefore}`);
  
    // Update the price in mock_pyth_feed
    const newPrice = currentPriceBefore + (10 * LAMPORTS_PER_SOL); // Increase price by 10 SOL
    const newExpo = currentExpoBefore; // Keep the exponent the same
  
    // Call the update_mock_pyth_price instruction
    await program.methods
      .updateMockPythPrice(
        new anchor.BN(newPrice),
        newExpo
      )
      .accounts({
        mockPythFeed: mockPythFeedPubkey,
        signer: provider.wallet.publicKey,
      })
      .rpc();
  
    console.log(`Updated MockPythFeed Price to: ${newPrice}`);
  
    // Fetch the reserve account before refresh
    const reserveAccountBeforeRefresh = await program.account.reserve.fetch(reservePDA);
    const liquidityMarketPriceBefore = reserveAccountBeforeRefresh.liquidity.marketPrice.toString();
  
    console.log(`Reserve Liquidity Market Price before refresh: ${liquidityMarketPriceBefore}`);
  
    // Call handle_refresh_reserve instruction
    await program.methods
      .refreshReserve(
        true // is_test
      )
      .accounts({
        reserve: reservePDA,
        signer: provider.wallet.publicKey,
        mockPythFeed: mockPythFeedPubkey,
      })
      .rpc();
  
    console.log("Called refresh_reserve");
  
    // Fetch the reserve account after refresh
    const reserveAccountAfterRefresh = await program.account.reserve.fetch(reservePDA);
    const liquidityMarketPriceAfter = reserveAccountAfterRefresh.liquidity.marketPrice.toString();
  
    console.log(`Reserve Liquidity Market Price after refresh: ${liquidityMarketPriceAfter}`);
  
    // Assert that the liquidity.market_price has been updated to the new price
    assert.equal(
      liquidityMarketPriceAfter,
      newPrice.toString(),
      "Reserve liquidity.market_price should be updated to the new price"
    );
  });

  it("init_obligation", async () => {
    try {
      // 1) Initialize Lending Market (if not already initialized)
      const [lendingMarketPDA, lendingMarketBump] = await PublicKey.findProgramAddress(
        [provider.wallet.publicKey.toBuffer()],
        program.programId
      );
  
      // Check if the lending market account exists; if not, initialize it
      let lendingMarketAccount;
      try {
        lendingMarketAccount = await program.account.lendingMarket.fetch(lendingMarketPDA);
        console.log("Lending Market already initialized.");
      } catch (e) {
        // If not, stop
        console.log("Lending Market not initialized. Please initialize it first.");
        throw e;
      }
  
      // 2) Derive the Obligation PDA
      const key = new anchor.BN(1); // Using key = 1 for example
      const keyBuffer = key.toArrayLike(Buffer, "le", 8);
  
      const [obligationPDA, obligationBump] = await PublicKey.findProgramAddress(
        [Buffer.from("obligation"), keyBuffer, provider.wallet.publicKey.toBuffer()],
        program.programId
      );
  
      // 3) Call the init_obligation instruction
      await program.methods
        .initObligation(key)
        .accounts({
          obligation: obligationPDA,
          lendingMarket: lendingMarketPDA,
          signer: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .rpc();
  
      console.log("Obligation initialized.");
  
      // 4) Fetch the Obligation account and verify
      const obligationAccount = await program.account.obligation.fetch(obligationPDA);
  
      // 5) Assertions to verify that the obligation has been initialized correctly
      assert.strictEqual(obligationAccount.version, 1, "Incorrect version");
      assert.isTrue(
        obligationAccount.lendingMarket.equals(lendingMarketPDA),
        "Lending market mismatch"
      );
      assert.isTrue(
        obligationAccount.owner.equals(provider.wallet.publicKey),
        "Owner mismatch"
      );
      assert.strictEqual(obligationAccount.deposits.length, 0, "Deposits should be empty");
      assert.strictEqual(obligationAccount.borrows.length, 0, "Borrows should be empty");
      assert.strictEqual(
        obligationAccount.borrowedValue.toNumber(),
        0,
        "Borrowed value should be zero"
      );
      assert.strictEqual(
        obligationAccount.depositedValue.toNumber(),
        0,
        "Deposited value should be zero"
      );
      assert.isAbove(obligationAccount.lastUpdate.slot.toNumber(), 0, "Slot should be set");
  
      console.log("Obligation account verified.");
    } catch (error) {
      console.error("Error during init_obligation test:", error);
      throw error;
    }
  });

  it("deposit_obligation_collateral", async () => {
    try {
      // 1) Ensure that the necessary accounts are initialized.
  
      // Fetch the necessary accounts.
      const payer = provider.wallet.publicKey;
  
      // Get the lending market PDA.
      const [lendingMarketPDA, lendingMarketBump] = await PublicKey.findProgramAddress(
        [provider.wallet.publicKey.toBuffer()],
        program.programId
      );
  
      // Fetch the reserve PDA.
      const key = new anchor.BN(1);
      const keyBuffer = key.toArrayLike(Buffer, 'le', 8);
  
      const [reservePDA, reserveBump] = await PublicKey.findProgramAddress(
        [
          Buffer.from("reserve"),
          keyBuffer,
          provider.wallet.publicKey.toBuffer(),
        ],
        program.programId
      );
  
      // Fetch the obligation PDA.
      const [obligationPDA, obligationBump] = await PublicKey.findProgramAddress(
        [Buffer.from("obligation"), keyBuffer, provider.wallet.publicKey.toBuffer()],
        program.programId
      );
  
      // Fetch the reserve account.
      const reserveAccount = await program.account.reserve.fetch(reservePDA);
  
      // Fetch the obligation account.
      let obligationAccount;
      try {
        obligationAccount = await program.account.obligation.fetch(obligationPDA);
      } catch (e) {
        // Obligation not initialized; initialize it
        await program.methods
          .initObligation(key)
          .accounts({
            obligation: obligationPDA,
            lendingMarket: lendingMarketPDA,
            signer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          })
          .rpc();
  
        console.log("Obligation initialized.");
        obligationAccount = await program.account.obligation.fetch(obligationPDA);
      }
  
      // 2) Ensure that the user has cTokens (collateral tokens) in their account.
  
      // Collateral Mint Account
      const collateralMintPubkey = reserveAccount.collateral.mintPubkey;
  
      // Collateral User Account (the user's cToken account)
      const collateralUserAccount = await getAssociatedTokenAddress(
        collateralMintPubkey,
        provider.wallet.publicKey,
        false,
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
  
      // Check if the collateral user account exists, if not, create it.
      let collateralUserAccountInfo = await provider.connection.getAccountInfo(collateralUserAccount);
      if (!collateralUserAccountInfo) {
        // Create the associated token account for collateral tokens.
        const createCollateralUserAccountIx = createAssociatedTokenAccountInstruction(
          provider.wallet.publicKey, // Payer
          collateralUserAccount,     // Associated token account
          provider.wallet.publicKey, // Owner
          collateralMintPubkey       // Mint
        );
  
        const tx = new Transaction().add(createCollateralUserAccountIx);
        await provider.sendAndConfirm(tx, []);
      }
  
      // Ensure that the user has cTokens.
  
      // If the user has zero cTokens, we need to deposit liquidity to get cTokens.
      let collateralBalanceBefore = await provider.connection.getTokenAccountBalance(collateralUserAccount);
      let collateralBalanceBeforeAmount = parseFloat(collateralBalanceBefore.value.uiAmountString || "0");
  
      if (collateralBalanceBeforeAmount == 0) {
        // The user has no cTokens, deposit liquidity to get cTokens.
        const liquidityAmount = new anchor.BN(0.5 * LAMPORTS_PER_SOL); // 0.5 SOL in lamports
  
        // Liquidity User Account is the user's WSOL account.
        const liquidityMintPubkey = reserveAccount.liquidity.mintPubkey;
  
        const liquidityUserAccount = await getAssociatedTokenAddress(
          liquidityMintPubkey,
          provider.wallet.publicKey,
          false,
          TOKEN_PROGRAM_ID,
          ASSOCIATED_TOKEN_PROGRAM_ID
        );
  
        // Ensure liquidity user account exists
        let liquidityUserAccountInfo = await provider.connection.getAccountInfo(liquidityUserAccount);
        if (!liquidityUserAccountInfo) {
          // Wrap SOL into WSOL
          const wsolTokenAccount = await wrapSOL(provider, 0.5);
          // liquidityUserAccount is the wsolTokenAccount
        }
  
        // Deposit reserve liquidity to get cTokens.
        await program.methods
          .depositReserveLiquidity(
            liquidityAmount
          )
          .accounts({
            liquidityUserAccount: liquidityUserAccount,             // User's WSOL account
            collateralUserAccount: collateralUserAccount,           // User's Collateral (LP) Token account
            reserve: reservePDA,
            liquidityReserveAccount: reserveAccount.liquidity.supplyPubkey,
            collateralMintAccount: collateralMintPubkey,
            lendingMarket: lendingMarketPDA,
            signer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          })
          .rpc();
  
        console.log("Deposited reserve liquidity to obtain cTokens.");
      }
  
      // Fetch the collateral balance after ensuring the user has cTokens.
      collateralBalanceBefore = await provider.connection.getTokenAccountBalance(collateralUserAccount);
      collateralBalanceBeforeAmount = parseFloat(collateralBalanceBefore.value.uiAmountString || "0");
      console.log(`Collateral Token Balance after ensuring cTokens: ${collateralBalanceBeforeAmount} CTokens`);
  
      // 3) Set up the accounts needed for the deposit_obligation_collateral instruction.
  
      // Collateral Reserve Account (reserve's cToken account)
      const collateralReserveAccountPubkey = reserveAccount.collateral.supplyPubkey;
  
      // Ensure the collateral reserve account exists.
      let collateralReserveAccountInfo = await provider.connection.getAccountInfo(collateralReserveAccountPubkey);
      if (!collateralReserveAccountInfo) {
        console.error("Collateral Reserve Account does not exist.");
        throw new Error("Collateral Reserve Account does not exist.");
      }
  
      // 4) Determine the amount of cTokens to deposit into the obligation.
      const collateralAmountToDeposit = new anchor.BN(collateralBalanceBefore.value.amount).div(new anchor.BN(2)); // Deposit half
  
      if (collateralAmountToDeposit.lte(new anchor.BN(0))) {
        throw new Error("Insufficient collateral to deposit.");
      }
  
      console.log(`Depositing ${collateralAmountToDeposit.toString()} cTokens into the obligation.`);
  
      // 5) Call the deposit_obligation_collateral instruction.
      await program.methods
        .depositObligationCollateral(
          collateralAmountToDeposit
        )
        .accounts({
          collateralUserAccount: collateralUserAccount,
          collateralReserveAccount: collateralReserveAccountPubkey,
          depositReserve: reservePDA,
          obligation: obligationPDA,
          lendingMarket: lendingMarketPDA,
          signer: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .rpc();
  
      console.log("Deposited obligation collateral.");
  
      // 6) Verify the results.
  
      // Fetch the collateral balances after the deposit.
      const collateralBalanceAfterDeposit = await provider.connection.getTokenAccountBalance(collateralUserAccount);
      const collateralBalanceAfterDepositAmount = parseFloat(collateralBalanceAfterDeposit.value.uiAmountString || "0");
      console.log(`Collateral Token Balance after deposit: ${collateralBalanceAfterDepositAmount} CTokens`);
  
      // The user's collateral token balance should decrease by the deposited amount.
      const decimals = collateralBalanceAfterDeposit.value.decimals;
      const collateralAmountToDepositDecimal = collateralAmountToDeposit.toNumber() / Math.pow(10, decimals);
      const expectedCollateralBalanceAfterDeposit = collateralBalanceBeforeAmount - collateralAmountToDepositDecimal;
  
      assert.closeTo(
        collateralBalanceAfterDepositAmount,
        expectedCollateralBalanceAfterDeposit,
        0.0001,
        `Collateral Token balance should decrease by the deposited amount`
      );
  
      // Fetch the obligation account to verify the deposit.
      const obligationAccountAfterDeposit = await program.account.obligation.fetch(obligationPDA);
  
      // Verify that the obligation's deposits array includes the deposit.
      assert.strictEqual(
        obligationAccountAfterDeposit.deposits.length,
        1,
        "Obligation should have one deposit after depositObligationCollateral."
      );
  
      const depositEntry = obligationAccountAfterDeposit.deposits[0];
  
      if (depositEntry.depositReserve.toString() !== reservePDA.toString()) {
        console.error("Mismatch between deposit reserve and withdraw reserve");
        console.log("Deposit Reserve:", depositEntry.depositReserve.toString());
        console.log("Withdraw Reserve:", reservePDA.toString());
      }

      // Verify that the depositEntry's depositedAmount matches the deposited amount.
      assert.strictEqual(
        depositEntry.depositedAmount.toString(),
        collateralAmountToDeposit.toString(),
        "Obligation deposit amount should match the deposited amount."
      );
  
      // Verify that the reserve's collateral reserve account balance increased by the deposited amount.
      const collateralReserveBalanceAfter = await provider.connection.getTokenAccountBalance(collateralReserveAccountPubkey);
      const collateralReserveBalanceAfterAmount = parseFloat(collateralReserveBalanceAfter.value.uiAmountString || "0");
      console.log(`Collateral Reserve Account Balance after deposit: ${collateralReserveBalanceAfterAmount} CTokens`);
  
      // Since we didn't track the balance before, we can assume it's increased by the deposited amount.
      // For a more precise test, fetch the balance before the deposit and compare.
  
      console.log("Obligation deposit verified.");
    } catch (error) {
      console.error("Error during deposit_obligation_collateral test:", error);
      throw error;
    }
  });

  it("withdraw_obligation_collateral", async () => {
    try {
      // 1) Fetch necessary accounts and PDAs
  
      // Fetch the necessary accounts.
      const payer = provider.wallet.publicKey;
  
      // Get the lending market PDA.
      const [lendingMarketPDA, lendingMarketBump] = await PublicKey.findProgramAddress(
        [provider.wallet.publicKey.toBuffer()],
        program.programId
      );
  
      // Fetch the reserve PDA.
      const key = new anchor.BN(1);
      const keyBuffer = key.toArrayLike(Buffer, 'le', 8);
  
      const [reservePDA, reserveBump] = await PublicKey.findProgramAddress(
        [
          Buffer.from("reserve"),
          keyBuffer,
          provider.wallet.publicKey.toBuffer(),
        ],
        program.programId
      );
  
      // Fetch the obligation PDA.
      const [obligationPDA, obligationBump] = await PublicKey.findProgramAddress(
        [Buffer.from("obligation"), keyBuffer, provider.wallet.publicKey.toBuffer()],
        program.programId
      );
  
      // Fetch the reserve account.
      const reserveAccount = await program.account.reserve.fetch(reservePDA);
  
      // Fetch the obligation account.
      let obligationAccount;
      try {
        obligationAccount = await program.account.obligation.fetch(obligationPDA);
      } catch (e) {
        // Obligation not initialized; initialize it
        await program.methods
          .initObligation(key)
          .accounts({
            obligation: obligationPDA,
            lendingMarket: lendingMarketPDA,
            signer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          })
          .rpc();
  
        console.log("Obligation initialized.");
        obligationAccount = await program.account.obligation.fetch(obligationPDA);
      }
  
      // 2) Ensure that the obligation has some collateral deposited.
  
      // Collateral Mint Account
      const collateralMintPubkey = reserveAccount.collateral.mintPubkey;
  
      // Collateral User Account (the user's cToken account)
      const collateralUserAccount = await getAssociatedTokenAddress(
        collateralMintPubkey,
        provider.wallet.publicKey,
        false,
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
  
      // Collateral Reserve Account (reserve's cToken account)
      const collateralReserveAccountPubkey = reserveAccount.collateral.supplyPubkey;
  
      // Check if the obligation has deposits
      if (obligationAccount.deposits.length == 0) {
        console.log("Obligation has no collateral deposits. Depositing collateral...");
  
        // Ensure the user has cTokens (collateral tokens)
        // Similar to the previous test, we can deposit reserve liquidity to get cTokens
        // Check user's cToken balance
        let collateralBalanceBefore = await provider.connection.getTokenAccountBalance(collateralUserAccount);
        let collateralBalanceBeforeAmount = parseFloat(collateralBalanceBefore.value.uiAmountString || "0");
  
        if (collateralBalanceBeforeAmount == 0) {
          // The user has no cTokens, deposit liquidity to get cTokens.
          const liquidityAmount = new anchor.BN(0.5 * LAMPORTS_PER_SOL); // 0.5 SOL in lamports
  
          // Liquidity User Account is the user's WSOL account.
          const liquidityMintPubkey = reserveAccount.liquidity.mintPubkey;
  
          const liquidityUserAccount = await getAssociatedTokenAddress(
            liquidityMintPubkey,
            provider.wallet.publicKey,
            false,
            TOKEN_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
          );
  
          // Ensure liquidity user account exists
          let liquidityUserAccountInfo = await provider.connection.getAccountInfo(liquidityUserAccount);
          if (!liquidityUserAccountInfo) {
            // Wrap SOL into WSOL
            const wsolTokenAccount = await wrapSOL(provider, 0.5);
            // liquidityUserAccount is the wsolTokenAccount
          }
  
          // Deposit reserve liquidity to get cTokens.
          await program.methods
            .depositReserveLiquidity(
              liquidityAmount
            )
            .accounts({
              liquidityUserAccount: liquidityUserAccount,             // User's WSOL account
              collateralUserAccount: collateralUserAccount,           // User's Collateral (LP) Token account
              reserve: reservePDA,
              liquidityReserveAccount: reserveAccount.liquidity.supplyPubkey,
              collateralMintAccount: collateralMintPubkey,
              lendingMarket: lendingMarketPDA,
              signer: provider.wallet.publicKey,
              systemProgram: SystemProgram.programId,
              tokenProgram: TOKEN_PROGRAM_ID,
              rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            })
            .rpc();
  
          console.log("Deposited reserve liquidity to obtain cTokens.");
        }
  
        // Fetch the collateral balance after ensuring the user has cTokens.
        collateralBalanceBefore = await provider.connection.getTokenAccountBalance(collateralUserAccount);
        collateralBalanceBeforeAmount = parseFloat(collateralBalanceBefore.value.uiAmountString || "0");
        console.log(`Collateral Token Balance after ensuring cTokens: ${collateralBalanceBeforeAmount} CTokens`);
  
        // Now deposit collateral into the obligation
        const collateralAmountToDeposit = new anchor.BN(collateralBalanceBefore.value.amount).div(new anchor.BN(2)); // Deposit half
  
        if (collateralAmountToDeposit.lte(new anchor.BN(0))) {
          throw new Error("Insufficient collateral to deposit.");
        }
  
        console.log(`Depositing ${collateralAmountToDeposit.toString()} cTokens into the obligation.`);
  
        await program.methods
          .depositObligationCollateral(
            collateralAmountToDeposit
          )
          .accounts({
            collateralUserAccount: collateralUserAccount,
            collateralReserveAccount: collateralReserveAccountPubkey,
            depositReserve: reservePDA,
            obligation: obligationPDA,
            lendingMarket: lendingMarketPDA,
            signer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          })
          .rpc();
  
        console.log("Deposited obligation collateral.");
  
        // Fetch the obligation account after deposit
        obligationAccount = await program.account.obligation.fetch(obligationPDA);
      }
  
      // 3) Determine the amount of collateral to withdraw
  
      // Let's withdraw half of the collateral in the obligation
      const depositEntry = obligationAccount.deposits[0];
      const collateralAmountDeposited = depositEntry.depositedAmount;
  
      const collateralAmountToWithdraw = collateralAmountDeposited.div(new anchor.BN(2)); // Withdraw half
  
      if (collateralAmountToWithdraw.lte(new anchor.BN(0))) {
        throw new Error("Insufficient collateral to withdraw.");
      }
  
      console.log(`Withdrawing ${collateralAmountToWithdraw.toString()} cTokens from the obligation.`);
  
      // Fetch the collateral balance before withdrawal
      const collateralBalanceBeforeWithdraw = await provider.connection.getTokenAccountBalance(collateralUserAccount);
      const collateralBalanceBeforeWithdrawAmount = parseFloat(collateralBalanceBeforeWithdraw.value.uiAmountString || "0");
  
      // Fetch the reserve's collateral reserve account balance before withdrawal
      const collateralReserveBalanceBeforeWithdraw = await provider.connection.getTokenAccountBalance(collateralReserveAccountPubkey);
      const collateralReserveBalanceBeforeWithdrawAmount = parseFloat(collateralReserveBalanceBeforeWithdraw.value.uiAmountString || "0");
  
      // Fetch all deposit reserves
      const depositReserves = await Promise.all(
        obligationAccount.deposits.map(async (deposit) => {
          return deposit.depositReserve;
        })
      );

      console.log("Deposit Reserves:", depositReserves);
      console.log("Obligation state before withdrawal:");
      console.log("Deposits:", obligationAccount.deposits);
      console.log("Borrows:", obligationAccount.borrows);

      // Refresh reserve
      // await program.methods.refreshReserve().accounts({
      //   reserve: reservePDA,
      //   signer: provider.wallet.publicKey,
      //   mockPythFeed: reserveAccount.mock_pyth_feed,
      // }).rpc();

      // console.log("Refreshed reserve done");
      //Ideally later add refresh obligation

      // 4) Call the withdraw_obligation_collateral instruction
      await program.methods
      .withdrawObligationCollateral(
        collateralAmountToWithdraw
      )
      .accounts({
        collateralUserAccount: collateralUserAccount,
        collateralReserveAccount: collateralReserveAccountPubkey,
        withdrawReserve: reservePDA,
        obligation: obligationPDA,
        lendingMarket: lendingMarketPDA,
        signer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .remainingAccounts(
        depositReserves
        // .filter(reservePDA => !reservePDA.equals(reservePDA)) // Exclude withdrawReserve if necessary
        .map(reservePDA => ({
          pubkey: reservePDA,
          isWritable: true,
          isSigner: false
        }))
      )
      .rpc();

      console.log("Withdrawn obligation collateral.");
      
      // 5) Verify the results
  
      // Fetch the collateral balance after withdrawal
      const collateralBalanceAfterWithdraw = await provider.connection.getTokenAccountBalance(collateralUserAccount);
      const collateralBalanceAfterWithdrawAmount = parseFloat(collateralBalanceAfterWithdraw.value.uiAmountString || "0");
      console.log(`Collateral Token Balance after withdrawal: ${collateralBalanceAfterWithdrawAmount} CTokens`);
  
      // The user's collateral token balance should increase by the withdrawn amount.
      const decimals = collateralBalanceAfterWithdraw.value.decimals;
      const collateralAmountToWithdrawDecimal = collateralAmountToWithdraw.toNumber() / Math.pow(10, decimals);
      const expectedCollateralBalanceAfterWithdraw = collateralBalanceBeforeWithdrawAmount + collateralAmountToWithdrawDecimal;
  
      assert.closeTo(
        collateralBalanceAfterWithdrawAmount,
        expectedCollateralBalanceAfterWithdraw,
        0.0001,
        `Collateral Token balance should increase by the withdrawn amount`
      );
  
      // Fetch the obligation account to verify the withdrawal.
      const obligationAccountAfterWithdraw = await program.account.obligation.fetch(obligationPDA);
  
      // Verify that the obligation's deposits array has updated the depositedAmount
      const depositEntryAfter = obligationAccountAfterWithdraw.deposits[0];
      const expectedDepositedAmount = collateralAmountDeposited.sub(collateralAmountToWithdraw);
      assert.strictEqual(
        depositEntryAfter.depositedAmount.toString(),
        expectedDepositedAmount.toString(),
        "Obligation depositedAmount should decrease by the withdrawn amount."
      );
  
      // Verify that the reserve's collateral reserve account balance decreased by the withdrawn amount.
      const collateralReserveBalanceAfterWithdraw = await provider.connection.getTokenAccountBalance(collateralReserveAccountPubkey);
      const collateralReserveBalanceAfterWithdrawAmount = parseFloat(collateralReserveBalanceAfterWithdraw.value.uiAmountString || "0");
      console.log(`Collateral Reserve Account Balance after withdrawal: ${collateralReserveBalanceAfterWithdrawAmount} CTokens`);
  
      const expectedCollateralReserveBalanceAfterWithdraw = collateralReserveBalanceBeforeWithdrawAmount - collateralAmountToWithdrawDecimal;
  
      assert.closeTo(
        collateralReserveBalanceAfterWithdrawAmount,
        expectedCollateralReserveBalanceAfterWithdraw,
        0.0001,
        `Collateral Reserve Account balance should decrease by the withdrawn amount`
      );
  
      console.log("Obligation withdrawal verified.");
    } catch (error) {
      console.error("Error during withdraw_obligation_collateral test:", error);
      throw error;
    }
  });
});