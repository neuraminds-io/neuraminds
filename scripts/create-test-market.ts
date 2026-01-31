import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram, Connection, LAMPORTS_PER_SOL } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import * as fs from "fs";
import * as path from "path";

// Load IDL
const marketIdlPath = path.join(__dirname, "../target/idl/polyguard_market.json");
const marketIdl = JSON.parse(fs.readFileSync(marketIdlPath, "utf8"));

// Program ID
const MARKET_PROGRAM_ID = new PublicKey("98jqxMe88XGjXzCY3bwV1Kuqzj32fcwdhPZa193RUffQ");

// Load wallet - use the default Solana keypair which has program upgrade authority
const walletPath = process.env.ANCHOR_WALLET ||
  "/Users/dennisgoslar/.config/solana/id.json";
const walletKeypair = Keypair.fromSecretKey(
  Uint8Array.from(JSON.parse(fs.readFileSync(walletPath, "utf8")))
);

async function main() {
  const rpcUrl = process.env.RPC_URL || "http://localhost:8899";
  const networkName = rpcUrl.includes("localhost") ? "localnet" : "devnet";
  console.log(`Creating test market on ${networkName}...\n`);

  const connection = new Connection(rpcUrl, "confirmed");
  const wallet = new anchor.Wallet(walletKeypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  console.log("Wallet:", wallet.publicKey.toBase58());
  console.log("Balance:", (await connection.getBalance(wallet.publicKey)) / LAMPORTS_PER_SOL, "SOL\n");

  // In Anchor v0.30+, the IDL contains the address, just pass provider
  const marketProgram = new Program(marketIdl as anchor.Idl, provider);

  // Generate keypairs for oracle and treasury
  const oracle = Keypair.generate();
  const protocolTreasury = Keypair.generate();

  // Market parameters
  const marketId = "btc-100k-2026-" + Date.now();
  const question = "Will BTC reach $100,000 by end of 2026?";
  const description = "Bitcoin price prediction market. Resolves YES if BTC trades above $100,000 USD on any major exchange before December 31, 2026 23:59 UTC.";
  const category = "Crypto";
  const feeBps = 100; // 1% fee

  const now = Math.floor(Date.now() / 1000);
  const tradingEnd = now + 86400 * 365; // 1 year
  const resolutionDeadline = now + 86400 * 400; // 400 days

  // Derive PDAs
  const [oracleRegistryPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("oracle_registry")],
    MARKET_PROGRAM_ID
  );

  const [marketPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("market"), Buffer.from(marketId)],
    MARKET_PROGRAM_ID
  );

  const [yesMintPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("yes_mint"), marketPda.toBuffer()],
    MARKET_PROGRAM_ID
  );

  const [noMintPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("no_mint"), marketPda.toBuffer()],
    MARKET_PROGRAM_ID
  );

  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), marketPda.toBuffer()],
    MARKET_PROGRAM_ID
  );

  console.log("Oracle Registry PDA:", oracleRegistryPda.toBase58());
  console.log("Market PDA:", marketPda.toBase58());

  // Step 1: Initialize oracle registry (if not already done)
  try {
    console.log("\n1. Initializing oracle registry...");
    const tx = await marketProgram.methods
      .initializeOracleRegistry(true)
      .accounts({
        authority: wallet.publicKey,
        registry: oracleRegistryPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("   Oracle registry initialized:", tx);
  } catch (e: any) {
    if (e.message.includes("already in use")) {
      console.log("   Oracle registry already exists");
    } else {
      console.log("   Error:", e.message);
    }
  }

  // Step 2: Add oracle to registry
  try {
    console.log("\n2. Adding oracle to registry...");
    const tx = await marketProgram.methods
      .manageOracle(oracle.publicKey, { add: {} })
      .accounts({
        authority: wallet.publicKey,
        registry: oracleRegistryPda,
      })
      .rpc();
    console.log("   Oracle added:", tx);
  } catch (e: any) {
    if (e.message.includes("already") || e.message.includes("OracleAlreadyRegistered")) {
      console.log("   Oracle already in registry");
    } else {
      console.log("   Error:", e.message);
    }
  }

  // Step 3: Create USDC mint (for devnet testing)
  console.log("\n3. Creating collateral mint...");
  const collateralMint = await createMint(
    connection,
    walletKeypair,
    wallet.publicKey,
    null,
    6 // USDC has 6 decimals
  );
  console.log("   Collateral mint:", collateralMint.toBase58());

  // Step 4: Create market
  try {
    console.log("\n4. Creating market...");
    const tx = await marketProgram.methods
      .createMarket(
        marketId,
        question,
        description,
        category,
        new BN(resolutionDeadline),
        new BN(tradingEnd),
        feeBps
      )
      .accounts({
        authority: wallet.publicKey,
        oracle: oracle.publicKey,
        oracleRegistry: oracleRegistryPda,
        market: marketPda,
        collateralMint: collateralMint,
        yesMint: yesMintPda,
        noMint: noMintPda,
        vault: vaultPda,
        protocolTreasury: protocolTreasury.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("   Market created:", tx);
    console.log("\n=== Market Details ===");
    console.log("Market ID:", marketId);
    console.log("Market PDA:", marketPda.toBase58());
    console.log("Question:", question);
    console.log("Category:", category);
    console.log("Collateral Mint:", collateralMint.toBase58());
    console.log("YES Mint:", yesMintPda.toBase58());
    console.log("NO Mint:", noMintPda.toBase58());
    console.log("Vault:", vaultPda.toBase58());
    console.log("=====================\n");
  } catch (e: any) {
    console.log("   Error creating market:", e.message);
  }

  // Step 5: Mint some collateral to wallet for testing
  try {
    console.log("5. Minting test collateral...");
    const userCollateralAta = await getOrCreateAssociatedTokenAccount(
      connection,
      walletKeypair,
      collateralMint,
      wallet.publicKey
    );

    await mintTo(
      connection,
      walletKeypair,
      collateralMint,
      userCollateralAta.address,
      walletKeypair,
      1_000_000_000 // 1000 USDC
    );
    console.log("   Minted 1000 test USDC to:", userCollateralAta.address.toBase58());
  } catch (e: any) {
    console.log("   Error minting collateral:", e.message);
  }

  console.log("\nDone! Market is live on devnet.");
}

main().catch(console.error);
