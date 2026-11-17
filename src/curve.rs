//! The `BondingCurve` account: decoding and the figures that follow from its reserves.

use crate::{
    Error, Pubkey, BONDING_CURVE_DISCRIMINATOR, INITIAL_REAL_TOKEN_RESERVES, INITIAL_VIRTUAL_SOL_RESERVES,
    INITIAL_VIRTUAL_TOKEN_RESERVES, LAMPORTS_PER_SOL, TOKEN_TOTAL_SUPPLY, TOKEN_UNIT, WSOL_MINT,
};

/// A decoded pump.fun `BondingCurve` account.
///
/// Field order and types follow pump.fun's published IDL
/// (<https://github.com/pump-fun/pump-public-docs/blob/main/idl/pump.json>). Fields were appended over time;
/// accounts written before a field existed are shorter, and a missing field reads as zero, `false` or `None`, as
/// pump.fun's docs instruct.
///
/// Amounts are raw integers: tokens in 10^-6 units, SOL in lamports. On a curve quoted in another token
/// (`quote_mint` is `Some`), the "sol" reserves are in that token's raw units; the IDL calls them
/// `virtual_quote_reserves` / `real_quote_reserves`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BondingCurve {
    /// Virtual token reserves: the token side of the constant product.
    pub virtual_token_reserves: u64,
    /// Virtual SOL (quote) reserves: the SOL side of the constant product, 30 SOL of it synthetic at launch.
    pub virtual_sol_reserves: u64,
    /// Tokens the curve can still sell.
    pub real_token_reserves: u64,
    /// SOL (quote) the curve actually holds from buys.
    pub real_sol_reserves: u64,
    /// The coin's total supply in raw units (10^15 for a standard coin).
    pub token_total_supply: u64,
    /// Set once the curve has sold out; it then migrates to a pool and stops trading.
    pub complete: bool,
    /// The creator the creator fee accrues to. `None` on accounts too old to have the field, or when it is unset.
    pub creator: Option<Pubkey>,
    /// Whether the coin is a "mayhem mode" coin. Its curve parameters differ from the standard ones.
    pub is_mayhem_mode: bool,
    /// Whether the coin routes its creator fee to buyers as cashback (deprecated by pump.fun).
    pub is_cashback_coin: bool,
    /// The token the curve is priced in. `None` means SOL (unset or absent); see also [`BondingCurve::is_sol_quoted`].
    pub quote_mint: Option<Pubkey>,
    /// The coin's own creator fee rate, for coins on a custom pair. `0` means the standard schedule applies.
    pub creator_fee_bps: u64,
    /// Reserved by pump.fun; always `false` so far.
    pub can_edit_creator_fee: bool,
    /// Whether the creator fee is set aside for holders instead of a creator wallet.
    pub is_holder_reward: bool,
    /// Creator fees waiting on the curve to be swept (v3 trades keep them there).
    pub creator_fee: u64,
    /// Protocol fees waiting on the curve to be swept.
    pub protocol_fees: u64,
    /// How many pump coins deep the quote chain is (coins quoted in another pump coin).
    pub depth: u8,
    /// The quote reserves the curve was created with.
    pub initial_virtual_quote_reserves: u64,
    /// Tokens bought past completion by the v3 post-completion leg.
    pub post_complete_base_out: u64,
    /// Quote paid in by the v3 post-completion leg.
    pub post_complete_quote_in: u64,
}

impl BondingCurve {
    /// Bytes in the original layout: discriminator, five `u64`s and `complete`.
    pub const MIN_LEN: usize = 49;
    /// Bytes in the full layout this crate knows (pump.fun IDL, November 2026). Longer accounts decode fine; the
    /// extra bytes are ignored.
    pub const LEN: usize = 166;

