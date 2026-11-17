//! Decode pump.fun bonding-curve accounts and quote trades against them with exact integer math.
//!
//! - [`BondingCurve::decode`] reads the account bytes you get from `getAccountInfo` (every layout length pump.fun
//!   has written, from the original 49 bytes to the current 166).
//! - [`BondingCurve::buy_exact_sol_in`], [`BondingCurve::buy_exact_tokens_out`] and
//!   [`BondingCurve::sell_exact_tokens_in`] quote trades in `u128` with checked arithmetic, rounding the way the
//!   program does. Fees are a parameter ([`Fees`]); nothing is hardcoded.
//! - [`max_sol_cost`], [`min_sol_output`] and [`min_tokens_out`] turn a quote into the slippage limits the `buy` and
//!   `sell` instructions take.
//! - [`bonding_curve_address`] derives a coin's curve account from its mint, offline, with no dependencies (a small
//!   SHA-256 and ed25519 point check are built in).
//!
//! No dependencies, no `unsafe`, `no_std` without the default `std` feature. No network: fetch the account bytes
//! with whatever RPC client you already use.
//!
//! ```
//! use pump_curve::{BondingCurve, Fees, max_sol_cost};
//!
//! # let data = pump_curve::BondingCurve::fresh(pump_curve::Pubkey::new([1; 32])).to_bytes();
//! // `data` is the bonding-curve account's bytes (getAccountInfo, base64-decoded).
//! let curve = BondingCurve::decode(&data)?;
//!
//! // Fees are passed in. 95 bps protocol + 30 bps creator was pump.fun's published bonding-curve rate when this
//! // was written; read the current one from pump.fun (see `Fees`).
//! let fees = Fees::new(95, 30);
//!
//! let quote = curve.buy_exact_sol_in(1_000_000_000, fees)?; // spend 1 SOL, fees included
//! let limit = max_sol_cost(quote.total_sol, 100)?; // 1% slippage
//! assert!(quote.total_sol <= 1_000_000_000 && limit >= quote.total_sol);
//! println!("{} tokens, {:.2}% of the curve sold", quote.tokens_out, curve.progress_pct().unwrap());
//! # Ok::<(), pump_curve::Error>(())
//! ```
//!
//! A small, open piece of what powers [Blix](https://blixvip.com), the Solana memecoin terminal. Not affiliated with
//! pump.fun.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod curve;
mod ed25519;
mod error;
mod pubkey;
mod quote;
mod sha256;

pub use curve::BondingCurve;
pub use error::Error;
pub use pubkey::{Pubkey, MAX_SEEDS, MAX_SEED_LEN};
pub use quote::{
    max_sol_cost, min_sol_output, min_tokens_out, select_fee_tier, BuyQuote, FeeTier, Fees, SellQuote, BPS_DENOMINATOR,
};

/// The pump.fun program: `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`.
pub const PUMP_PROGRAM_ID: Pubkey = Pubkey([
    1, 86, 224, 246, 147, 102, 90, 207, 68, 219, 21, 104, 191, 23, 91, 170, 81, 137, 203, 151, 245, 210, 255, 59, 101,
    93, 43, 182, 253, 109, 24, 176,
]);

/// The wrapped-SOL mint, `So11111111111111111111111111111111111111112`. A curve whose `quote_mint` is this (or unset)
/// is priced in SOL.
pub const WSOL_MINT: Pubkey = Pubkey([
    6, 155, 136, 87, 254, 171, 129, 132, 251, 104, 127, 99, 70, 24, 192, 53, 218, 196, 57, 220, 26, 235, 59, 85, 152,
    160, 240, 0, 0, 0, 0, 1,
]);

/// The Anchor discriminator that starts every `BondingCurve` account: `sha256("account:BondingCurve")[..8]`, as
/// listed in pump.fun's published IDL.
pub const BONDING_CURVE_DISCRIMINATOR: [u8; 8] = [23, 183, 248, 55, 96, 216, 172, 96];

/// Lamports in one SOL.
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
/// Decimals of a pump.fun coin. Token amounts in this crate are raw units: 1 token = 10^6.
pub const TOKEN_DECIMALS: u8 = 6;
/// Raw token units in one whole token.
pub const TOKEN_UNIT: u64 = 1_000_000;

/// `Global.initial_virtual_token_reserves` as documented by pump.fun (1,073,000,000 tokens). Standard (non-mayhem)
/// curves start here. Source: <https://github.com/pump-fun/pump-public-docs/blob/main/docs/PUMP_PROGRAM_README.md>
pub const INITIAL_VIRTUAL_TOKEN_RESERVES: u64 = 1_073_000_000_000_000;
/// `Global.initial_virtual_sol_reserves` as documented by pump.fun (30 SOL).
pub const INITIAL_VIRTUAL_SOL_RESERVES: u64 = 30_000_000_000;
/// `Global.initial_real_token_reserves` as documented by pump.fun (793,100,000 tokens: what the curve can sell).
pub const INITIAL_REAL_TOKEN_RESERVES: u64 = 793_100_000_000_000;
/// `Global.token_total_supply` as documented by pump.fun (1,000,000,000 tokens).
pub const TOKEN_TOTAL_SUPPLY: u64 = 1_000_000_000_000_000;

/// Derives a coin's bonding-curve account from its mint: the program address for seeds `["bonding-curve", mint]`
/// under [`PUMP_PROGRAM_ID`]. Returns the address and its bump. Pure: no network.
///
/// ```
/// use pump_curve::{bonding_curve_address, Pubkey};
///
/// let mint: Pubkey = "DxYWLXgKJpQmfhTG7ikK6oX6oDhxMhN5wCs5BJekpump".parse()?;
/// let (curve, bump) = bonding_curve_address(&mint)?;
/// assert_eq!(curve.to_string(), "6FrLSW5z5uNP3ovpFYUQCoY5d7sXF1mEMsFvvQdeu8dF");
/// assert_eq!(bump, 254);
/// # Ok::<(), pump_curve::Error>(())
/// ```
pub fn bonding_curve_address(mint: &Pubkey) -> Result<(Pubkey, u8), Error> {
    Pubkey::find_program_address(&[b"bonding-curve", &mint.0], &PUMP_PROGRAM_ID)
}

/// Runs the README's Rust snippets as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_addresses_match_their_base58() {
        assert_eq!(Pubkey::from_base58("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"), Ok(PUMP_PROGRAM_ID));
        assert_eq!(Pubkey::from_base58("So11111111111111111111111111111111111111112"), Ok(WSOL_MINT));
    }
}
