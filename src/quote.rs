//! Trade quotes against a bonding curve, fees, and slippage limits. All integer, all checked.

use core::fmt;

use crate::{BondingCurve, Error, LAMPORTS_PER_SOL, TOKEN_DECIMALS, TOKEN_UNIT};

/// Basis points in 100%.
pub const BPS_DENOMINATOR: u64 = 10_000;

/// The fee rates a bonding-curve trade pays, in basis points of the SOL that moves on the curve.
///
/// This crate never assumes a fee: pump.fun sets it on chain and has changed it before. When this was written
/// (November 2026), pump.fun's published fee table listed **95 bps protocol + 30 bps creator (1.25% total)** for
/// bonding-curve trades, and real `TradeEvent`s on mainnet carried `fee_basis_points = 95` and
/// `creator_fee_basis_points = 30`. Sources: the fee table in
/// <https://github.com/pump-fun/pump-public-docs/blob/main/docs/FEE_PROGRAM_README.md> and the `FeeConfig`
/// account of pump.fun's fee program. Read the live value (or the tier for the coin's market cap, see
/// [`select_fee_tier`]) before you trade.
///
/// The creator part is only charged when the curve has a creator; the quote methods drop it for you when
/// [`BondingCurve::creator`] is `None`, as pump.fun's SDK does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Fees {
    /// Protocol fee, basis points.
    pub protocol_bps: u64,
    /// Creator fee, basis points.
    pub creator_bps: u64,
}

impl Fees {
    /// No fees.
    pub const ZERO: Fees = Fees { protocol_bps: 0, creator_bps: 0 };

    /// Fees from protocol and creator basis points.
    pub const fn new(protocol_bps: u64, creator_bps: u64) -> Self {
        Fees { protocol_bps, creator_bps }
    }

    /// Protocol plus creator basis points. Errors with [`Error::InvalidBasisPoints`] above 10,000.
    pub fn total_bps(&self) -> Result<u64, Error> {
        let total = self.protocol_bps.checked_add(self.creator_bps).ok_or(Error::InvalidBasisPoints)?;
        if total > BPS_DENOMINATOR {
            return Err(Error::InvalidBasisPoints);
        }
        Ok(total)
    }
}

/// One row of pump.fun's market-cap fee schedule (the fee program's `FeeTier`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct FeeTier {
    /// The tier applies from this market cap upward, in lamports.
    pub market_cap_lamports_threshold: u128,
    /// The rates in this tier.
    pub fees: Fees,
}

/// Picks the fee tier for a market cap the way pump.fun's fee program does (`calculate_fee_tier`): below the first
/// threshold, the first tier; otherwise the last tier whose threshold the market cap reaches. `None` for an empty
/// list. You supply the tiers (read them from pump.fun's `FeeConfig` account); this crate ships none.
///
/// ```
/// use pump_curve::{select_fee_tier, FeeTier, Fees};
///
/// let tiers = [
///     FeeTier { market_cap_lamports_threshold: 0, fees: Fees::new(95, 30) },
///     FeeTier { market_cap_lamports_threshold: 420_000_000_000, fees: Fees::new(5, 95) },
/// ];
/// assert_eq!(select_fee_tier(&tiers, 100_000_000_000), Some(Fees::new(95, 30)));
/// assert_eq!(select_fee_tier(&tiers, 500_000_000_000), Some(Fees::new(5, 95)));
/// ```
pub fn select_fee_tier(tiers: &[FeeTier], market_cap_lamports: u128) -> Option<Fees> {
    let first = tiers.first()?;
    if market_cap_lamports < first.market_cap_lamports_threshold {
        return Some(first.fees);
    }
    tiers
        .iter()
        .rev()
        .find(|t| market_cap_lamports >= t.market_cap_lamports_threshold)
        .map(|t| t.fees)
        .or(Some(first.fees))
}

