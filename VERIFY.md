# Verification

Done on 2026-11-17, Windows 11, `rustc 1.95.0`, `cargo 1.95.0`, `clippy 0.1.95`.

## Commands and results

| Command | Result |
|---|---|
| `cargo test` | 43 passed: 16 unit, 8 mainnet-fixture, 4 property, 15 doctests (includes the README's Rust snippet) |
| `cargo test --no-default-features` | same 43 passed (the example is gated on `std`) |
| `cargo clippy -- -D warnings` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy --all-targets --no-default-features -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | clean; `#![deny(missing_docs)]` is on |
| `cargo tree` | `pump-curve v1.0.0`, nothing else |

```text
     Running unittests src\lib.rs
test result: ok. 16 passed; 0 failed
     Running tests\mainnet.rs
test result: ok. 8 passed; 0 failed
     Running tests\properties.rs
test result: ok. 4 passed; 0 failed
   Doc-tests pump_curve
test result: ok. 15 passed; 0 failed
```

`target/` was removed with `cargo clean` afterwards.

## Sources checked

- **Account layout and discriminator**: pump.fun's published IDL, <https://github.com/pump-fun/pump-public-docs/blob/main/idl/pump.json> (`BondingCurve` discriminator `[23,183,248,55,96,216,172,96]`, 19 fields, 166 bytes). The JS pump-curve's offsets (8/16/24/32/40/48, creator at 49, quote mint at 83) agree. `PUMP_PROGRAM_README.md` in the same repo says shorter accounts read missing trailing fields as zero, which `decode` does.
- **Launch constants** (1,073,000,000 virtual tokens, 30 SOL, 793,100,000 real tokens, 10^15 supply): `PUMP_PROGRAM_README.md`, and confirmed on the live fixture (`initial_virtual_quote_reserves = 30_000_000_000`, `virtual - real tokens = 279_900_000_000_000`).
- **buy_exact_sol_in formula**: the IDL's own doc comment on that instruction.
- **buy and sell formulas, fee rounding (ceil per fee), creator fee only with a creator, tier selection**: `@pump-fun/pump-sdk` 4.3.1 source (`bondingCurve.ts`, `fees.ts`), read from the npm tarball.
- **Current fee values**: `docs/fees.png` linked from `FEE_PROGRAM_README.md` lists bonding curve 0.30% creator + 0.95% protocol. Live trade events carried `fee_basis_points = 95`, `creator_fee_basis_points = 30`. Not hardcoded; documented with source.

## Live checks (public RPC `https://api.mainnet-beta.solana.com`, sequential, a few seconds apart)

- `getAccountInfo` on three curve accounts, saved as `tests/fixtures/*.b64`: an active SOL curve (slot 470678916), the JS README's migrated curve `6FrLSW5z…` (slot 470678721), a curve quoted in another token (slot 470678834). All owned by `6EF8rrec…`, all 166 bytes.
- `getSignaturesForAddress` + `getTransaction` on recent pump program transactions; `TradeEvent`s decoded from the event self-CPI. Nine non-mayhem SOL trades saved in `tests/fixtures/trade_events.json` (4 buys via `buy_exact_sol_in` / `buy_exact_quote_in`, 5 sells). Test `quotes_reproduce_real_trades` rebuilds each pre-trade curve and matches tokens, SOL to/from the curve, protocol fee and creator fee **exactly**.
- PDA derivation: `bonding_curve_address` matches the three real curve accounts (bumps 254, 254, 255; the 254s mean the 255 candidate was on the curve, so the ed25519 check is exercised) and the documented Global PDA `4wTV1Ymi…`.
- `is_on_curve`: differential test against the JS implementation on 256 random values (132 on, 124 off), plus known wallets (trade signers) and PDAs.
- Example against a live account fetched at slot 470683126 (curve `BTCS3Ym9…`): it had completed since the fixture was taken; the example printed "this curve is complete" as intended.

## Example output (`cargo run --example quote`)

```text
the curve account of DxYWLXgKJpQmfhTG7ikK6oX6oDhxMhN5wCs5BJekpump is 6FrLSW5z5uNP3ovpFYUQCoY5d7sXF1mEMsFvvQdeu8dF (bump 254)

quoting a brand-new standard curve
price 0.0000000280 SOL per token · market cap 27.96 SOL · 0.00% of the curve sold

buy with 0.500 SOL: 17376518.13 tokens · 493827159 lamports to the curve + 4691359 protocol + 1481482 creator fee
same tokens via `buy`: total 499999999 lamports, max_sol_cost at 1% slippage 504999999
sell them straight back: 487654317 lamports out, min_sol_output at 1% slippage 482777773
round trip cost: 12345683 lamports
```

With the mainnet fixture (`cargo run --example quote -- "$(cat tests/fixtures/curve_sol_active.b64)"`): price 0.0000001019 SOL, market cap 101.92 SOL, 64.43% sold, the same figures the JS pump-curve's `decodeCurve` gives for those bytes. With the migrated fixture: "this curve is complete".

## Not verified

- The `buy` (exact tokens out) formula against a live non-mayhem `buy` event: none appeared in the recent transactions sampled (three batches of 12, 40 and 80 signatures; many fetches were rate limited). It is taken from pump.fun's SDK and is consistent (property test: the tokens a budget buys never cost more than the budget through `buy`).
- Mayhem-mode coins: their trades did **not** match these formulas (different parameters). Documented as a limitation.
- MSRV 1.70: declared from the features used (`is_some_and`, `let ... else` in tests); not built with a 1.70 toolchain.
- `no_std` on an actual embedded target: only `--no-default-features` on x86_64 was built and tested (no other target installed; not downloaded to save disk).
- The README's `curl` + `cargo run --example quote -- <base64>` flow was run with an equivalent `curl` call, not copy-pasted verbatim.

## Known limitations

See the README's Limitations section: mayhem coins, v3 post-completion leg, Token-2022 transfer fees on non-SOL quotes, fees not read from chain, progress assumes a standard curve, no owner check, no network or transaction building.

## Examples and project site (2026-11-17, rustc 1.95.0)

Checks re-run after adding four examples: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (with and
without default features), `cargo test` (43 passed: 16 unit, 8 mainnet-fixture, 4 property, 15 doctests, README
included) and `cargo test --no-default-features` (same), `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: all clean.
CI now also runs the two guide examples, whose `assert_eq!`s fail the build if the guide code drifts from the crate.

**`cargo run --example bonding_curve_formula`** (real curve from `tests/fixtures/curve_sol_active.b64`):
```text
buy with 1 SOL:      by hand 9526639470429 tokens, crate 9526639470429
buy 10M tokens:      by hand 1050588230 lamports, crate 1050588230
sell 10M tokens:     by hand 988821256 lamports, crate 988821256
all three match to the lamport
```

**`cargo run --example price_market_cap`**: by hand and crate both `price 0.000000101916 SOL, market cap 101.9155 SOL`;
exact 101915521861 lamports; progress 64.43%; migration cap 410.88 SOL; new coin 27.96 SOL.

**`examples/live_quote.rs`**, live against the public RPC (one `getAccountInfo` per coin, sequential). Test mints were
found from the pump program's latest signatures (a handful of public-RPC calls):
- `5RhfKrSQm6p7cbqLgBbpNYvxpz8rXXnofPotjfKqpump` (standard SOL curve): curve `4Q4BXxsXDwwcVzFY3AzbB71qZJ3c41Rcq5c2SbTmgH4T`,
  `price 0.0000000296 SOL · market cap 29.56 SOL · 3.71% of the curve sold · migrates at 410.88 SOL`, 0.5 SOL buy ->
  16442051.32 tokens, max_sol_cost 504999999, sell back 487654317 lamports. Exit 0.
- `DxYWLXgKJpQmfhTG7ikK6oX6oDhxMhN5wCs5BJekpump`: reported as completed. `6yZeBVNd5Mb7wMY4S1RgYxzBx3UkfDDuXcwgDHwVpump`:
  reported as mayhem mode. `H6YKejU3K13Mvget3tKrt8ReNK5jezPZRsvUcHTupump`: reported as quoted in another token. A
  `"value":null` answer: reported as not a pump.fun coin. All exit 0.
- First run found a bug, fixed before commit: a non-SOL curve was quoted in raw units and the follow-up sell failed with
  `CurveComplete` because the "0.5 SOL" buy emptied the curve. The example now stops on non-SOL curves and on buys that
  complete the curve.

**`node examples/cross_check.mjs`** with `PUMP_CURVE_JS` = a local checkout of blixvip/pump-curve (Node v24.15.0):
```text
ok    mainnet: active SOL curve: cap 101.9155 SOL (js 101.9155), progress 64.43% (js 64.43%)
ok    mainnet: migrated curve: complete, progress 100% (js 100%)
ok    mainnet: curve quoted in another token: not priced in SOL (both null)
ok    synthetic: brand-new curve: cap 27.9590 SOL (js 27.9590), progress 0% (js 0%)
ok    synthetic: after an 80 SOL buy: cap 375.8931 SOL (js 375.8931), progress 98.39% (js 98.39%)

all 5 curves agree
```
Without the JS checkout it exits 2 with the clone command. Tolerances: 1e-9 relative for price, market cap and
migration cap; 0.01 for progress (the crate floors to basis points, pump-curve rounds to two decimals).

**Site**: the Rust code blocks in `docs/pump-fun-*.html` are generated from the example files above, so they are the
code that was run. The landing page's quick-start snippet was compiled and run as a throwaway example (printed
`17376518132293 tokens, 0.00% of the curve sold`) and then deleted.

`target/` removed with `cargo clean` afterwards.
