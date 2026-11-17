//! Quote a live pump.fun coin from its mint, with no HTTP or JSON dependency: the example derives the curve
//! address and prints the `curl` command, then reads curl's JSON answer from stdin.
//!
//! ```bash
//! # 1. Print the curve address and the request to make:
//! cargo run -q --example live_quote -- <mint>
//!
//! # 2. Pipe the answer back in (the trailing "-" means: read getAccountInfo JSON from stdin):
//! curl -s https://api.mainnet-beta.solana.com -H 'content-type: application/json' \
//!   -d '{"jsonrpc":"2.0","id":1,"method":"getAccountInfo","params":["<curve address>",{"encoding":"base64"}]}' \
//!   | cargo run -q --example live_quote -- <mint> -
//! ```
//!
//! An optional third argument is the SOL to spend in the quote (default 0.1).
//! Solana RPC needs HTTPS, which `std` cannot do on its own; curl does the transport here so the crate stays
//! dependency-free. In a real program, use the HTTP client you already have.

use pump_curve::{
    bonding_curve_address, max_sol_cost, min_sol_output, BondingCurve, Fees, Pubkey, LAMPORTS_PER_SOL, PUMP_PROGRAM_ID,
    TOKEN_UNIT,
};
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mint: Pubkey = args.first().ok_or("usage: live_quote <mint> [-] [sol to spend]")?.parse()?;
    let (curve_address, _) = bonding_curve_address(&mint)?;
    let rpc = std::env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".into());

    if args.get(1).map(String::as_str) != Some("-") {
        println!("curve account: {curve_address}");
        println!("run:\n");
        println!(
            "curl -s {rpc} -H 'content-type: application/json' -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccountInfo\",\"params\":[\"{curve_address}\",{{\"encoding\":\"base64\"}}]}}' | cargo run -q --example live_quote -- {mint} -"
        );
        return Ok(());
    }

    let mut json = String::new();
    std::io::stdin().read_to_string(&mut json)?;
    if json.contains("\"error\"") {
        return Err(format!("the RPC answered with an error: {}", json.trim()).into());
    }
    if json.contains("\"value\":null") {
        println!("{mint} has no bonding-curve account: not a pump.fun coin, or the mint is wrong");
        return Ok(());
    }
    // Check the owner before trusting the bytes: only the pump program writes real curves.
    let owner = string_after(&json, "\"owner\":\"").ok_or("no owner in the answer")?;
    if owner != PUMP_PROGRAM_ID.to_string() {
        return Err(format!("the account is owned by {owner}, not the pump.fun program").into());
    }
    let b64 = string_after(&json, "\"data\":[\"").ok_or("no base64 data in the answer: was encoding base64?")?;
    let curve = BondingCurve::decode(&base64(b64)?)?;

    if curve.complete || curve.is_migrated() {
        println!("{mint} has completed its bonding curve: it trades on a pool now, not on the curve");
        return Ok(());
    }
    if curve.is_mayhem_mode {
        println!("{mint} is a mayhem-mode coin; this crate's quotes do not match those curves, so it stops here");
        return Ok(());
    }
    match (curve.price_sol(), curve.market_cap_sol(), curve.progress_pct()) {
        (Some(price), Some(cap), Some(progress)) => println!(
            "{mint}\nprice {price:.10} SOL · market cap {cap:.2} SOL · {progress:.2}% of the curve sold · migrates at {:.2} SOL",
            curve.migration_market_cap_sol().unwrap_or_default()
        ),
        _ => {
            let quote = curve.quote_mint.map(|m| m.to_string()).unwrap_or_default();
            println!("{mint} is priced in another token ({quote}), not SOL; this example only quotes SOL curves");
            return Ok(());
        }
    }

    let sol: f64 = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(0.1);
    let spend = (sol * LAMPORTS_PER_SOL as f64) as u64;
    // pump.fun's published bonding-curve rate when this was written. Read the live one before trading.
    let fees = Fees::new(95, 30);
    let buy = curve.buy_exact_sol_in(spend, fees)?;
    let exact = curve.buy_exact_tokens_out(buy.tokens_out, fees)?;
    println!(
        "buy {sol} SOL -> {:.2} tokens (fees {} + {} lamports); max_sol_cost at 1%: {}",
        buy.tokens_out as f64 / TOKEN_UNIT as f64,
        buy.protocol_fee,
        buy.creator_fee,
        max_sol_cost(exact.total_sol, 100)?
    );
    let mut after = curve;
    after.apply_buy(&buy)?;
    if after.complete {
        println!("that buy takes the last tokens on the curve and completes it, so there is no sell to quote");
        return Ok(());
    }
    let sell = after.sell_exact_tokens_in(buy.tokens_out, fees)?;
    println!(
        "sell them back -> {} lamports; min_sol_output at 1%: {}",
        sell.sol_out,
        min_sol_output(sell.sol_out, 100)?
    );
    println!("live chart: https://blixvip.com/#/coin/{mint}");
    Ok(())
}

/// The string value that starts right after `key` (up to the next quote). Enough for this one RPC answer.
fn string_after<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let start = json.find(key)? + key.len();
    let len = json[start..].find('"')?;
    Some(&json[start..start + len])
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