/// A buy: what goes in, what comes out.
///
/// The display format uses six decimal places for tokens and nine for SOL.
///
/// ```
/// use pump_curve::{BondingCurve, Fees, Pubkey};
///
/// let curve = BondingCurve::fresh(Pubkey::new([1; 32]));
/// let buy = curve.buy_exact_sol_in(500_000_000, Fees::new(95, 30))?;
/// assert_eq!(
///     buy.to_string(),
///     "17376518.132293 tokens for 0.500000000 SOL (curve 0.493827159 + fees 0.004691359 + 0.001481482)"
/// );
/// # Ok::<(), pump_curve::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct BuyQuote {
    /// Tokens received, raw units.
    pub tokens_out: u64,
    /// Lamports that go into the curve's reserves (`sol_amount` in pump.fun's `TradeEvent`).
    pub sol_to_curve: u64,
    /// Protocol fee, lamports.
    pub protocol_fee: u64,
    /// Creator fee, lamports.
    pub creator_fee: u64,
    /// Everything that leaves the wallet: `sol_to_curve + protocol_fee + creator_fee`. Rent for any accounts the
    /// transaction creates is extra.
    pub total_sol: u64,
}

/// A sell: what goes in, what comes out.
///
/// The display format shows the SOL proceeds after the two fees.
///
/// ```
/// use pump_curve::{BondingCurve, Fees, Pubkey};
///
/// let mut curve = BondingCurve::fresh(Pubkey::new([1; 32]));
/// let fees = Fees::new(95, 30);
/// let buy = curve.buy_exact_sol_in(500_000_000, fees)?;
/// curve.apply_buy(&buy)?;
/// let sell = curve.sell_exact_tokens_in(buy.tokens_out, fees)?;
/// assert_eq!(
///     sell.to_string(),
///     "17376518.132293 tokens for 0.487654317 SOL (curve 0.493827158 - fees 0.004691359 - 0.001481482)"
/// );
/// # Ok::<(), pump_curve::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct SellQuote {
    /// Tokens sold, raw units.
    pub tokens_in: u64,
    /// Lamports that leave the curve's reserves (`sol_amount` in pump.fun's `TradeEvent`).
    pub sol_from_curve: u64,
    /// Protocol fee, lamports.
    pub protocol_fee: u64,
    /// Creator fee, lamports.
    pub creator_fee: u64,
    /// What the seller receives: `sol_from_curve - protocol_fee - creator_fee`.
    pub sol_out: u64,
}

impl fmt::Display for BuyQuote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} tokens for {} SOL (curve {} + fees {} + {})",
            DecimalAmount::tokens(self.tokens_out),
            DecimalAmount::sol(self.total_sol),
            DecimalAmount::sol(self.sol_to_curve),
            DecimalAmount::sol(self.protocol_fee),
            DecimalAmount::sol(self.creator_fee),
        )
    }
}

impl fmt::Display for SellQuote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} tokens for {} SOL (curve {} - fees {} - {})",
            DecimalAmount::tokens(self.tokens_in),
            DecimalAmount::sol(self.sol_out),
            DecimalAmount::sol(self.sol_from_curve),
            DecimalAmount::sol(self.protocol_fee),
            DecimalAmount::sol(self.creator_fee),
        )
    }
}

struct DecimalAmount {
    raw: u64,
    unit: u64,
    decimals: usize,
}

impl DecimalAmount {
    fn tokens(raw: u64) -> Self {
        Self { raw, unit: TOKEN_UNIT, decimals: TOKEN_DECIMALS.into() }
    }

    fn sol(raw: u64) -> Self {
        Self { raw, unit: LAMPORTS_PER_SOL, decimals: 9 }
    }
}

impl fmt::Display for DecimalAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:0width$}", self.raw / self.unit, self.raw % self.unit, width = self.decimals)
    }
}

fn to_u64(v: u128) -> Result<u64, Error> {
    u64::try_from(v).map_err(|_| Error::Overflow)
}

/// ceil(amount * bps / 10_000), as pump.fun charges each fee.
fn fee(amount: u128, bps: u64) -> Result<u128, Error> {
    let num = amount.checked_mul(bps as u128).ok_or(Error::Overflow)?;
    Ok(num.div_ceil_compat(BPS_DENOMINATOR as u128))
}

