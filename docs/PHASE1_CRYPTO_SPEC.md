# Phase 1: Cryptographic Implementation Specification

> **Priority:** CRITICAL
> **Duration:** 4 weeks
> **Blocking:** All privacy features

---

## Overview

This document provides detailed technical specifications for implementing production-grade cryptography in the Polyguard privacy program. The current placeholder implementations must be completely replaced.

---

## 1. ElGamal Encryption

### 1.1 Mathematical Foundation

**Twisted ElGamal on Curve25519:**

We use twisted ElGamal which is additively homomorphic, allowing encrypted balances to be updated without decryption.

**Key Generation:**
```
Private key: s ← random scalar in Z_q
Public key: P = s * G (where G is the base point)
```

**Encryption of amount m:**
```
Random scalar: r ← random in Z_q
Ciphertext: (C₁, C₂) where:
  C₁ = r * G
  C₂ = m * G + r * P
```

**Decryption:**
```
m * G = C₂ - s * C₁
Then solve discrete log for small m (using lookup table for amounts < 2^40)
```

**Homomorphic Addition:**
```
(C₁, C₂) + (C₁', C₂') = (C₁ + C₁', C₂ + C₂')
This encrypts m + m'
```

### 1.2 Implementation Structure

```rust
// programs/polyguard-privacy/src/crypto/elgamal.rs

use curve25519_dalek::{
    ristretto::{CompressedRistretto, RistrettoPoint},
    scalar::Scalar,
};

/// ElGamal public key (32 bytes compressed)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElGamalPubkey(pub CompressedRistretto);

/// ElGamal secret key (32 bytes)
#[derive(Clone, Copy, Debug)]
pub struct ElGamalSecretKey(pub Scalar);

/// ElGamal keypair
#[derive(Clone, Copy, Debug)]
pub struct ElGamalKeypair {
    pub public: ElGamalPubkey,
    pub secret: ElGamalSecretKey,
}

/// ElGamal ciphertext (64 bytes: two compressed points)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElGamalCiphertext {
    pub c1: CompressedRistretto,  // r * G
    pub c2: CompressedRistretto,  // m * G + r * P
}

impl ElGamalKeypair {
    /// Generate a new random keypair
    pub fn new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let secret = Scalar::random(rng);
        let public = &secret * &RISTRETTO_BASEPOINT_TABLE;
        Self {
            public: ElGamalPubkey(public.compress()),
            secret: ElGamalSecretKey(secret),
        }
    }

    /// Generate keypair from seed (deterministic)
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let secret = Scalar::from_bytes_mod_order(*seed);
        let public = &secret * &RISTRETTO_BASEPOINT_TABLE;
        Self {
            public: ElGamalPubkey(public.compress()),
            secret: ElGamalSecretKey(secret),
        }
    }
}

impl ElGamalPubkey {
    /// Encrypt an amount
    pub fn encrypt<R: RngCore + CryptoRng>(
        &self,
        amount: u64,
        rng: &mut R,
    ) -> ElGamalCiphertext {
        let r = Scalar::random(rng);
        let amount_scalar = Scalar::from(amount);

        let pubkey_point = self.0.decompress()
            .expect("Invalid public key");

        let c1 = &r * &RISTRETTO_BASEPOINT_TABLE;
        let c2 = &amount_scalar * &RISTRETTO_BASEPOINT_TABLE + &r * pubkey_point;

        ElGamalCiphertext {
            c1: c1.compress(),
            c2: c2.compress(),
        }
    }

    /// Encrypt with specific randomness (for proofs)
    pub fn encrypt_with_randomness(
        &self,
        amount: u64,
        randomness: &Scalar,
    ) -> ElGamalCiphertext {
        let amount_scalar = Scalar::from(amount);
        let pubkey_point = self.0.decompress()
            .expect("Invalid public key");

        let c1 = randomness * &RISTRETTO_BASEPOINT_TABLE;
        let c2 = &amount_scalar * &RISTRETTO_BASEPOINT_TABLE + randomness * pubkey_point;

        ElGamalCiphertext {
            c1: c1.compress(),
            c2: c2.compress(),
        }
    }
}

impl ElGamalSecretKey {
    /// Decrypt a ciphertext (returns None if amount > MAX_AMOUNT)
    pub fn decrypt(&self, ciphertext: &ElGamalCiphertext) -> Option<u64> {
        let c1 = ciphertext.c1.decompress()?;
        let c2 = ciphertext.c2.decompress()?;

        // m * G = c2 - s * c1
        let m_point = c2 - &self.0 * c1;

        // Solve discrete log using precomputed table
        discrete_log_lookup(&m_point)
    }
}

impl ElGamalCiphertext {
    /// Homomorphic addition of two ciphertexts
    pub fn add(&self, other: &ElGamalCiphertext) -> Self {
        let c1_sum = self.c1.decompress().unwrap()
            + other.c1.decompress().unwrap();
        let c2_sum = self.c2.decompress().unwrap()
            + other.c2.decompress().unwrap();

        Self {
            c1: c1_sum.compress(),
            c2: c2_sum.compress(),
        }
    }

    /// Homomorphic subtraction
    pub fn subtract(&self, other: &ElGamalCiphertext) -> Self {
        let c1_diff = self.c1.decompress().unwrap()
            - other.c1.decompress().unwrap();
        let c2_diff = self.c2.decompress().unwrap()
            - other.c2.decompress().unwrap();

        Self {
            c1: c1_diff.compress(),
            c2: c2_diff.compress(),
        }
    }

    /// Serialize to 64 bytes
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        bytes[0..32].copy_from_slice(self.c1.as_bytes());
        bytes[32..64].copy_from_slice(self.c2.as_bytes());
        bytes
    }

    /// Deserialize from 64 bytes
    pub fn from_bytes(bytes: &[u8; 64]) -> Option<Self> {
        let c1 = CompressedRistretto::from_slice(&bytes[0..32]).ok()?;
        let c2 = CompressedRistretto::from_slice(&bytes[32..64]).ok()?;
        Some(Self { c1, c2 })
    }
}

/// Precomputed discrete log table for amounts 0 to MAX_AMOUNT
const MAX_AMOUNT: u64 = 1 << 40;  // ~1 trillion (enough for any token amount)

lazy_static! {
    static ref DL_TABLE: HashMap<CompressedRistretto, u64> = {
        // Build lookup table at startup
        // For efficiency, use baby-step giant-step or batch lookup
        build_discrete_log_table(MAX_AMOUNT)
    };
}

fn discrete_log_lookup(point: &RistrettoPoint) -> Option<u64> {
    DL_TABLE.get(&point.compress()).copied()
}
```

