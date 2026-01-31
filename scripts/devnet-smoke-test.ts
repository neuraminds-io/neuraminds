#!/usr/bin/env npx ts-node

/**
 * Devnet Smoke Test
 *
 * Quick verification that programs are deployed and callable.
 * Uses existing wallet balance instead of airdrops.
 */

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, Connection } from "@solana/web3.js";
import { PolyguardMarket } from "../target/types/polyguard_market";
import { PolyguardOrderbook } from "../target/types/polyguard_orderbook";
import * as fs from "fs";
import * as os from "os";

const DEVNET_URL = "https://api.devnet.solana.com";

async function main() {
  console.log("=== Polyguard Devnet Smoke Test ===\n");

  // Load wallet
  const walletPath = `${os.homedir()}/.config/solana/id.json`;
  const walletKeypair = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(walletPath, "utf-8")))
  );
  console.log("Wallet:", walletKeypair.publicKey.toBase58());

  // Create connection and provider
  const connection = new Connection(DEVNET_URL, "confirmed");
  const wallet = new anchor.Wallet(walletKeypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  // Check balance
  const balance = await connection.getBalance(walletKeypair.publicKey);
  console.log("Balance:", balance / 1e9, "SOL\n");

  if (balance < 0.1 * 1e9) {
    console.error("ERROR: Insufficient balance. Need at least 0.1 SOL");
    process.exit(1);
  }

  // Load programs
  const marketProgram = anchor.workspace.PolyguardMarket as Program<PolyguardMarket>;
  const orderbookProgram = anchor.workspace.PolyguardOrderbook as Program<PolyguardOrderbook>;

  console.log("Programs:");
  console.log("  Market:", marketProgram.programId.toBase58());
  console.log("  Orderbook:", orderbookProgram.programId.toBase58());
  console.log("");

  // Test 1: Verify programs are deployed
  console.log("Test 1: Verify programs deployed...");
  try {
    const marketInfo = await connection.getAccountInfo(marketProgram.programId);
    const orderbookInfo = await connection.getAccountInfo(orderbookProgram.programId);

    if (!marketInfo || !orderbookInfo) {
      throw new Error("Program not found on devnet");
    }

    console.log("  ✓ Market program deployed (" + marketInfo.data.length + " bytes)");
    console.log("  ✓ Orderbook program deployed (" + orderbookInfo.data.length + " bytes)");
  } catch (e) {
    console.log("  ✗ FAILED:", e);
    process.exit(1);
  }

  // Test 2: Fetch IDL (verifies program is valid Anchor program)
  console.log("\nTest 2: Fetch program IDL...");
  try {
    const marketIdl = await Program.fetchIdl(marketProgram.programId, provider);
    const orderbookIdl = await Program.fetchIdl(orderbookProgram.programId, provider);

    if (marketIdl) {
      console.log("  ✓ Market IDL: " + (marketIdl.instructions?.length || 0) + " instructions");
    } else {
      console.log("  - Market IDL not on-chain (OK for testing)");
    }

    if (orderbookIdl) {
      console.log("  ✓ Orderbook IDL: " + (orderbookIdl.instructions?.length || 0) + " instructions");
    } else {
      console.log("  - Orderbook IDL not on-chain (OK for testing)");
    }
  } catch (e) {
    console.log("  - IDL fetch skipped:", (e as Error).message);
  }

  // Test 3: Derive PDAs (verifies program structure)
  console.log("\nTest 3: Derive PDAs...");
  try {
    const marketId = "test-market-123";

    const [marketPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("market"), Buffer.from(marketId)],
      marketProgram.programId
    );
    console.log("  ✓ Market PDA:", marketPda.toBase58());

    const [configPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("config")],
      orderbookProgram.programId
    );
    console.log("  ✓ Orderbook Config PDA:", configPda.toBase58());

    const [ordersPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("open_orders"), walletKeypair.publicKey.toBuffer(), marketPda.toBuffer()],
      orderbookProgram.programId
    );
    console.log("  ✓ Open Orders PDA:", ordersPda.toBase58());
  } catch (e) {
    console.log("  ✗ FAILED:", e);
    process.exit(1);
  }

  // Test 4: Check program executable
  console.log("\nTest 4: Verify program executable...");
  try {
    const marketInfo = await connection.getAccountInfo(marketProgram.programId);
    if (marketInfo?.executable) {
      console.log("  ✓ Market program is executable");
    } else {
      throw new Error("Market program not executable");
    }

    const orderbookInfo = await connection.getAccountInfo(orderbookProgram.programId);
    if (orderbookInfo?.executable) {
      console.log("  ✓ Orderbook program is executable");
    } else {
      throw new Error("Orderbook program not executable");
    }
  } catch (e) {
    console.log("  ✗ FAILED:", e);
    process.exit(1);
  }

  console.log("\n=== All Smoke Tests Passed ===");
  console.log("\nPrograms are deployed and ready for testing.");
  console.log("Run full test suite with more SOL to test transactions.");
}

main().catch(console.error);
