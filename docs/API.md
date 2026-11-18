# pump-curve API

Full reference with examples: run `cargo doc --open`. Every public item has rustdoc, and the examples in it are compiled and run as tests. This page is the map.

## Accounts

| Item | What it does |
|---|---|
| `BondingCurve::decode(&[u8]) -> Result<BondingCurve, Error>` | Decode a pump.fun BondingCurve account (every layout from 49 to 166 bytes). Checks the discriminator. |
| `BondingCurve::to_bytes()` | Encode back to the current layout (useful for fixtures). |
| `BondingCurve::fresh(creator)` | A brand-new curve with the launch reserves. |
| `is_sol_quoted()`, `is_migrated()` | Quote asset is SOL; curve is complete. |
| `price_sol()`, `market_cap_sol()`, `market_cap_lamports(mint_supply)` | Price and market cap from virtual reserves. |
| `progress_bps()`, `progress_pct()`, `migration_market_cap_sol()` | Curve progress and the market cap at completion. |
| `bonding_curve_address(&mint) -> Result<(Pubkey, u8), Error>` | Derive the curve PDA, no dependencies. |

## Quotes (all integer math, checked)

| Item | What it does |
|---|---|
| `Fees { protocol_bps, creator_bps }`, `Fees::total_bps()` | Fee rates you pass in. Nothing is hardcoded. |
| `FeeTier`, `select_fee_tier(&tiers, market_cap_lamports)` | pump.fun's market-cap fee-tier rule. |
| `buy_exact_sol_in(lamports, fees) -> BuyQuote` | Spend SOL, get tokens. |
| `buy_exact_tokens_out(tokens, fees) -> BuyQuote` | Tokens wanted, SOL needed. |
| `sell_exact_tokens_in(tokens, fees) -> SellQuote` | Sell tokens, get SOL. |
| `apply_buy(&BuyQuote)`, `apply_sell(&SellQuote)` | Move the reserves as the trade would. |
| `max_sol_cost`, `min_sol_output`, `min_tokens_out` | Slippage limits for instruction arguments. |

`BuyQuote` and `SellQuote` implement `Display` for SOL-quoted curves. Their summaries print token amounts to six decimal places, SOL amounts to nine, and show the curve amount and both fees separately.

## Keys

`Pubkey` (32 bytes) with `from_base58`, `is_on_curve`, `create_program_address`, `find_program_address`. Constants: `PUMP_PROGRAM_ID`, `WSOL_MINT`, `BONDING_CURVE_DISCRIMINATOR`, `LAMPORTS_PER_SOL`, `TOKEN_DECIMALS`, `TOKEN_UNIT`, the initial reserve constants, `TOKEN_TOTAL_SUPPLY`, `MAX_SEEDS`, `MAX_SEED_LEN`, `BPS_DENOMINATOR`.

## Errors

`Error` variants: `AccountTooShort { len }`, `WrongDiscriminator`, `CurveComplete` (curve finished, no longer trades), `EmptyReserves`, `AmountTooSmall`, `InsufficientReserves` (trade larger than the tokens left), `InvalidBasisPoints`, `Overflow`, `InvalidBase58`, `InvalidPubkeyLength`, `InvalidSeeds`, `NoViableBump`. Each has a readable `Display`; with the default `std` feature it also implements `std::error::Error`.