### 1.3 On-Chain Storage

```rust
// Updated PrivateAccount state
#[account]
#[derive(InitSpace)]
pub struct PrivateAccount {
    pub owner: Pubkey,
    pub elgamal_pubkey: [u8; 32],           // Compressed Ristretto point
    pub encrypted_balance: [u8; 64],         // ElGamalCiphertext
    pub pending_balance: [u8; 64],           // For async operations
    pub created_at: i64,
    pub last_activity: i64,
    pub bump: u8,
}
```

---

## 2. Pedersen Commitments

### 2.1 Mathematical Foundation

**Pedersen Commitment:**
```
Setup: G, H are two random generators (H derived from G via hash)
Commit(m, r): C = m * G + r * H
```

**Properties:**
- **Hiding:** C reveals nothing about m
- **Binding:** Cannot find m', r' such that m'*G + r'*H = m*G + r*H

### 2.2 Implementation

```rust
// programs/polyguard-privacy/src/crypto/pedersen.rs

use curve25519_dalek::{
    ristretto::{CompressedRistretto, RistrettoPoint},
    scalar::Scalar,
};

/// Pedersen commitment (32 bytes)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PedersenCommitment(pub CompressedRistretto);

/// Opening for a Pedersen commitment
#[derive(Clone, Copy, Debug)]
pub struct PedersenOpening {
    pub value: u64,
    pub blinding: Scalar,
}

lazy_static! {
    /// Second generator H = hash_to_point("polyguard_pedersen_h")
    static ref H: RistrettoPoint = {
        RistrettoPoint::hash_from_bytes::<Sha512>(
            b"polyguard_pedersen_h"
        )
    };
}

impl PedersenCommitment {
    /// Create a commitment to a value
    pub fn commit<R: RngCore + CryptoRng>(
        value: u64,
        rng: &mut R,
    ) -> (Self, PedersenOpening) {
        let blinding = Scalar::random(rng);
        let commitment = Self::commit_with_blinding(value, &blinding);
        let opening = PedersenOpening { value, blinding };
        (commitment, opening)
    }

    /// Create commitment with specific blinding factor
    pub fn commit_with_blinding(value: u64, blinding: &Scalar) -> Self {
        let value_scalar = Scalar::from(value);
        let point = &value_scalar * &RISTRETTO_BASEPOINT_TABLE
            + blinding * &*H;
        Self(point.compress())
    }

    /// Verify a commitment opening
    pub fn verify(&self, opening: &PedersenOpening) -> bool {
        let expected = Self::commit_with_blinding(
            opening.value,
            &opening.blinding,
        );
        self.0 == expected.0
    }

    /// Homomorphic addition
    pub fn add(&self, other: &PedersenCommitment) -> Self {
        let sum = self.0.decompress().unwrap()
            + other.0.decompress().unwrap();
        Self(sum.compress())
    }

    /// Homomorphic subtraction
    pub fn subtract(&self, other: &PedersenCommitment) -> Self {
        let diff = self.0.decompress().unwrap()
            - other.0.decompress().unwrap();
        Self(diff.compress())
    }
}

impl PedersenOpening {
    /// Add two openings
    pub fn add(&self, other: &PedersenOpening) -> Self {
        Self {
            value: self.value + other.value,
            blinding: self.blinding + other.blinding,
        }
    }
}
```