trait DivCeil {
    fn div_ceil_compat(self, d: Self) -> Self;
}
impl DivCeil for u128 {
    fn div_ceil_compat(self, d: u128) -> u128 {
        self / d + u128::from(self % d != 0)
    }
}

impl BondingCurve {
    /// The fees actually charged on this curve: the creator part dropped when it has no creator.
    pub fn charged_fees(&self, fees: Fees) -> Fees {
        if self.creator.is_some() {
            fees
        } else {
            Fees { creator_bps: 0, ..fees }
        }
    }

    fn tradable(&self) -> Result<(u128, u128), Error> {
        if self.complete {
            return Err(Error::CurveComplete);
        }
        if self.virtual_token_reserves == 0 || self.virtual_sol_reserves == 0 {
            return Err(Error::EmptyReserves);
        }
        Ok((self.virtual_token_reserves as u128, self.virtual_sol_reserves as u128))
    }

    /// Quotes `buy_exact_sol_in`: spend at most `spendable_lamports` (fees included) and see how many tokens come
    /// back. Follows the formula pump.fun publishes in its IDL for that instruction:
    ///
    /// 1. `net = floor(spendable * 10_000 / (10_000 + total_fee_bps))`
    /// 2. if `net` plus its fees (each rounded up) exceeds `spendable`, `net` is cut by the excess
    /// 3. `tokens = floor((net - 1) * virtual_tokens / (virtual_sol + net - 1))`
    ///
    /// Tokens are capped at the curve's real token reserves. When the cap bites, the program either fills
    /// partially (spending less) or fails, depending on the `partial_fill` flag; quote the capped amount with
    /// [`buy_exact_tokens_out`](BondingCurve::buy_exact_tokens_out) to see that cost.
    ///
    /// ```
    /// use pump_curve::{BondingCurve, Fees, Pubkey};
    ///
    /// let curve = BondingCurve::fresh(Pubkey::new([1; 32]));
    /// let q = curve.buy_exact_sol_in(1_000_000_000, Fees::new(95, 30))?;
    /// assert_eq!(q.total_sol, q.sol_to_curve + q.protocol_fee + q.creator_fee);
    /// assert!(q.total_sol <= 1_000_000_000);
    /// assert_eq!(q.tokens_out, 34_199_203_120_618); // about 34.2M tokens for 1 SOL at launch
    /// # Ok::<(), pump_curve::Error>(())
    /// ```
    pub fn buy_exact_sol_in(&self, spendable_lamports: u64, fees: Fees) -> Result<BuyQuote, Error> {
        let (vt, vs) = self.tradable()?;
        let f = self.charged_fees(fees);
        let total_bps = f.total_bps()? as u128;
        let spendable = spendable_lamports as u128;
        let mut net = spendable * 10_000 / (10_000 + total_bps);
        let fees_on =
            |n: u128| -> Result<(u128, u128), Error> { Ok((fee(n, f.protocol_bps)?, fee(n, f.creator_bps)?)) };
        let (p, c) = fees_on(net)?;
        if net + p + c > spendable {
            net = net.checked_sub(net + p + c - spendable).ok_or(Error::AmountTooSmall)?;
        }
        if net < 2 {
            return Err(Error::AmountTooSmall);
        }
        let (protocol_fee, creator_fee) = fees_on(net)?;
        let tokens = (net - 1).checked_mul(vt).ok_or(Error::Overflow)? / (vs + net - 1);
        let tokens = tokens.min(self.real_token_reserves as u128);
        if tokens == 0 {
            return Err(Error::AmountTooSmall);
        }
        Ok(BuyQuote {
            tokens_out: to_u64(tokens)?,
            sol_to_curve: to_u64(net)?,
            protocol_fee: to_u64(protocol_fee)?,
            creator_fee: to_u64(creator_fee)?,
            total_sol: to_u64(net + protocol_fee + creator_fee)?,
        })
    }