    /// Decodes a bonding-curve account's data (the bytes from `getAccountInfo`, base64-decoded).
    ///
    /// Checks the discriminator and the 49-byte minimum, then reads every later field that is fully present.
    /// It does not check the account's owner: make sure it is [`crate::PUMP_PROGRAM_ID`] before trusting the result.
    ///
    /// ```
    /// use pump_curve::{BondingCurve, Error};
    ///
    /// let curve = BondingCurve::fresh(pump_curve::Pubkey::new([7; 32]));
    /// let decoded = BondingCurve::decode(&curve.to_bytes())?;
    /// assert_eq!(decoded, curve);
    ///
    /// assert_eq!(BondingCurve::decode(&curve.to_bytes()[..10]), Err(Error::AccountTooShort { len: 10 }));
    /// assert_eq!(BondingCurve::decode(&[0u8; 166]), Err(Error::WrongDiscriminator));
    /// # Ok::<(), Error>(())
    /// ```
    pub fn decode(data: &[u8]) -> Result<Self, Error> {
        if data.len() >= 8 && data[..8] != BONDING_CURVE_DISCRIMINATOR {
            return Err(Error::WrongDiscriminator);
        }
        if data.len() < Self::MIN_LEN {
            return Err(Error::AccountTooShort { len: data.len() });
        }
        let u64_at = |at: usize| -> u64 {
            match data.get(at..at + 8) {
                Some(b) => u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
                None => 0,
            }
        };
        let bool_at = |at: usize| data.get(at).is_some_and(|&b| b != 0);
        let key_at = |at: usize| -> Option<Pubkey> {
            let b = data.get(at..at + 32)?;
            let mut k = [0u8; 32];
            k.copy_from_slice(b);
            let k = Pubkey(k);
            if k.is_default() {
                None
            } else {
                Some(k)
            }
        };
        Ok(BondingCurve {
            virtual_token_reserves: u64_at(8),
            virtual_sol_reserves: u64_at(16),
            real_token_reserves: u64_at(24),
            real_sol_reserves: u64_at(32),
            token_total_supply: u64_at(40),
            complete: bool_at(48),
            creator: key_at(49),
            is_mayhem_mode: bool_at(81),
            is_cashback_coin: bool_at(82),
            quote_mint: key_at(83),
            creator_fee_bps: u64_at(115),
            can_edit_creator_fee: bool_at(123),
            is_holder_reward: bool_at(124),
            creator_fee: u64_at(125),
            protocol_fees: u64_at(133),
            depth: data.get(141).copied().unwrap_or(0),
            initial_virtual_quote_reserves: u64_at(142),
            post_complete_base_out: u64_at(150),
            post_complete_quote_in: u64_at(158),
        })
    }

    /// Encodes the curve in the full 166-byte layout. Handy for tests and fixtures; the inverse of [`decode`].
    ///
    /// [`decode`]: BondingCurve::decode
    pub fn to_bytes(&self) -> [u8; Self::LEN] {
        let mut b = [0u8; Self::LEN];
        b[..8].copy_from_slice(&BONDING_CURVE_DISCRIMINATOR);
        let mut put = |at: usize, v: u64| b[at..at + 8].copy_from_slice(&v.to_le_bytes());
        put(8, self.virtual_token_reserves);
        put(16, self.virtual_sol_reserves);
        put(24, self.real_token_reserves);
        put(32, self.real_sol_reserves);
        put(40, self.token_total_supply);
        put(115, self.creator_fee_bps);
        put(125, self.creator_fee);
        put(133, self.protocol_fees);
        put(142, self.initial_virtual_quote_reserves);
        put(150, self.post_complete_base_out);
        put(158, self.post_complete_quote_in);
        b[48] = self.complete as u8;
        b[49..81].copy_from_slice(&self.creator.unwrap_or_default().0);
        b[81] = self.is_mayhem_mode as u8;
        b[82] = self.is_cashback_coin as u8;
        b[83..115].copy_from_slice(&self.quote_mint.unwrap_or_default().0);
        b[123] = self.can_edit_creator_fee as u8;
        b[124] = self.is_holder_reward as u8;
        b[141] = self.depth;
        b
    }

    /// A brand-new standard SOL curve, built from the `Global` launch values pump.fun documents: 1,073,000,000
    /// virtual tokens, 30 virtual SOL, 793,100,000 tokens for sale, 1,000,000,000 supply. Useful to quote a coin's
    /// first buy before its account exists.
    pub fn fresh(creator: Pubkey) -> Self {
        BondingCurve {
            virtual_token_reserves: INITIAL_VIRTUAL_TOKEN_RESERVES,
            virtual_sol_reserves: INITIAL_VIRTUAL_SOL_RESERVES,
            real_token_reserves: INITIAL_REAL_TOKEN_RESERVES,
            token_total_supply: TOKEN_TOTAL_SUPPLY,
            creator: if creator.is_default() { None } else { Some(creator) },
            initial_virtual_quote_reserves: INITIAL_VIRTUAL_SOL_RESERVES,
            ..BondingCurve::default()
        }
    }