---

## 3. Bulletproofs (Range Proofs)

### 3.1 Overview

Bulletproofs prove that a committed value is in range [0, 2^n) without revealing the value.

**Properties:**
- Proof size: O(log n) - about 672 bytes for 64-bit range
- Verification: O(n) group operations
- No trusted setup required

### 3.2 Implementation

```rust
// programs/polyguard-privacy/src/crypto/proofs.rs

use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use merlin::Transcript;

/// Range proof for a committed value
#[derive(Clone, Debug)]
pub struct PolyguardRangeProof {
    pub proof: RangeProof,
    pub commitment: PedersenCommitment,
}

lazy_static! {
    static ref BP_GENS: BulletproofGens = BulletproofGens::new(64, 1);
    static ref PC_GENS: PedersenGens = PedersenGens::default();
}

impl PolyguardRangeProof {
    /// Create a range proof for value in [0, 2^64)
    pub fn prove<R: RngCore + CryptoRng>(
        value: u64,
        blinding: &Scalar,
        rng: &mut R,
    ) -> Result<Self, ProofError> {
        let mut transcript = Transcript::new(b"polyguard_range_proof");

        let (proof, commitment) = RangeProof::prove_single(
            &*BP_GENS,
            &*PC_GENS,
            &mut transcript,
            value,
            blinding,
            64,  // 64-bit range
        )?;

        Ok(Self {
            proof,
            commitment: PedersenCommitment(commitment.compress()),
        })
    }

    /// Verify a range proof
    pub fn verify(&self) -> Result<(), ProofError> {
        let mut transcript = Transcript::new(b"polyguard_range_proof");

        let commitment = self.commitment.0.decompress()
            .ok_or(ProofError::InvalidCommitment)?;

        self.proof.verify_single(
            &*BP_GENS,
            &*PC_GENS,
            &mut transcript,
            &commitment,
            64,
        )
    }

    /// Serialize to bytes (for on-chain storage)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.commitment.0.as_bytes());
        bytes.extend_from_slice(&self.proof.to_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProofError> {
        if bytes.len() < 32 {
            return Err(ProofError::InvalidFormat);
        }

        let commitment = CompressedRistretto::from_slice(&bytes[0..32])
            .map_err(|_| ProofError::InvalidCommitment)?;
        let proof = RangeProof::from_bytes(&bytes[32..])
            .map_err(|_| ProofError::InvalidProof)?;

        Ok(Self {
            proof,
            commitment: PedersenCommitment(commitment),
        })
    }
}
```

### 3.3 Balance Proof

Proves that encrypted balance >= amount to withdraw:

