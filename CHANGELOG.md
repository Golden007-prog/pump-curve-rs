# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/).

## [1.0.0] - 2026-11-17

### Added

- `BondingCurve::decode`: pump.fun bonding-curve account bytes to a struct, with the discriminator check and every layout length from 49 to 166 bytes (creator, mayhem and cashback flags, quote mint, creator fee rate, fee buckets, depth and the v3 fields); `to_bytes` and `fresh` for fixtures and first-buy quotes.
- Exact integer quotes with `u128` intermediates and checked arithmetic: `buy_exact_sol_in`, `buy_exact_tokens_out`, `sell_exact_tokens_in`, returning the curve amount and the protocol and creator fees separately; `apply_buy` / `apply_sell` to chain quotes; `charged_fees`.
- `Fees`, `FeeTier` and `select_fee_tier` (pump.fun's market-cap tier rule). No fee is hardcoded.
- Slippage helpers `max_sol_cost`, `min_sol_output`, `min_tokens_out`.
- Figures: `price_sol`, `market_cap_sol`, `market_cap_lamports`, `progress_bps`, `progress_pct`, `migration_market_cap_sol`, `is_sol_quoted`, `is_migrated`.
- `bonding_curve_address` and `Pubkey` (base58, `find_program_address`, `create_program_address`, `is_on_curve`) with a built-in SHA-256 and ed25519 point check.
- `Error` enum with `Display`, and `std::error::Error` behind the default `std` feature.
- `no_std` support, `#![forbid(unsafe_code)]`, zero dependencies.
- Tests against mainnet fixtures (three curve accounts, nine trade events) and property-style tests; `examples/quote.rs`.
