//! A pump.fun token's price, market cap, curve progress and migration market cap, from the bonding-curve account.
//! This is the code from the guide at <https://blixvip.github.io/pump-curve-rs/pump-fun-token-price-market-cap.html>.
//!
//! ```bash
//! cargo run --example price_market_cap
//! ```

use pump_curve::{BondingCurve, LAMPORTS_PER_SOL, TOKEN_TOTAL_SUPPLY, TOKEN_UNIT};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A real SOL-quoted pump.fun curve read from mainnet (tests/fixtures/curve_sol_active.b64).
    let data = base64(include_str!("../tests/fixtures/curve_sol_active.b64"))?;
    let curve = BondingCurve::decode(&data)?;

    // By hand: price = virtual SOL / virtual tokens, both in whole units.
    let sol = curve.virtual_sol_reserves as f64 / LAMPORTS_PER_SOL as f64;
    let tokens = curve.virtual_token_reserves as f64 / TOKEN_UNIT as f64;
    let price = sol / tokens;
    // Market cap = price x total supply (one billion tokens on a standard coin).
    let supply = curve.token_total_supply as f64 / TOKEN_UNIT as f64;
    let market_cap = price * supply;
    println!("by hand:  price {price:.12} SOL, market cap {market_cap:.4} SOL");

    // The crate gives the same figures, and returns None when they do not apply (migrated, or not priced in SOL).
    let price_c = curve.price_sol().ok_or("not priced in SOL")?;
    let cap_c = curve.market_cap_sol().ok_or("not priced in SOL")?;
    println!("crate:    price {price_c:.12} SOL, market cap {cap_c:.4} SOL");
    assert_eq!(price, price_c);
    assert_eq!(market_cap, cap_c);

    // Exact integer market cap in lamports, the way pump.fun's fee program computes it for fee tiers.
    let cap_lamports = curve.market_cap_lamports(TOKEN_TOTAL_SUPPLY)?;
    println!("exact:    market cap {cap_lamports} lamports");

    // How far along the curve is, and the market cap at which it completes and migrates.
    let progress = curve.progress_pct().ok_or("no progress for this curve")?;
    let migrates_at = curve.migration_market_cap_sol().ok_or("no migration cap")?;
    println!("progress: {progress:.2}% of the curve sold, migrates at {migrates_at:.2} SOL market cap");

    // In dollars: multiply by SOL's price from any source you trust. 140.0 is a placeholder, not a quote.
    let sol_usd = 140.0;
    println!("at ${sol_usd} per SOL: market cap ${:.0}", cap_c * sol_usd);

    // A brand-new coin, for comparison.
    let fresh = BondingCurve::fresh(pump_curve::Pubkey::default());
    println!(
        "new coin: price {:.12} SOL, market cap {:.2} SOL, migrates at {:.2} SOL",
        fresh.price_sol().unwrap_or_default(),
        fresh.market_cap_sol().unwrap_or_default(),
        fresh.migration_market_cap_sol().unwrap_or_default(),
    );
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
