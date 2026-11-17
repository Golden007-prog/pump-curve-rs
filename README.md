# pump-curve-rs

**pump.fun bonding curve math in Rust: decode curves and quote buys and sells with exact integer math.** Reads the account bytes, derives the curve address from the mint, and gives you the tokens, SOL, fees and slippage limits a trade would have, rounded the way the program rounds. Zero dependencies, `no_std`, no `unsafe`.

![dependencies: 0](https://img.shields.io/badge/dependencies-0-brightgreen)
![rust: 1.70+](https://img.shields.io/badge/rust-1.70%2B-orange)
[![license: MIT](https://img.shields.io/badge/license-MIT-black)](LICENSE)
[![Discord](https://img.shields.io/badge/Discord-join-5865F2?logo=discord&logoColor=white)](https://discord.gg/grokbot)
[![GitHub stars](https://img.shields.io/github/stars/blixvip/pump-curve-rs?style=social)](https://github.com/blixvip/pump-curve-rs/stargazers)

<a href="https://blixvip.com"><img src="assets/blix-hero.jpg" width="100%" alt="BLIX: AI-powered Solana memecoin terminal. Trade memes first."></a>

**[Start trading at blixvip.com →](https://blixvip.com)** &nbsp;·&nbsp; [Website and guides](https://blixvip.github.io/pump-curve-rs/) &nbsp;·&nbsp; [Public API](https://blixvip.com/api) &nbsp;·&nbsp; [MCP server](https://blixvip.com/mcp) &nbsp;·&nbsp; [Discord](https://discord.gg/grokbot)

Built by the team behind **[Blix](https://blixvip.com)**, the 0% fee Solana memecoin terminal. This is the Rust port of our [pump-curve](https://github.com/blixvip/pump-curve) reader, with the trade math added.

If pump-curve-rs saves you time, **[give it a ⭐](https://github.com/blixvip/pump-curve-rs)**. It helps other Solana developers find it.

## The problem

A bot or backend that trades pump.fun coins needs to know, before it sends a transaction, how many tokens a buy returns and what limit to put in `max_sol_cost` or `min_sol_output`. Floating-point estimates drift from what the program does by a few lamports, which is enough to fail a tight slippage check, and the official SDK is TypeScript. Pulling a full Solana SDK into a Rust crate just to read six integers and do a constant-product quote is a lot of weight.

## Features

- **Decode** the `BondingCurve` account from its raw bytes: discriminator check, virtual and real reserves, supply, `complete`, and every field pump.fun has appended since (creator, mayhem and cashback flags, quote mint, creator fee rate, fee buckets, depth). Old 49-byte and 81-byte accounts and today's 166-byte accounts all decode; missing fields read as zero or `None`, as pump.fun's docs say to.
- **Quote** all three trades with `u128` intermediates and checked arithmetic:
  `buy_exact_sol_in` (SOL in, tokens out), `buy_exact_tokens_out` (tokens wanted, SOL needed) and `sell_exact_tokens_in` (tokens in, SOL out). Each returns the SOL that moves on the curve, the protocol fee and the creator fee separately.
- **Fees as a parameter**, never hardcoded. `select_fee_tier` reproduces pump.fun's market-cap tier selection when you give it the tiers.
- **Slippage helpers**: `max_sol_cost`, `min_sol_output`, `min_tokens_out`.
- **Figures**: price per token, market cap (exact in lamports, or as a float in SOL), curve progress, and the market cap at which the curve completes.
- **Curve address from the mint**, offline: a built-in SHA-256 and ed25519 point check give `find_program_address` with no dependency. `Pubkey` parses and prints base58.
- `#![no_std]` when you turn off the default `std` feature (which only adds `std::error::Error` for `Error`). `#![forbid(unsafe_code)]`. Docs on every public item.

Checked against mainnet: the quotes reproduce nine real pump.fun trades to the lamport and the token, fees included (see [How it works](#how-it-works)).

## Install

```bash
cargo add --git https://github.com/blixvip/pump-curve-rs
```

or in `Cargo.toml`:

```toml
[dependencies]
pump-curve = { git = "https://github.com/blixvip/pump-curve-rs" }
# no_std:
# pump-curve = { git = "https://github.com/blixvip/pump-curve-rs", default-features = false }
```

Rust 1.70 or newer. No dependencies.

## Quick start

```rust
use pump_curve::{bonding_curve_address, max_sol_cost, min_sol_output, BondingCurve, Fees, Pubkey};

// 1. Where the curve lives (pure, no network).
let mint: Pubkey = "DxYWLXgKJpQmfhTG7ikK6oX6oDhxMhN5wCs5BJekpump".parse()?;
let (curve_address, _bump) = bonding_curve_address(&mint)?;
assert_eq!(curve_address.to_string(), "6FrLSW5z5uNP3ovpFYUQCoY5d7sXF1mEMsFvvQdeu8dF");

// 2. Fetch that account with your RPC client (getAccountInfo, base64) and decode it.
let data = BondingCurve::fresh(Pubkey::new([1; 32])).to_bytes(); // stand-in for the fetched bytes
let curve = BondingCurve::decode(&data)?;

// 3. Quote. Fees are yours to supply: 95 + 30 bps was pump.fun's published bonding-curve rate in November 2026.
let fees = Fees::new(95, 30);
let buy = curve.buy_exact_sol_in(500_000_000, fees)?; // spend 0.5 SOL, fees included
println!("{} tokens for {} lamports", buy.tokens_out, buy.total_sol);

let cost = curve.buy_exact_tokens_out(buy.tokens_out, fees)?;
let max_cost = max_sol_cost(cost.total_sol, 100)?; // `buy` with 1% slippage

let mut after = curve;
after.apply_buy(&buy)?;
let sell = after.sell_exact_tokens_in(buy.tokens_out, fees)?;
let min_out = min_sol_output(sell.sol_out, 100)?; // `sell` with 1% slippage
assert!(sell.sol_out < buy.total_sol);
# let _ = (max_cost, min_out);
# Ok::<(), pump_curve::Error>(())
```

## Examples

`examples/quote.rs` derives a curve address, then quotes a 0.5 SOL buy, the same tokens through `buy`, and the sell back:

```bash
cargo run --example quote
```

```text
the curve account of DxYWLXgKJpQmfhTG7ikK6oX6oDhxMhN5wCs5BJekpump is 6FrLSW5z5uNP3ovpFYUQCoY5d7sXF1mEMsFvvQdeu8dF (bump 254)

quoting a brand-new standard curve
price 0.0000000280 SOL per token · market cap 27.96 SOL · 0.00% of the curve sold

buy with 0.500 SOL: 17376518.13 tokens · 493827159 lamports to the curve + 4691359 protocol + 1481482 creator fee
same tokens via `buy`: total 499999999 lamports, max_sol_cost at 1% slippage 504999999
sell them straight back: 487654317 lamports out, min_sol_output at 1% slippage 482777773
round trip cost: 12345683 lamports
```

Pass a real account's base64 data to quote it instead. With `curl` and the free public Solana RPC:

```bash
curl -s https://api.mainnet-beta.solana.com -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getAccountInfo","params":["<curve address>",{"encoding":"base64"}]}'
cargo run --example quote -- <the base64 string from result.value.data[0]>
```

More examples:

| | |
|---|---|
| [`live_quote`](examples/live_quote.rs) | Quote a live coin from its mint. The crate has no HTTP dependency, so the example prints the `curl` request, then reads curl's JSON from stdin: `curl ... \| cargo run -q --example live_quote -- <mint> - 0.5`. Checks the owner, and reports completed, mayhem-mode and non-SOL curves instead of quoting them. |
| [`cross_check`](examples/cross_check.rs) + [`cross_check.mjs`](examples/cross_check.mjs) | Decode the same account bytes with this crate and the JavaScript [pump-curve](https://github.com/blixvip/pump-curve) and compare price, market cap, progress and migration cap: `git clone https://github.com/blixvip/pump-curve ../pump-curve && node examples/cross_check.mjs` |
| [`bonding_curve_formula`](examples/bonding_curve_formula.rs) | The buy and sell formulas written out by hand, asserted equal to the crate on a real mainnet curve |
| [`price_market_cap`](examples/price_market_cap.rs) | Price, market cap, progress and migration market cap from a real curve, by hand and with the crate |

Guides on the project site: [The pump.fun bonding curve formula, explained](https://blixvip.github.io/pump-curve-rs/pump-fun-bonding-curve-formula.html) · [How to calculate a pump.fun token's price and market cap](https://blixvip.github.io/pump-curve-rs/pump-fun-token-price-market-cap.html)

## API

| Item | What it does |
|---|---|
| `BondingCurve::decode(&[u8])` | Account bytes to a `BondingCurve` (checks discriminator and length) |
| `BondingCurve::fresh(creator)` / `to_bytes()` | A new standard curve from pump.fun's documented launch values; encode back to 166 bytes |
| `buy_exact_sol_in(lamports, fees)` | SOL budget (fees included) to `BuyQuote` |
| `buy_exact_tokens_out(tokens, fees)` | Exact tokens to `BuyQuote` with the cost |
| `sell_exact_tokens_in(tokens, fees)` | Exact tokens to `SellQuote` with the proceeds |
| `apply_buy` / `apply_sell` | Move the reserves as a quoted trade would, to chain quotes |
| `charged_fees(fees)` | The fees a curve actually charges (no creator fee without a creator) |
| `price_sol()`, `market_cap_sol()`, `market_cap_lamports(supply)` | Price and market cap |
| `progress_bps()`, `progress_pct()`, `migration_market_cap_sol()` | How far along the curve is, and where it ends |
| `is_sol_quoted()`, `is_migrated()` | Quote asset and migration state |
| `Fees`, `FeeTier`, `select_fee_tier` | Fee rates and pump.fun's market-cap tier rule |
| `max_sol_cost`, `min_sol_output`, `min_tokens_out` | Slippage limits |
| `bonding_curve_address(mint)`, `Pubkey::find_program_address`, `Pubkey::create_program_address`, `Pubkey::is_on_curve` | Program-derived addresses |
| `Pubkey` | 32-byte address, `FromStr` / `Display` in base58 |
| `Error` | Every failure, `Display` and (with `std`) `std::error::Error` |

Full reference with examples: `cargo doc --open`.

## Configuration

Nothing to configure. The crate makes no network calls; bring your own RPC client and pass in the account bytes. The free public endpoint `https://api.mainnet-beta.solana.com` serves `getAccountInfo` and `getMultipleAccounts` for this (rate limited); the free plans of Helius, QuickNode and others work too.

Cargo features: `std` (default) implements `std::error::Error`. Turn it off for `no_std`.

Fee rates are an argument on every quote. Read them from pump.fun before trading: the `FeeConfig` account of pump.fun's fee program, or the table in [pump.fun's public docs](https://github.com/pump-fun/pump-public-docs/blob/main/docs/FEE_PROGRAM_README.md). When this was written that table listed 0.95% protocol + 0.30% creator for bonding-curve trades, and live trade events carried the same 95 and 30 bps.

## How it works

A pump.fun coin trades against its bonding-curve account until the curve's real tokens are sold; then it migrates to a pool. The account is the program-derived address of seeds `["bonding-curve", mint]` under the pump program `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`. Its layout, from pump.fun's [published IDL](https://github.com/pump-fun/pump-public-docs/blob/main/idl/pump.json):

| Bytes | Field |
|---|---|
| 0–7 | discriminator `17 b7 f8 37 60 d8 ac 60` (`sha256("account:BondingCurve")[..8]`) |
| 8–15 | virtual token reserves |
| 16–23 | virtual SOL (quote) reserves |
| 24–31 | real token reserves |
| 32–39 | real SOL (quote) reserves |
| 40–47 | token total supply |
| 48 | complete |
| 49–80 | creator |
| 81, 82 | is_mayhem_mode, is_cashback_coin |
| 83–114 | quote mint (zero = SOL) |
| 115–122 | creator fee bps (custom pairs) |
| 123, 124 | can_edit_creator_fee, is_holder_reward |
| 125–140 | creator fee and protocol fees waiting to be swept |
| 141 | depth |
| 142–165 | initial virtual quote reserves, post-completion base out and quote in |

The price is a constant product over the virtual reserves. The quotes use the program's own rounding:

- **buy_exact_sol_in** (from the formula pump.fun documents in the IDL): `net = floor(spend × 10000 / (10000 + fee_bps))`, trimmed if net plus its fees exceeds the budget; `tokens = floor((net − 1) × vT / (vS + net − 1))`.
- **buy** (from pump.fun's SDK): `sol = floor(tokens × vS / (vT − tokens)) + 1`, plus fees.
- **sell** (from pump.fun's SDK): `sol = floor(tokens × vS / (vT + tokens))`, less fees.
- Each fee is `ceil(sol × bps / 10000)`, charged separately for protocol and creator. No creator, no creator fee.

These were checked against nine real mainnet `TradeEvent`s (four buys through `buy_exact_sol_in` / `buy_exact_quote_in`, five `sell`s): rebuilding the curve from each event's reserves, the quote matches the chain's tokens, SOL and both fees exactly. The fixtures are in `tests/fixtures/`.

## Limitations

- **Mayhem-mode coins** use different curve parameters. Real mayhem trades did not match these formulas, so do not quote them with this crate. `progress_bps` returns `None` for them; the quote methods do not refuse them, so check `is_mayhem_mode` yourself.
- **The `buy` (exact tokens out) formula is from pump.fun's SDK** and was not matched against a live non-mayhem `buy` event (none turned up in the sample). `buy_exact_sol_in` and `sell` were.
- **v3 trades past completion** (the post-completion leg that buys from the future pool) are not modelled. A quote larger than the remaining real tokens is capped (`buy_exact_sol_in`) or refused (`buy_exact_tokens_out`).
- **Curves quoted in another token** decode and quote, but the amounts are in that token's raw units, there is no SOL price, and Token-2022 transfer fees on the quote token are not applied.
- **Fees are not read from chain.** You pass them. Tier selection works if you supply the tiers; decoding the `FeeConfig` account is not included.
- **Progress** assumes a standard curve (1,073,000,000 starting virtual tokens).
- `is_on_curve` follows `curve25519-dalek` (which Solana uses): a y coordinate at or above 2^255 − 19 is reduced rather than rejected. The difference matters for 19 of 2^255 values.
- The crate does not fetch anything or build transactions. It also does not check the account owner; confirm it is the pump program before decoding.
- MSRV 1.70 is declared from the language features used; CI against that exact toolchain is not set up yet.

Not affiliated with pump.fun.

## Contributing

Issues and pull requests are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md). Questions, ideas or help: **[discord.gg/grokbot](https://discord.gg/grokbot)**.

## Good first issues

New to the project? These are small, scoped and come with step-by-step instructions: **[good first issues](https://github.com/blixvip/pump-curve-rs/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22)**. Bigger pieces (a WASM build for browsers, Python bindings, publishing to crates.io) are under **[help wanted](https://github.com/blixvip/pump-curve-rs/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22)**, and the [roadmap issue](https://github.com/blixvip/pump-curve-rs/issues/15) groups everything open. Questions go in [Discussions](https://github.com/blixvip/pump-curve-rs/discussions) or the [Discord](https://discord.gg/grokbot).

## Related projects

| | |
|---|---|
| [pump-curve](https://github.com/blixvip/pump-curve) | The JavaScript version: read a pump.fun curve from Solana, watch it until migration |
| [sol-holders](https://github.com/blixvip/sol-holders) | Top holders of any Solana token, with pools and curves left out |

**[Blix](https://blixvip.com)** is a Solana memecoin terminal with 0% fees on every trade: every pump.fun launch live, AI coin ratings, narratives, wallet and X tracking, and one-click trading. Its [public API](https://blixvip.com/api) and [MCP server](https://blixvip.com/mcp) are BLIX-provided services; this crate does not use or need them.

## Contributors

Every merged pull request is credited in the release notes. Pick a [good first issue](https://github.com/blixvip/pump-curve-rs/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22) or say hi in the [Discord](https://discord.gg/grokbot).

<a href="https://github.com/blixvip/pump-curve-rs/graphs/contributors"><img src="https://contrib.rocks/image?repo=blixvip/pump-curve-rs" alt="Contributors to pump-curve-rs"></a>

## License

MIT
