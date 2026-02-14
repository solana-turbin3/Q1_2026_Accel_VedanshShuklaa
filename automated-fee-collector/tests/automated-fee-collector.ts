/**
 * Comprehensive Test Suite for Automated Fee Collector
 * 
 * Tests all functionality in order:
 * 1. Mint initialization with transfer fees
 * 2. Treasury setup
 * 3. Token transfers (generating fees)
 * 4. Manual fee collection
 * 5. Tuktuk scheduling
 */

import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import {
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { expect } from "chai";
import { FeeCollectorClient } from "./client";

describe("automated-fee-collector", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AutomatedFeeCollector as Program;
  let client: FeeCollectorClient;
  let authority: Keypair;
  let mint: Keypair;
  let treasury: PublicKey;
  let alice: Keypair;
  let bob: Keypair;

  before(async () => {
    client = new FeeCollectorClient(program, provider, null);
    authority = Keypair.generate();
    alice = Keypair.generate();
    bob = Keypair.generate();

    // Airdrop SOL to all test accounts
    for (const account of [authority, alice, bob]) {
      const sig = await provider.connection.requestAirdrop(
        account.publicKey,
        5 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig);
    }
  });

  describe("Initialization", () => {
    it("Creates a mint with transfer fees", async () => {
      const result = await client.initializeMint(
        authority,
        100, // 1% fee
        1_000_000 // Max fee
      );
      
      mint = result.mint;
      
      console.log("✅ Mint created:", mint.publicKey.toString());
      expect(mint).to.exist;
    });

    it("Initializes the treasury", async () => {
      treasury = await client.initializeTreasury(authority, mint.publicKey);
      
      const treasuryAccount = await getAccount(
        provider.connection,
        treasury,
        undefined,
        TOKEN_2022_PROGRAM_ID
      );
      
      console.log("✅ Treasury created:", treasury.toString());
      expect(treasuryAccount.owner.toString()).to.equal(authority.publicKey.toString());
    });

    it("Mints tokens to Alice for testing", async () => {
      const aliceTokenAccount = getAssociatedTokenAddressSync(
        mint.publicKey,
        alice.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      // Create Alice's token account first (if needed)
      try {
        await getAccount(
          provider.connection,
          aliceTokenAccount,
          undefined,
          TOKEN_2022_PROGRAM_ID
        );
      } catch {
        await program.methods
          .initTreasury() // Reuse the same instruction pattern
          .accounts({
            authority: alice.publicKey,
            treasury: aliceTokenAccount,
            mint: mint.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .signers([alice])
          .rpc();
      }

      // Mint 1,000,000 tokens to Alice
      await mintTo(
        provider.connection,
        authority, // Payer
        mint.publicKey,
        aliceTokenAccount,
        authority, // Mint authority
        1_000_000,
        [],
        undefined,
        TOKEN_2022_PROGRAM_ID
      );

      const account = await getAccount(
        provider.connection,
        aliceTokenAccount,
        undefined,
        TOKEN_2022_PROGRAM_ID
      );

      console.log("✅ Minted 1,000,000 tokens to Alice");
      expect(account.amount.toString()).to.equal("1000000");
    });
  });

  describe("Transfer with Fees", () => {
    it("Transfers tokens from Alice to Bob with fees deducted", async () => {
      const transferAmount = 100_000;
      const expectedFee = 1_000; // 1% of 100,000

      const tx = await client.testTransfer(
        alice,
        bob.publicKey,
        mint.publicKey,
        transferAmount
      );

      const bobTokenAccount = getAssociatedTokenAddressSync(
        mint.publicKey,
        bob.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      const bobAccount = await getAccount(
        provider.connection,
        bobTokenAccount,
        undefined,
        TOKEN_2022_PROGRAM_ID
      );

      // Bob should receive transferAmount - fee
      const expectedReceived = transferAmount - expectedFee;
      
      console.log(`✅ Bob received: ${bobAccount.amount} (expected: ${expectedReceived})`);
      console.log(`✅ Fee withheld: ${expectedFee}`);
      
      // Note: The fee is withheld on Bob's account
      expect(Number(bobAccount.amount)).to.be.closeTo(expectedReceived, 100);
    });

    it("Makes multiple transfers to accumulate fees", async () => {
      // Transfer 3 more times to accumulate more fees
      for (let i = 0; i < 3; i++) {
        await client.testTransfer(
          alice,
          bob.publicKey,
          mint.publicKey,
          50_000
        );
        
        // Wait a bit between transfers
        await new Promise(resolve => setTimeout(resolve, 1000));
      }

      console.log("✅ Made 3 additional transfers, fees accumulating...");
    });
  });

  describe("Fee Collection", () => {
    it("Manually collects fees to treasury", async () => {
      const bobTokenAccount = getAssociatedTokenAddressSync(
        mint.publicKey,
        bob.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      // Check treasury balance before collection
      const treasuryBefore = await getAccount(
        provider.connection,
        treasury,
        undefined,
        TOKEN_2022_PROGRAM_ID
      );

      console.log("Treasury balance before:", treasuryBefore.amount.toString());

      // Collect fees from Bob's account
      await client.manualCollect(
        authority,
        mint.publicKey,
        treasury,
        [bobTokenAccount] // Source accounts to harvest from
      );

      // Check treasury balance after collection
      const treasuryAfter = await getAccount(
        provider.connection,
        treasury,
        undefined,
        TOKEN_2022_PROGRAM_ID
      );

      console.log("Treasury balance after:", treasuryAfter.amount.toString());
      
      const collectedFees = Number(treasuryAfter.amount) - Number(treasuryBefore.amount);
      console.log(`✅ Collected ${collectedFees} in fees`);
      
      expect(collectedFees).to.be.greaterThan(0);
    });
  });

  describe("Tuktuk Scheduling", () => {
    it("Initializes task queue", async () => {
      const { taskQueue, queueAuthority } = await client.initializeTaskQueue(authority);
      
      console.log("✅ Task queue initialized");
      console.log("   Queue:", taskQueue.toString());
      console.log("   Authority:", queueAuthority.toString());
      
      expect(taskQueue).to.exist;
      expect(queueAuthority).to.exist;
    });

    it("Schedules recurring fee collection", async () => {
      const { taskQueue } = await client.initializeTaskQueue(authority);
      
      const tx = await client.scheduleFeeCollection(
        authority,
        mint.publicKey,
        treasury,
        taskQueue,
        1, // Task ID
        "hourly"
      );

      console.log("✅ Scheduled hourly fee collection");
      console.log("   Transaction:", tx);
      
      expect(tx).to.exist;
    });
  });

  describe("Fee Updates", () => {
    it("Updates transfer fee parameters", async () => {
      const newFeeBasisPoints = 200; // 2%
      const newMaxFee = 2_000_000;

      await program.methods
        .updateFee(newFeeBasisPoints, new BN(newMaxFee))
        .accounts({
          authority: authority.publicKey,
          mintAccount: mint.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .signers([authority])
        .rpc();

      console.log("✅ Updated fees to 2% (takes effect in 2 epochs)");
    });
  });
});

/**
 * Run this test suite with:
 * anchor test
 * 
 * or for more verbose output:
 * anchor test -- --features=test-bpf
 */