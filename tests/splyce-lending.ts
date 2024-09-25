import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SplyceLending } from "../target/types/splyce_lending";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";

describe("splyce-lending", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SplyceLending as Program<SplyceLending>;

  it("Init_lending_market", async () => {

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
});