//! Quote a buy and a sell against a pump.fun bonding curve.
//!
//! ```bash
//! cargo run --example quote
//! ```
//!
//! With no arguments it uses a brand-new standard curve. Pass a bonding-curve account's base64 data (from
//! `getAccountInfo` with `"encoding": "base64"`) to quote against a real one:
//!
//! ```bash
//! cargo run --example quote -- <base64 account data>
//! ```

use pump_curve::{bonding_curve_address, max_sol_cost, min_sol_output, BondingCurve, Fees, Pubkey, LAMPORTS_PER_SOL};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Any mint's curve account can be derived offline. This is the mint from the JS pump-curve README.
    let mint: Pubkey = "DxYWLXgKJpQmfhTG7ikK6oX6oDhxMhN5wCs5BJekpump".parse()?;
    let (curve_address, bump) = bonding_curve_address(&mint)?;
    println!("the curve account of {mint} is {curve_address} (bump {bump})\n");

    let curve = match std::env::args().nth(1) {
        Some(b64) => {
            println!("quoting the account passed in");
            BondingCurve::decode(&base64(&b64)?)?
        }
        None => {
            println!("quoting a brand-new standard curve");
            BondingCurve::fresh(Pubkey::new([1; 32]))
        }
    };

    // pump.fun's published bonding-curve rate when this was written: 0.95% protocol + 0.30% creator.
    // Read the live value before trading; see the `Fees` docs.
    let fees = Fees::new(95, 30);

    if curve.complete {
        println!("this curve is complete: it has migrated (or is about to) and no longer trades");
        return Ok(());
    }
    match (curve.price_sol(), curve.market_cap_sol(), curve.progress_pct()) {
        (Some(price), Some(cap), Some(progress)) => {
            println!("price {price:.10} SOL per token · market cap {cap:.2} SOL · {progress:.2}% of the curve sold")
        }
        _ => println!("not priced in SOL: quoted in another token, amounts below are in its raw units"),
    }

    let spend = LAMPORTS_PER_SOL / 2;
    let buy = curve.buy_exact_sol_in(spend, fees)?;
    if curve.is_sol_quoted() {
        println!("\nbuy: {buy}");
    } else {
        println!("\nbuy: {buy:?}");
    }
    let exact = curve.buy_exact_tokens_out(buy.tokens_out, fees)?;
    println!(
        "same tokens via `buy`: total {} lamports, max_sol_cost at 1% slippage {}",
        exact.total_sol,
        max_sol_cost(exact.total_sol, 100)?
    );

    let mut after = curve;
    after.apply_buy(&buy)?;
    let sell = after.sell_exact_tokens_in(buy.tokens_out, fees)?;
    if curve.is_sol_quoted() {
        println!("sell them straight back: {sell}");
    } else {
        println!("sell them straight back: {sell:?}");
    }
    println!("min_sol_output at 1% slippage: {}", min_sol_output(sell.sol_out, 100)?);
    println!("round trip cost: {} lamports", buy.total_sol - sell.sol_out);
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
