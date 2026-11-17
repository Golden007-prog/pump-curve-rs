//! Checks against real mainnet data, read through the public RPC https://api.mainnet-beta.solana.com on 2026-11-17.
//!
//! Account fixtures (`getAccountInfo`, base64):
//! - `curve_sol_active.b64`: curve BTCS3Ym9ysQkbSQH69wGCajRq4CoVkL4fKEbTqq1BWs3 of mint
//!   HRtcoHqcFCBrKST5fb6uRgFtRbDVvbiLz5k46auspump, SOL-quoted and trading, slot 470678916.
//! - `curve_migrated.b64`: curve 6FrLSW5z5uNP3ovpFYUQCoY5d7sXF1mEMsFvvQdeu8dF of mint
//!   DxYWLXgKJpQmfhTG7ikK6oX6oDhxMhN5wCs5BJekpump (the JS pump-curve README example), completed, slot 470678721.
//! - `curve_other_quote.b64`: curve CDrBQR7JoZzadXJEjwTRLmywceP7wXdD2jGRwNr6gcso of mint
//!   9H5izLf79ie2r39Pa7cv98rmVZyDoi7ngnJqqfxnpump, quoted in another token, slot 470678834.
//!
//! Trade fixtures: `trade_events.json` holds nine pump.fun `TradeEvent`s (non-mayhem, SOL-quoted) decoded from the
//! program's event instructions in `getTransaction` results; `trades.rs` is the same table in Rust.

use pump_curve::{bonding_curve_address, BondingCurve, Fees, Pubkey, PUMP_PROGRAM_ID, WSOL_MINT};

mod fixtures {
    include!("fixtures/trades.rs");
}

fn b64(text: &str) -> Vec<u8> {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0);
    for &c in text.trim().as_bytes() {
        if c == b'=' {
            break;
        }
        acc = (acc << 6) | A.iter().position(|&a| a == c).expect("base64") as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

fn key(s: &str) -> Pubkey {
    s.parse().unwrap()
}

#[test]
fn curve_addresses_match_the_chain() {
    for (mint, curve, bump) in [
        ("DxYWLXgKJpQmfhTG7ikK6oX6oDhxMhN5wCs5BJekpump", "6FrLSW5z5uNP3ovpFYUQCoY5d7sXF1mEMsFvvQdeu8dF", 254),
        ("9H5izLf79ie2r39Pa7cv98rmVZyDoi7ngnJqqfxnpump", "CDrBQR7JoZzadXJEjwTRLmywceP7wXdD2jGRwNr6gcso", 254),
        ("HRtcoHqcFCBrKST5fb6uRgFtRbDVvbiLz5k46auspump", "BTCS3Ym9ysQkbSQH69wGCajRq4CoVkL4fKEbTqq1BWs3", 255),
    ] {
        assert_eq!(bonding_curve_address(&key(mint)).unwrap(), (key(curve), bump), "{mint}");
    }
    let (global, _) = Pubkey::find_program_address(&[b"global"], &PUMP_PROGRAM_ID).unwrap();
    assert_eq!(global, key("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf"));
}

#[test]
fn wallets_are_on_the_curve_and_program_addresses_are_not() {
    assert!(key("BsU7rYPX4UmefGdTpCQyLPCoVSh5L3WkdeKbU3oBg4CG").is_on_curve());
    // Traders that signed the fixture trades.
    assert!(key("3YjDWCdHQqpYmksUFHjrKdaS8V7r8pqfRuUiryHKrS1x").is_on_curve());
    assert!(key("3qvuM3U5UZPVagL1spxmgVgUvzT3h49gyJZyagxRtqKr").is_on_curve());
    assert!(!key("6FrLSW5z5uNP3ovpFYUQCoY5d7sXF1mEMsFvvQdeu8dF").is_on_curve());
    assert!(!key("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf").is_on_curve());
    // A coin "creator" is not always a wallet: this one is a program address (a fee-sharing config).
    assert!(!key("HWZHpQLpjra7K1uN1urtfYBxbHZ3XehcEhPutMDGaahi").is_on_curve());
}

mod on_curve {
    include!("fixtures/on_curve.rs");
}

/// Differential check against the JS pump-curve's BigInt implementation on 256 random values.
#[test]
fn on_curve_agrees_with_the_js_implementation() {
    let mut on = 0;
    for &(hex, expected) in on_curve::ON_CURVE {
        let mut b = [0u8; 32];
        for (i, byte) in b.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap();
        }
        assert_eq!(Pubkey::new(b).is_on_curve(), expected, "{hex}");
        on += expected as u32;
    }
    assert!(on > 64 && on < 192, "a mix of both answers");
}