```rust
/// Proof that balance >= amount (without revealing either)
#[derive(Clone, Debug)]
pub struct BalanceProof {
    /// Range proof that (balance - amount) >= 0
    pub range_proof: PolyguardRangeProof,
    /// Commitment to (balance - amount)
    pub difference_commitment: PedersenCommitment,
}

impl BalanceProof {
    /// Create proof that balance >= amount
    pub fn prove<R: RngCore + CryptoRng>(
        balance: u64,
        amount: u64,
        balance_blinding: &Scalar,
        rng: &mut R,
    ) -> Result<Self, ProofError> {
        if balance < amount {
            return Err(ProofError::InsufficientBalance);
        }

        let difference = balance - amount;
        let diff_blinding = Scalar::random(rng);

        // Prove difference is in valid range (i.e., >= 0)
        let range_proof = PolyguardRangeProof::prove(
            difference,
            &diff_blinding,
            rng,
        )?;

        Ok(Self {
            range_proof,
            difference_commitment: range_proof.commitment,
        })
    }

    /// Verify balance proof given commitments
    pub fn verify(
        &self,
        balance_commitment: &PedersenCommitment,
        amount_commitment: &PedersenCommitment,
    ) -> Result<(), ProofError> {
        // Verify range proof
        self.range_proof.verify()?;

        // Verify that difference_commitment = balance_commitment - amount_commitment
        let expected_diff = balance_commitment.subtract(amount_commitment);
        if expected_diff.0 != self.difference_commitment.0 {
            return Err(ProofError::CommitmentMismatch);
        }

        Ok(())
    }
}
```

---

## 4. Updated Privacy Program Instructions

### 4.1 Create Private Account

```rust
pub fn handler(
    ctx: Context<CreatePrivateAccount>,
    elgamal_pubkey: [u8; 32],
) -> Result<()> {
    // Validate ElGamal public key is a valid curve point
    let pubkey = CompressedRistretto::from_slice(&elgamal_pubkey)
        .map_err(|_| PrivacyError::InvalidElGamalKey)?;
    pubkey.decompress()
        .ok_or(PrivacyError::InvalidElGamalKey)?;

    let private_account = &mut ctx.accounts.private_account;
    private_account.owner = ctx.accounts.owner.key();
    private_account.elgamal_pubkey = elgamal_pubkey;

    // Initialize balance to encryption of 0
    let zero_ciphertext = ElGamalCiphertext::encrypt_zero(&elgamal_pubkey)?;
    private_account.encrypted_balance = zero_ciphertext.to_bytes();
    private_account.pending_balance = [0u8; 64];

    // ... rest of initialization
    Ok(())
}
```

### 4.2 Private Deposit

```rust
pub fn handler(
    ctx: Context<PrivateDeposit>,
    amount: u64,
    encrypted_amount: [u8; 64],
    deposit_proof: Vec<u8>,
) -> Result<()> {
    // Verify the encrypted_amount actually encrypts `amount`
    // This requires a proof that encrypted_amount opens to amount
    let proof = DepositProof::from_bytes(&deposit_proof)
        .map_err(|_| PrivacyError::InvalidProof)?;

    let ciphertext = ElGamalCiphertext::from_bytes(&encrypted_amount)
        .ok_or(PrivacyError::InvalidCiphertext)?;

    proof.verify(amount, &ciphertext, &ctx.accounts.private_account.elgamal_pubkey)?;

    // Transfer collateral to vault
    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_collateral.to_account_info(),
            to: ctx.accounts.privacy_vault.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        },
    );
    token::transfer(transfer_ctx, amount)?;

    // Homomorphically add to encrypted balance
    let current_balance = ElGamalCiphertext::from_bytes(
        &ctx.accounts.private_account.encrypted_balance
    ).ok_or(PrivacyError::InvalidCiphertext)?;

    let new_balance = current_balance.add(&ciphertext);
    ctx.accounts.private_account.encrypted_balance = new_balance.to_bytes();

    Ok(())
}
```

### 4.3 Private Withdraw

