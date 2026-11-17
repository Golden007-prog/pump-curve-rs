//! Prints this crate's reading of several bonding curves as JSON, with each curve's account bytes in base64, so
//! another implementation can decode the same bytes and compare. `examples/cross_check.mjs` does that against the
//! JavaScript [pump-curve](https://github.com/blixvip/pump-curve) package.
//!
//! ```bash
//! cargo run -q --example cross_check
//! ```

use pump_curve::{BondingCurve, Fees, Pubkey, LAMPORTS_PER_SOL};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cases: Vec<(&str, Vec<u8>)> = vec![
        ("mainnet: active SOL curve", base64_decode(include_str!("../tests/fixtures/curve_sol_active.b64"))?),
        ("mainnet: migrated curve", base64_decode(include_str!("../tests/fixtures/curve_migrated.b64"))?),
        (
            "mainnet: curve quoted in another token",
            base64_decode(include_str!("../tests/fixtures/curve_other_quote.b64"))?,
        ),
    ];
    // Synthetic ones, so the comparison also covers a new coin and a coin near the end of its curve.
    let fresh = BondingCurve::fresh(Pubkey::new([1; 32]));
    cases.push(("synthetic: brand-new curve", fresh.to_bytes().to_vec()));
    let mut late = fresh;
    let buy = late.buy_exact_sol_in(80 * LAMPORTS_PER_SOL, Fees::ZERO)?;
    late.apply_buy(&buy)?;
    cases.push(("synthetic: after an 80 SOL buy", late.to_bytes().to_vec()));

    println!("[");
    for (i, (name, bytes)) in cases.iter().enumerate() {
        let c = BondingCurve::decode(bytes)?;
        println!(
            "  {{\"name\":\"{name}\",\"base64\":\"{}\",\"complete\":{},\"priceSol\":{},\"marketCapSol\":{},\"progressPct\":{},\"migrationMarketCapSol\":{}}}{}",
            base64_encode(bytes),
            c.complete,
            num(c.price_sol()),
            num(c.market_cap_sol()),
            num(c.progress_pct()),
            num(c.migration_market_cap_sol()),
            if i + 1 < cases.len() { "," } else { "" }
        );
    }
    println!("]");
    Ok(())
}

/// A JSON number, or null. `{:e}` keeps full f64 precision.
fn num(v: Option<f64>) -> String {
    v.map_or("null".into(), |x| format!("{x:e}"))
}

const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn base64_decode(text: &str) -> Result<Vec<u8>, &'static str> {
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