    /// Quotes `buy`: receive exactly `tokens` and see what it costs. Follows pump.fun's SDK
    /// (`getBuySolAmountFromTokenAmount`): `sol = floor(tokens * virtual_sol / (virtual_tokens - tokens)) + 1`,
    /// plus each fee rounded up. Pass `total_sol` (with slippage, see [`max_sol_cost`]) as the instruction's
    /// `max_sol_cost`.
    ///
    /// Errors with [`Error::InsufficientReserves`] when `tokens` exceeds the curve's real token reserves.
    ///
    /// ```
    /// use pump_curve::{BondingCurve, Fees, Pubkey};
    ///
    /// let curve = BondingCurve::fresh(Pubkey::new([1; 32]));
    /// let q = curve.buy_exact_tokens_out(10_000_000 * 1_000_000, Fees::new(95, 30))?; // 10M tokens
    /// assert_eq!(q.sol_to_curve, 282_220_132); // 0.282 SOL into the curve
    /// assert_eq!(q.total_sol, 285_747_885); // 0.286 SOL with 1.25% fees
    /// # Ok::<(), pump_curve::Error>(())
    /// ```
    pub fn buy_exact_tokens_out(&self, tokens: u64, fees: Fees) -> Result<BuyQuote, Error> {
        let (vt, vs) = self.tradable()?;
        let f = self.charged_fees(fees);
        f.total_bps()?;
        if tokens == 0 {
            return Err(Error::AmountTooSmall);
        }
        if tokens > self.real_token_reserves || tokens as u128 >= vt {
            return Err(Error::InsufficientReserves);
        }
        let t = tokens as u128;
        let sol = t.checked_mul(vs).ok_or(Error::Overflow)? / (vt - t) + 1;
        let protocol_fee = fee(sol, f.protocol_bps)?;
        let creator_fee = fee(sol, f.creator_bps)?;
        Ok(BuyQuote {
            tokens_out: tokens,
            sol_to_curve: to_u64(sol)?,
            protocol_fee: to_u64(protocol_fee)?,
            creator_fee: to_u64(creator_fee)?,
            total_sol: to_u64(sol + protocol_fee + creator_fee)?,
        })
    }

    /// Quotes `sell`: sell exactly `tokens` and see what comes back.
    /// `sol = floor(tokens * virtual_sol / (virtual_tokens + tokens))`, less each fee rounded up. Pass `sol_out`
    /// (with slippage, see [`min_sol_output`]) as the instruction's `min_sol_output`.
    ///
    /// Errors with [`Error::InsufficientReserves`] when the curve does not hold that much SOL, and
    /// [`Error::AmountTooSmall`] when the fees would eat the whole amount.
    ///
    /// ```
    /// use pump_curve::{BondingCurve, Fees, Pubkey};
    ///
    /// let mut curve = BondingCurve::fresh(Pubkey::new([1; 32]));
    /// let bought = curve.buy_exact_sol_in(1_000_000_000, Fees::new(95, 30))?;
    /// curve.apply_buy(&bought)?;
    /// let sold = curve.sell_exact_tokens_in(bought.tokens_out, Fees::new(95, 30))?;
    /// assert!(sold.sol_out < bought.total_sol); // the round trip costs the fees twice
    /// # Ok::<(), pump_curve::Error>(())
    /// ```
    pub fn sell_exact_tokens_in(&self, tokens: u64, fees: Fees) -> Result<SellQuote, Error> {
        let (vt, vs) = self.tradable()?;
        let f = self.charged_fees(fees);
        f.total_bps()?;
        if tokens == 0 {
            return Err(Error::AmountTooSmall);
        }
        let t = tokens as u128;
        let sol = t.checked_mul(vs).ok_or(Error::Overflow)? / (vt + t);
        if sol > self.real_sol_reserves as u128 {
            return Err(Error::InsufficientReserves);
        }
        let protocol_fee = fee(sol, f.protocol_bps)?;
        let creator_fee = fee(sol, f.creator_bps)?;
        let out = sol.checked_sub(protocol_fee + creator_fee).ok_or(Error::AmountTooSmall)?;
        if out == 0 {
            return Err(Error::AmountTooSmall);
        }
        Ok(SellQuote {
            tokens_in: tokens,
            sol_from_curve: to_u64(sol)?,
            protocol_fee: to_u64(protocol_fee)?,
            creator_fee: to_u64(creator_fee)?,
            sol_out: to_u64(out)?,
        })
    }