    /// True when the curve is priced in SOL: `quote_mint` unset or wrapped SOL.
    pub fn is_sol_quoted(&self) -> bool {
        match self.quote_mint {
            None => true,
            Some(m) => m == WSOL_MINT,
        }
    }

    /// True when pump.fun's migration has emptied the curve: every reserve zero (the SDK's `isBondingCurveMigrated`).
    pub fn is_migrated(&self) -> bool {
        self.virtual_token_reserves == 0
            && self.virtual_sol_reserves == 0
            && self.real_token_reserves == 0
            && self.real_sol_reserves == 0
    }

    /// Market cap in lamports (quote units on a non-SOL curve) for a given mint supply, the way pump.fun's fee
    /// program computes it: `virtual_sol_reserves * mint_supply / virtual_token_reserves`, rounded down.
    ///
    /// For fee-tier selection pump.fun's SDK passes `mint_supply` = 10^15 (one billion tokens) on standard coins
    /// and the mint's supply on mayhem coins.
    ///
    /// ```
    /// use pump_curve::{BondingCurve, Pubkey, TOKEN_TOTAL_SUPPLY};
    ///
    /// let curve = BondingCurve::fresh(Pubkey::default());
    /// // 30 SOL * 1e9 / 1.073e9 = 27.96 SOL
    /// assert_eq!(curve.market_cap_lamports(TOKEN_TOTAL_SUPPLY)?, 27_958_993_476);
    /// # Ok::<(), pump_curve::Error>(())
    /// ```
    pub fn market_cap_lamports(&self, mint_supply: u64) -> Result<u128, Error> {
        if self.virtual_token_reserves == 0 {
            return Err(Error::EmptyReserves);
        }
        Ok(self.virtual_sol_reserves as u128 * mint_supply as u128 / self.virtual_token_reserves as u128)
    }

    fn supply_or_default(&self) -> u64 {
        if self.token_total_supply == 0 {
            TOKEN_TOTAL_SUPPLY
        } else {
            self.token_total_supply
        }
    }

    fn priced(&self) -> bool {
        !self.complete && self.is_sol_quoted() && self.virtual_token_reserves > 0 && self.virtual_sol_reserves > 0
    }

    /// The spot price in SOL per whole token, as a float for display. `None` once complete, on a curve quoted in
    /// another token, or with empty reserves. Use the quote methods for anything that moves money.
    pub fn price_sol(&self) -> Option<f64> {
        if !self.priced() {
            return None;
        }
        let sol = self.virtual_sol_reserves as f64 / LAMPORTS_PER_SOL as f64;
        let tokens = self.virtual_token_reserves as f64 / TOKEN_UNIT as f64;
        Some(sol / tokens)
    }

    /// Market cap in SOL, as a float for display: spot price times `token_total_supply` (one billion tokens if the
    /// field is zero). `None` whenever [`price_sol`](BondingCurve::price_sol) is.
    pub fn market_cap_sol(&self) -> Option<f64> {
        let price = self.price_sol()?;
        Some(price * (self.supply_or_default() as f64 / TOKEN_UNIT as f64))
    }

    /// Curve progress in basis points (0 to 10,000): the share of the curve's sellable tokens already bought.
    /// 10,000 once complete.
    ///
    /// Computed as `sold / (sold + real_token_reserves)` with `sold = 1,073,000,000 - virtual_token_reserves`
    /// tokens, which holds for standard curves. `None` for mayhem-mode coins (their launch parameters differ) and
    /// when nothing is sold and nothing is left.
    pub fn progress_bps(&self) -> Option<u16> {
        if self.complete {
            return Some(10_000);
        }
        if self.is_mayhem_mode {
            return None;
        }
        let sold = INITIAL_VIRTUAL_TOKEN_RESERVES.saturating_sub(self.virtual_token_reserves) as u128;
        let total = sold + self.real_token_reserves as u128;
        if total == 0 {
            return None;
        }
        Some((sold * 10_000 / total) as u16)
    }

    /// [`progress_bps`](BondingCurve::progress_bps) as a percentage (0.0 to 100.0).
    pub fn progress_pct(&self) -> Option<f64> {
        self.progress_bps().map(|bps| bps as f64 / 100.0)
    }