#[test]
fn active_sol_curve_from_mainnet() {
    let data = b64(include_str!("fixtures/curve_sol_active.b64"));
    assert_eq!(data.len(), 166);
    let c = BondingCurve::decode(&data).unwrap();
    assert!(!c.complete);
    assert!(c.is_sol_quoted());
    assert!(!c.is_mayhem_mode);
    assert_eq!(c.token_total_supply, 1_000_000_000_000_000);
    assert_eq!(c.initial_virtual_quote_reserves, 30_000_000_000);
    // The standard curve's invariant: virtual minus real tokens never changes.
    assert_eq!(c.virtual_token_reserves - c.real_token_reserves, 279_900_000_000_000);
    assert_eq!(c.virtual_sol_reserves - c.real_sol_reserves, 30_000_000_000);
    // The JS pump-curve reads the same account as 101.92 SOL of market cap and 64.43% sold.
    assert!((c.market_cap_sol().unwrap() - 101.9155).abs() < 0.001);
    assert_eq!(c.progress_bps(), Some(6443));
    assert!(c.creator.is_some());
    // Quotes work on it.
    let fees = Fees::new(95, 30);
    let q = c.buy_exact_sol_in(100_000_000, fees).unwrap();
    assert!(q.tokens_out > 0 && q.total_sol <= 100_000_000);
}

#[test]
fn migrated_curve_from_mainnet() {
    let data = b64(include_str!("fixtures/curve_migrated.b64"));
    let c = BondingCurve::decode(&data).unwrap();
    assert!(c.complete);
    assert!(c.is_migrated());
    assert_eq!(c.creator, Some(key("BsU7rYPX4UmefGdTpCQyLPCoVSh5L3WkdeKbU3oBg4CG")));
    assert_eq!(c.price_sol(), None);
    assert_eq!(c.progress_bps(), Some(10_000));
    assert_eq!(c.buy_exact_sol_in(1_000_000_000, Fees::ZERO), Err(pump_curve::Error::CurveComplete));
}

#[test]
fn other_quote_curve_from_mainnet() {
    let data = b64(include_str!("fixtures/curve_other_quote.b64"));
    let c = BondingCurve::decode(&data).unwrap();
    assert!(!c.complete);
    assert!(!c.is_sol_quoted());
    assert_ne!(c.quote_mint, Some(WSOL_MINT));
    assert_eq!(c.quote_mint, Some(key("TTWofwAge91oFhZs7kpQdyrVRkmevgM88xijGvQFbKo")));
    assert_eq!(c.creator, Some(key("HWZHpQLpjra7K1uN1urtfYBxbHZ3XehcEhPutMDGaahi")));
    assert_eq!(c.price_sol(), None, "not priced in SOL");
    assert_eq!(c.progress_bps(), Some(1466));
}

#[test]
fn wrong_account_is_refused() {
    let mut data = b64(include_str!("fixtures/curve_sol_active.b64"));
    data[0] ^= 1;
    assert_eq!(BondingCurve::decode(&data), Err(pump_curve::Error::WrongDiscriminator));
}

/// Rebuild the curve as it stood before each trade (the event carries the reserves after it) and check that the
/// quote reproduces the chain's numbers exactly: tokens, SOL and both fees.
#[test]
fn quotes_reproduce_real_trades() {
    let mut buys = 0;
    let mut sells = 0;
    for &(ix, is_buy, sol, tokens, post_vs, post_vt, fee_bps, fee, creator_bps, creator_fee, sig) in fixtures::TRADES {
        let fees = Fees::new(fee_bps, creator_bps);
        let (vs, vt) = if is_buy { (post_vs - sol, post_vt + tokens) } else { (post_vs + sol, post_vt - tokens) };
        let curve = BondingCurve {
            virtual_sol_reserves: vs,
            virtual_token_reserves: vt,
            real_sol_reserves: vs - 30_000_000_000,
            real_token_reserves: vt - 279_900_000_000_000,
            token_total_supply: 1_000_000_000_000_000,
            creator: Some(Pubkey::new([1; 32])),
            ..BondingCurve::default()
        };
        if is_buy {
            // buy_exact_sol_in / buy_exact_quote_in: the user's budget is not in the event, but the amount that
            // left the wallet is, and spending exactly that must give the same split.
            let q = curve.buy_exact_sol_in(sol + fee + creator_fee, fees).unwrap();
            assert_eq!(
                (q.tokens_out, q.sol_to_curve, q.protocol_fee, q.creator_fee),
                (tokens, sol, fee, creator_fee),
                "{ix} {sig}"
            );
            buys += 1;
        } else {
            let q = curve.sell_exact_tokens_in(tokens, fees).unwrap();
            assert_eq!((q.sol_from_curve, q.protocol_fee, q.creator_fee), (sol, fee, creator_fee), "{ix} {sig}");
            sells += 1;
        }
    }
    assert!(buys >= 3 && sells >= 5, "{buys} buys, {sells} sells");
}