    /// Moves the reserves as a quoted buy would: SOL in, tokens out. Sets `complete` when the real tokens run out,
    /// as the program does. Lets you quote a sequence of trades, or check what a buy does to the price.
    pub fn apply_buy(&mut self, q: &BuyQuote) -> Result<(), Error> {
        let next = BondingCurve {
            virtual_sol_reserves: self.virtual_sol_reserves.checked_add(q.sol_to_curve).ok_or(Error::Overflow)?,
            real_sol_reserves: self.real_sol_reserves.checked_add(q.sol_to_curve).ok_or(Error::Overflow)?,
            virtual_token_reserves: self
                .virtual_token_reserves
                .checked_sub(q.tokens_out)
                .ok_or(Error::InsufficientReserves)?,
            real_token_reserves: self
                .real_token_reserves
                .checked_sub(q.tokens_out)
                .ok_or(Error::InsufficientReserves)?,
            ..*self
        };
        *self = BondingCurve { complete: next.real_token_reserves == 0, ..next };
        Ok(())
    }

    /// Moves the reserves as a quoted sell would: tokens in, SOL out.
    pub fn apply_sell(&mut self, q: &SellQuote) -> Result<(), Error> {
        *self = BondingCurve {
            virtual_sol_reserves: self.virtual_sol_reserves.checked_sub(q.sol_from_curve).ok_or(Error::Overflow)?,
            real_sol_reserves: self
                .real_sol_reserves
                .checked_sub(q.sol_from_curve)
                .ok_or(Error::InsufficientReserves)?,
            virtual_token_reserves: self.virtual_token_reserves.checked_add(q.tokens_in).ok_or(Error::Overflow)?,
            real_token_reserves: self.real_token_reserves.checked_add(q.tokens_in).ok_or(Error::Overflow)?,
            ..*self
        };
        Ok(())
    }
}

fn check_bps(slippage_bps: u64) -> Result<(), Error> {
    if slippage_bps > BPS_DENOMINATOR {
        Err(Error::InvalidBasisPoints)
    } else {
        Ok(())
    }
}

/// The `max_sol_cost` for a `buy`: the quoted cost plus `slippage_bps`, rounded up. Slippage above 10,000 bps
/// (100%) is refused.
///
/// ```
/// assert_eq!(pump_curve::max_sol_cost(1_000_000_000, 150)?, 1_015_000_000); // +1.5%
/// # Ok::<(), pump_curve::Error>(())
/// ```
pub fn max_sol_cost(quoted_lamports: u64, slippage_bps: u64) -> Result<u64, Error> {
    check_bps(slippage_bps)?;
    let num = quoted_lamports as u128 * (BPS_DENOMINATOR + slippage_bps) as u128;
    to_u64(num.div_ceil_compat(BPS_DENOMINATOR as u128))
}

/// The `min_sol_output` for a `sell`: the quoted proceeds less `slippage_bps`, rounded down.
///
/// ```
/// assert_eq!(pump_curve::min_sol_output(1_000_000_000, 150)?, 985_000_000); // -1.5%
/// # Ok::<(), pump_curve::Error>(())
/// ```
pub fn min_sol_output(quoted_lamports: u64, slippage_bps: u64) -> Result<u64, Error> {
    check_bps(slippage_bps)?;
    to_u64(quoted_lamports as u128 * (BPS_DENOMINATOR - slippage_bps) as u128 / BPS_DENOMINATOR as u128)
}

