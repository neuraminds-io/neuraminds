//! Cryptographic primitives for Polyguard Privacy Layer
//!
//! This module provides:
//! - ElGamal encryption (twisted variant on Ristretto255)
//! - Pedersen commitments for hiding values
//! - Range proofs for proving values are in valid range
//! - Balance proofs for proving sufficient funds

pub mod elgamal;
pub mod pedersen;
pub mod proofs;
pub mod errors;

pub use elgamal::*;
pub use pedersen::*;
pub use proofs::*;
pub use errors::*;
