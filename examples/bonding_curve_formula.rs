//! The pump.fun bonding curve formula written out by hand, checked against the crate on a real mainnet curve.
//! This is the code from the guide at <https://blixvip.github.io/pump-curve-rs/pump-fun-bonding-curve-formula.html>.
//!
//! ```bash
//! cargo run --example bonding_curve_formula
//! ```

use pump_curve::{BondingCurve, Fees};

/// Each fee is rounded up, separately: `ceil(sol * bps / 10_000)`.
fn fee(sol: u128, bps: u128) -> u128 {
    let n = sol * bps;
    n / 10_000 + u128::from(n % 10_000 != 0)
}

/// Sell `tokens`: SOL out of the constant product, rounded down, then both fees taken off.
fn sell(v_tokens: u128, v_sol: u128, tokens: u128, protocol_bps: u128, creator_bps: u128) -> u128 {
    let sol = tokens * v_sol / (v_tokens + tokens);
    sol - fee(sol, protocol_bps) - fee(sol, creator_bps)
}

/// Buy exactly `tokens`: SOL into the constant product, rounded down plus one lamport, then both fees on top.
fn buy_exact_tokens(v_tokens: u128, v_sol: u128, tokens: u128, protocol_bps: u128, creator_bps: u128) -> u128 {
    let sol = tokens * v_sol / (v_tokens - tokens) + 1;
    sol + fee(sol, protocol_bps) + fee(sol, creator_bps)
}

/// Spend `budget` lamports, fees included: how many tokens come back.
fn buy_with_sol(v_tokens: u128, v_sol: u128, budget: u128, protocol_bps: u128, creator_bps: u128) -> u128 {
    let mut net = budget * 10_000 / (10_000 + protocol_bps + creator_bps);
    let total = net + fee(net, protocol_bps) + fee(net, creator_bps);
    if total > budget {
        net -= total - budget; // rounding the fees up can overshoot by a lamport or two
    }
    (net - 1) * v_tokens / (v_sol + net - 1)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A real SOL-quoted pump.fun curve read from mainnet (tests/fixtures/curve_sol_active.b64).
    let data = base64(include_str!("../tests/fixtures/curve_sol_active.b64"))?;
    let curve = BondingCurve::decode(&data)?;
    let (vt, vs) = (curve.virtual_token_reserves as u128, curve.virtual_sol_reserves as u128);
    println!("virtual reserves: {vt} token units, {vs} lamports");
    println!("k = x * y = {}", vt * vs);

    let fees = Fees::new(95, 30); // 0.95% protocol + 0.30% creator
    let (p, c) = (95, 30);

    let budget = 1_000_000_000; // 1 SOL
    let by_hand = buy_with_sol(vt, vs, budget, p, c);
    let by_crate = curve.buy_exact_sol_in(budget as u64, fees)?.tokens_out as u128;
    println!("buy with 1 SOL:      by hand {by_hand} tokens, crate {by_crate}");
    assert_eq!(by_hand, by_crate);

    let tokens = 10_000_000_000_000; // 10 million tokens (6 decimals)
    let by_hand = buy_exact_tokens(vt, vs, tokens, p, c);
    let by_crate = curve.buy_exact_tokens_out(tokens as u64, fees)?.total_sol as u128;
    println!("buy 10M tokens:      by hand {by_hand} lamports, crate {by_crate}");
    assert_eq!(by_hand, by_crate);

    let by_hand = sell(vt, vs, tokens, p, c);
    let by_crate = curve.sell_exact_tokens_in(tokens as u64, fees)?.sol_out as u128;
    println!("sell 10M tokens:     by hand {by_hand} lamports, crate {by_crate}");
    assert_eq!(by_hand, by_crate);

    println!("all three match to the lamport");
    Ok(())
}

/// Standard base64, enough for account data.
fn base64(text: &str) -> Result<Vec<u8>, &'static str> {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0);
    for &c in text.trim().as_bytes() {
        if c == b'=' {
            break;
        }
        let v = A.iter().position(|&a| a == c).ok_or("not base64")? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}