/// The `min_tokens_out` for `buy_exact_sol_in`: the quoted tokens less `slippage_bps`, rounded down.
///
/// ```
/// assert_eq!(pump_curve::min_tokens_out(2_000_000, 500)?, 1_900_000); // -5%
/// # Ok::<(), pump_curve::Error>(())
/// ```
pub fn min_tokens_out(quoted_tokens: u64, slippage_bps: u64) -> Result<u64, Error> {
    min_sol_output(quoted_tokens, slippage_bps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Pubkey;

    #[test]
    fn fee_rounds_up() {
        assert_eq!(fee(412_946, 95), Ok(3_923));
        assert_eq!(fee(10_000, 95), Ok(95));
        assert_eq!(fee(0, 95), Ok(0));
    }

    #[test]
    fn creator_fee_needs_a_creator() {
        let no_creator = BondingCurve::fresh(Pubkey::default());
        let q = no_creator.buy_exact_tokens_out(1_000_000_000, Fees::new(95, 30)).unwrap();
        assert_eq!(q.creator_fee, 0);
        assert!(q.protocol_fee > 0);
    }

    #[test]
    fn bad_inputs() {
        let c = BondingCurve::fresh(Pubkey([1; 32]));
        let fees = Fees::new(95, 30);
        assert_eq!(c.buy_exact_sol_in(1, fees), Err(Error::AmountTooSmall));
        assert_eq!(c.buy_exact_tokens_out(0, fees), Err(Error::AmountTooSmall));
        assert_eq!(c.buy_exact_tokens_out(c.real_token_reserves + 1, fees), Err(Error::InsufficientReserves));
        assert_eq!(c.sell_exact_tokens_in(0, fees), Err(Error::AmountTooSmall));
        // A fresh curve holds no real SOL, so nothing can be sold into it.
        assert_eq!(c.sell_exact_tokens_in(1_000_000_000, fees), Err(Error::InsufficientReserves));
        assert_eq!(c.buy_exact_sol_in(1_000_000, Fees::new(10_001, 0)), Err(Error::InvalidBasisPoints));
        let done = BondingCurve { complete: true, ..c };
        assert_eq!(done.buy_exact_sol_in(1_000_000, fees), Err(Error::CurveComplete));
        assert_eq!(BondingCurve::default().sell_exact_tokens_in(1, fees), Err(Error::EmptyReserves));
        assert_eq!(max_sol_cost(1, 10_001), Err(Error::InvalidBasisPoints));
        assert_eq!(max_sol_cost(u64::MAX, 10_000), Err(Error::Overflow));
        assert_eq!(min_sol_output(u64::MAX, 0), Ok(u64::MAX));
    }

    #[test]
    fn buying_everything_completes_the_curve() {
        let mut c = BondingCurve::fresh(Pubkey([1; 32]));
        let q = c.buy_exact_tokens_out(c.real_token_reserves, Fees::new(95, 30)).unwrap();
        c.apply_buy(&q).unwrap();
        assert!(c.complete);
        assert_eq!(c.progress_bps(), Some(10_000));
        // About 85 SOL goes into a standard curve before it completes.
        assert!((84_000_000_000..87_000_000_000).contains(&c.real_sol_reserves), "{}", c.real_sol_reserves);
    }

    #[test]
    fn fee_tiers() {
        assert_eq!(select_fee_tier(&[], 5), None);
        let tiers = [
            FeeTier { market_cap_lamports_threshold: 10, fees: Fees::new(1, 1) },
            FeeTier { market_cap_lamports_threshold: 20, fees: Fees::new(2, 2) },
            FeeTier { market_cap_lamports_threshold: 30, fees: Fees::new(3, 3) },
        ];
        assert_eq!(select_fee_tier(&tiers, 0), Some(Fees::new(1, 1)));
        assert_eq!(select_fee_tier(&tiers, 10), Some(Fees::new(1, 1)));
        assert_eq!(select_fee_tier(&tiers, 29), Some(Fees::new(2, 2)));
        assert_eq!(select_fee_tier(&tiers, 30), Some(Fees::new(3, 3)));
        assert_eq!(select_fee_tier(&tiers, u128::MAX), Some(Fees::new(3, 3)));
    }
}