```rust
pub fn handler(
    ctx: Context<PrivateWithdraw>,
    amount: u64,
    balance_proof: Vec<u8>,
) -> Result<()> {
    // Parse and verify balance proof
    let proof = BalanceProof::from_bytes(&balance_proof)
        .map_err(|_| PrivacyError::InvalidProof)?;

    // Get current balance commitment (derived from ciphertext)
    let balance_ciphertext = ElGamalCiphertext::from_bytes(
        &ctx.accounts.private_account.encrypted_balance
    ).ok_or(PrivacyError::InvalidCiphertext)?;

    let balance_commitment = balance_ciphertext.to_commitment();

    // Create commitment to withdrawal amount
    let amount_commitment = PedersenCommitment::commit_with_blinding(
        amount,
        &Scalar::zero(),  // Known amount, no hiding needed
    );

    // Verify proof that balance >= amount
    proof.verify(&balance_commitment, &amount_commitment)
        .map_err(|_| PrivacyError::InvalidBalanceProof)?;

    // Subtract amount from encrypted balance
    let amount_ciphertext = ElGamalPubkey::from_bytes(
        &ctx.accounts.private_account.elgamal_pubkey
    )?.encrypt_with_randomness(amount, &Scalar::zero());

    let new_balance = balance_ciphertext.subtract(&amount_ciphertext);
    ctx.accounts.private_account.encrypted_balance = new_balance.to_bytes();

    // Transfer collateral from vault
    let seeds = &[
        b"privacy_vault_authority".as_ref(),
        &[ctx.bumps.vault_authority],
    ];
    let signer_seeds = &[&seeds[..]];

    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.privacy_vault.to_account_info(),
            to: ctx.accounts.user_collateral.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(transfer_ctx, amount)?;

    Ok(())
}
```

---

## 5. Solana Compute Budget Considerations

### 5.1 Estimated Compute Costs

| Operation | Estimated CU |
|-----------|-------------|
| ElGamal encrypt | 50,000 |
| ElGamal decrypt | 50,000 |
| Pedersen commit | 25,000 |
| Range proof verify | 200,000 |
| Balance proof verify | 250,000 |

**Solana Limit:** 1,400,000 CU per transaction (with priority fee)

### 5.2 Optimization Strategies

1. **Batch verification:** Combine multiple proofs for amortized cost
2. **Precomputation:** Store frequently used values
3. **Proof aggregation:** Aggregate multiple range proofs
4. **Off-chain computation:** Generate proofs client-side

### 5.3 Transaction Splitting

For complex operations exceeding compute budget:

```rust
// Split into multiple transactions
// Transaction 1: Submit proof to temporary account
// Transaction 2: Verify proof and update state
```

---

## 6. Testing Requirements

### 6.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_elgamal_encrypt_decrypt() {
        let mut rng = rand::thread_rng();
        let keypair = ElGamalKeypair::new(&mut rng);

        for amount in [0, 1, 100, 1_000_000, u64::MAX / 2] {
            let ciphertext = keypair.public.encrypt(amount, &mut rng);
            let decrypted = keypair.secret.decrypt(&ciphertext);
            assert_eq!(decrypted, Some(amount));
        }
    }

    #[test]
    fn test_elgamal_homomorphic_addition() {
        let mut rng = rand::thread_rng();
        let keypair = ElGamalKeypair::new(&mut rng);

        let a = 1000;
        let b = 2000;

        let c1 = keypair.public.encrypt(a, &mut rng);
        let c2 = keypair.public.encrypt(b, &mut rng);
        let sum = c1.add(&c2);

        assert_eq!(keypair.secret.decrypt(&sum), Some(a + b));
    }

    #[test]
    fn test_range_proof_valid() {
        let mut rng = rand::thread_rng();
        let value = 42u64;
        let blinding = Scalar::random(&mut rng);

        let proof = PolyguardRangeProof::prove(value, &blinding, &mut rng)
            .unwrap();
        assert!(proof.verify().is_ok());
    }

    #[test]
    fn test_balance_proof_sufficient() {
        let mut rng = rand::thread_rng();
        let balance = 1000;
        let amount = 500;
        let blinding = Scalar::random(&mut rng);

        let proof = BalanceProof::prove(balance, amount, &blinding, &mut rng)
            .unwrap();

        let balance_commitment = PedersenCommitment::commit_with_blinding(balance, &blinding);
        let amount_commitment = PedersenCommitment::commit_with_blinding(amount, &Scalar::zero());

        assert!(proof.verify(&balance_commitment, &amount_commitment).is_ok());
    }

    #[test]
    fn test_balance_proof_insufficient() {
        let mut rng = rand::thread_rng();
        let balance = 500;
        let amount = 1000;
        let blinding = Scalar::random(&mut rng);

        let result = BalanceProof::prove(balance, amount, &blinding, &mut rng);
        assert!(result.is_err());
    }
}
```

### 6.2 Integration Tests

```typescript
// tests/privacy-crypto.ts
describe("Privacy Cryptography", () => {
    it("encrypts and decrypts balance correctly", async () => {
        // Generate keypair client-side
        const keypair = ElGamalKeypair.generate();

        // Create private account
        await program.methods
            .createPrivateAccount(Array.from(keypair.publicKey))
            .accounts({...})
            .rpc();

        // Deposit with proof
        const amount = 1000;
        const { ciphertext, proof } = keypair.encryptWithProof(amount);

        await program.methods
            .privateDeposit(new BN(amount), Array.from(ciphertext), Array.from(proof))
            .accounts({...})
            .rpc();

        // Verify balance updated
        const account = await program.account.privateAccount.fetch(accountPda);
        const decrypted = keypair.decrypt(account.encryptedBalance);
        expect(decrypted).to.equal(amount);
    });

    it("prevents withdrawal exceeding balance", async () => {
        // Deposit 1000
        await deposit(1000);

        // Try to withdraw 2000 with forged proof
        const forgedProof = generateForgedBalanceProof(2000);

        try {
            await program.methods
                .privateWithdraw(new BN(2000), Array.from(forgedProof))
                .accounts({...})
                .rpc();
            expect.fail("Should have thrown");
        } catch (e) {
            expect(e.message).to.include("InvalidBalanceProof");
        }
    });
});
```

---

## 7. Client SDK

### 7.1 TypeScript SDK

```typescript
// sdk/src/crypto/elgamal.ts
import { ristretto255 } from '@noble/curves/ed25519';

