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

// Load wallet
const walletPath = process.env.ANCHOR_WALLET || "/Users/dennisgoslar/.config/solana/id.json";
const walletKeypair = Keypair.fromSecretKey(
  Uint8Array.from(JSON.parse(fs.readFileSync(walletPath, "utf8")))
);

// Test markets to create
const testMarkets = [
  {
    id: "trump-2028",
    question: "Will Trump win the 2028 presidential election?",
    description: "Resolves YES if Donald Trump wins the 2028 US presidential election.",
    category: "Politics",
  },
  {
    id: "eth-10k-2025",
    question: "Will ETH reach $10,000 by end of 2025?",
    description: "Resolves YES if Ethereum trades above $10,000 USD on any major exchange before December 31, 2025.",
    category: "Crypto",
  },
  {
    id: "fed-rate-cut-q1",
    question: "Will the Fed cut rates in Q1 2026?",
    description: "Resolves YES if the Federal Reserve announces a rate cut during Q1 2026.",
    category: "Economics",
  },
  {
    id: "superbowl-chiefs",
    question: "Will the Chiefs win Super Bowl LX?",
    description: "Resolves YES if the Kansas City Chiefs win Super Bowl LX.",
    category: "Sports",
  },
  {
    id: "openai-ipo-2026",
    question: "Will OpenAI IPO in 2026?",
    description: "Resolves YES if OpenAI completes an initial public offering by December 31, 2026.",
    category: "Companies",
  },
  {
    id: "tesla-fsd-approved",
    question: "Will Tesla FSD get full approval in 2026?",
    description: "Resolves YES if Tesla Full Self-Driving receives regulatory approval for unsupervised driving in any US state.",
    category: "Tech & Science",
  },
  {
    id: "sp500-6000",
    question: "Will S&P 500 reach 6,000 by June 2026?",
    description: "Resolves YES if the S&P 500 index closes above 6,000 before June 30, 2026.",
    category: "Financials",
  },
  {
    id: "taylor-swift-tour",
    question: "Will Taylor Swift announce new tour in 2026?",
    description: "Resolves YES if Taylor Swift officially announces a new concert tour for 2026 or 2027.",
    category: "Culture",
  },
  {
    id: "paris-climate-goals",
    question: "Will any G7 nation meet 2030 Paris goals early?",
    description: "Resolves YES if any G7 nation officially reports meeting their Paris Agreement 2030 emissions targets before 2028.",
    category: "Climate",
  },
];

async function main() {
  const rpcUrl = process.env.RPC_URL || "http://localhost:8899";
  const networkName = rpcUrl.includes("localhost") ? "localnet" : "devnet";
  console.log(`Creating ${testMarkets.length} test markets on ${networkName}...\n`);

  const connection = new Connection(rpcUrl, "confirmed");
  const wallet = new anchor.Wallet(walletKeypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  console.log("Wallet:", wallet.publicKey.toBase58());
  const balance = await connection.getBalance(wallet.publicKey);
  console.log("Balance:", balance / LAMPORTS_PER_SOL, "SOL\n");

  const marketProgram = new Program(marketIdl as anchor.Idl, provider);

  // Get oracle registry PDA
  const [oracleRegistryPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("oracle_registry")],
    MARKET_PROGRAM_ID
  );

  // Generate a single oracle for all markets
  const oracle = Keypair.generate();

  // Ensure oracle is registered
  try {
    console.log("Ensuring oracle is registered...");
    await marketProgram.methods
      .manageOracle(oracle.publicKey, { add: {} })
      .accounts({
        authority: wallet.publicKey,
        registry: oracleRegistryPda,
      })
      .rpc();
    console.log("Oracle registered:", oracle.publicKey.toBase58());
  } catch (e: any) {
    if (e.message.includes("already") || e.message.includes("OracleAlreadyRegistered")) {
      console.log("Oracle already registered");
    } else {
      console.log("Oracle registration:", e.message);
    }
  }

  // Create a shared collateral mint (USDC-like)
  console.log("\nCreating collateral mint...");
  const collateralMint = await createMint(
    connection,
    walletKeypair,
    wallet.publicKey,
    null,
    6
  );
  console.log("Collateral mint:", collateralMint.toBase58());

  // Mint test USDC to wallet
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
    10_000_000_000 // 10,000 USDC
  );
  console.log("Minted 10,000 test USDC\n");

  // Create each market
  const createdMarkets: any[] = [];

  for (const market of testMarkets) {
    const marketId = `${market.id}-${Date.now()}`;
    const now = Math.floor(Date.now() / 1000);

    // Vary trading end times
    const daysUntilEnd = Math.floor(Math.random() * 365) + 30;
    const tradingEnd = now + 86400 * daysUntilEnd;
    const resolutionDeadline = tradingEnd + 86400 * 30;

    // Random fee between 50-200 bps
    const feeBps = Math.floor(Math.random() * 150) + 50;

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

    const protocolTreasury = Keypair.generate();

    try {
      console.log(`Creating market: ${market.question.substring(0, 50)}...`);

      await marketProgram.methods
        .createMarket(
          marketId,
          market.question,
          market.description,
          market.category,
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

      createdMarkets.push({
        id: marketId,
        pda: marketPda.toBase58(),
        question: market.question,
        category: market.category,
      });

      console.log(`   Created: ${marketPda.toBase58()}`);
    } catch (e: any) {
      console.log(`   Error: ${e.message.substring(0, 80)}`);
    }
  }

  console.log("\n=== Summary ===");
  console.log(`Created ${createdMarkets.length} markets:\n`);

  for (const m of createdMarkets) {
    console.log(`${m.category}: ${m.question.substring(0, 45)}...`);
    console.log(`   PDA: ${m.pda}\n`);
  }

  console.log("Collateral Mint:", collateralMint.toBase58());
  console.log("\nDone!");
}

main().catch(console.error);