    /// The market cap in SOL at which this curve completes: the price the same constant product reaches once the
    /// remaining real tokens are sold, times the supply. About 411 SOL for a standard curve. A float for display;
    /// `None` whenever [`price_sol`](BondingCurve::price_sol) is, or when the curve has no tokens past its real
    /// reserves.
    pub fn migration_market_cap_sol(&self) -> Option<f64> {
        if !self.priced() {
            return None;
        }
        let end_tokens = self.virtual_token_reserves.checked_sub(self.real_token_reserves)?;
        if end_tokens == 0 {
            return None;
        }
        let k = self.virtual_sol_reserves as f64 / LAMPORTS_PER_SOL as f64 * self.virtual_token_reserves as f64
            / TOKEN_UNIT as f64;
        let end = end_tokens as f64 / TOKEN_UNIT as f64;
        Some(k / (end * end) * (self.supply_or_default() as f64 / TOKEN_UNIT as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_layout_length_decodes() {
        let full = BondingCurve {
            virtual_token_reserves: 900_000_000_000_000,
            virtual_sol_reserves: 35_000_000_000,
            real_token_reserves: 620_100_000_000_000,
            real_sol_reserves: 5_000_000_000,
            token_total_supply: TOKEN_TOTAL_SUPPLY,
            complete: false,
            creator: Some(Pubkey([9; 32])),
            is_mayhem_mode: true,
            is_cashback_coin: true,
            quote_mint: Some(Pubkey([5; 32])),
            creator_fee_bps: 40,
            can_edit_creator_fee: true,
            is_holder_reward: true,
            creator_fee: 11,
            protocol_fees: 12,
            depth: 2,
            initial_virtual_quote_reserves: 13,
            post_complete_base_out: 14,
            post_complete_quote_in: 15,
        };
        let bytes = full.to_bytes();
        assert_eq!(BondingCurve::decode(&bytes), Ok(full));
        // Original 49 bytes: only the reserves and `complete`.
        let old = BondingCurve::decode(&bytes[..49]).unwrap();
        assert_eq!(old.virtual_sol_reserves, 35_000_000_000);
        assert_eq!(old.creator, None);
        assert_eq!(old.quote_mint, None);
        // 81 bytes: creator, nothing after.
        let with_creator = BondingCurve::decode(&bytes[..81]).unwrap();
        assert_eq!(with_creator.creator, Some(Pubkey([9; 32])));
        assert!(!with_creator.is_mayhem_mode);
        // 115 bytes: up to the quote mint.
        let with_quote = BondingCurve::decode(&bytes[..115]).unwrap();
        assert_eq!(with_quote.quote_mint, Some(Pubkey([5; 32])));
        assert_eq!(with_quote.creator_fee_bps, 0);
        // A field cut in half reads as absent; longer accounts ignore the tail.
        assert_eq!(BondingCurve::decode(&bytes[..120]).unwrap().creator_fee_bps, 0);
        let mut longer = [0u8; 200];
        longer[..166].copy_from_slice(&bytes);
        assert_eq!(BondingCurve::decode(&longer), Ok(full));
        assert_eq!(BondingCurve::decode(&bytes[..48]), Err(Error::AccountTooShort { len: 48 }));
        assert_eq!(BondingCurve::decode(&bytes[..4]), Err(Error::AccountTooShort { len: 4 }));
    }

    #[test]
    fn fresh_curve_figures() {
        let c = BondingCurve::fresh(Pubkey([1; 32]));
        assert_eq!(c.progress_bps(), Some(0));
        let cap = c.market_cap_sol().unwrap();
        assert!((cap - 27.958).abs() < 0.001, "{cap}");
        let end = c.migration_market_cap_sol().unwrap();
        assert!((end - 410.9).abs() < 0.5, "{end}");
        assert!(c.is_sol_quoted());
    }

    #[test]
    fn complete_and_other_quote_curves_have_no_sol_price() {
        let done = BondingCurve { complete: true, ..BondingCurve::fresh(Pubkey::default()) };
        assert_eq!(done.price_sol(), None);
        assert_eq!(done.progress_bps(), Some(10_000));
        let other = BondingCurve { quote_mint: Some(Pubkey([3; 32])), ..BondingCurve::fresh(Pubkey::default()) };
        assert_eq!(other.price_sol(), None);
        assert_eq!(other.progress_bps(), Some(0));
        let wsol = BondingCurve { quote_mint: Some(WSOL_MINT), ..BondingCurve::fresh(Pubkey::default()) };
        assert!(wsol.is_sol_quoted());
        let empty = BondingCurve::default();
        assert_eq!(empty.market_cap_lamports(1), Err(Error::EmptyReserves));
        assert!(empty.is_migrated());
    }
}