export class ElGamalKeypair {
    private secretKey: Uint8Array;
    public publicKey: Uint8Array;

    static generate(): ElGamalKeypair {
        const secretKey = ristretto255.utils.randomPrivateKey();
        const publicKey = ristretto255.getPublicKey(secretKey);
        return new ElGamalKeypair(secretKey, publicKey);
    }

    encrypt(amount: bigint): ElGamalCiphertext {
        const r = ristretto255.utils.randomPrivateKey();
        const rG = ristretto255.getPublicKey(r);
        const rP = ristretto255.scalarMult(this.publicKey, r);
        const mG = ristretto255.scalarMultBase(amount);
        const c2 = ristretto255.add(mG, rP);

        return new ElGamalCiphertext(rG, c2);
    }

    decrypt(ciphertext: ElGamalCiphertext): bigint {
        const sC1 = ristretto255.scalarMult(ciphertext.c1, this.secretKey);
        const mG = ristretto255.subtract(ciphertext.c2, sC1);
        return discreteLog(mG);
    }
}
```

---

## 8. Security Considerations

### 8.1 Randomness

- Use cryptographically secure RNG for all operations
- Never reuse randomness across encryptions
- Implement deterministic derivation from seed for reproducibility in tests

### 8.2 Side Channels

- Use constant-time operations for scalar multiplication
- Avoid branching on secret values
- Use timing-safe comparison functions

### 8.3 Key Management

- Secret keys should never be stored on-chain
- Implement secure key derivation from wallet signature
- Support hardware wallet integration

---

## 9. Migration Path

### 9.1 State Migration

Existing accounts with placeholder data need migration:

```rust
pub fn migrate_private_account(
    ctx: Context<MigratePrivateAccount>,
    new_elgamal_pubkey: [u8; 32],
) -> Result<()> {
    // Only allow migration for accounts with zero balance
    // (placeholder encryption != real encryption)

    // Reset to zero with proper encryption
    let zero_ciphertext = ElGamalCiphertext::encrypt_zero(&new_elgamal_pubkey)?;
    ctx.accounts.private_account.encrypted_balance = zero_ciphertext.to_bytes();
    ctx.accounts.private_account.elgamal_pubkey = new_elgamal_pubkey;

    Ok(())
}
```

### 9.2 Version Tracking

```rust
#[account]
pub struct PrivateAccount {
    pub version: u8,  // 0 = placeholder, 1 = real crypto
    // ...
}
```

---

*Specification Version: 1.0*
*Author: Security Team*
*Review Required: Cryptography Expert*
