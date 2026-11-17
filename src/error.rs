use core::fmt;

/// Everything that can go wrong in this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The account data is shorter than the 49 bytes every bonding curve has.
    AccountTooShort {
        /// The length that was passed.
        len: usize,
    },
    /// The first 8 bytes are not the `BondingCurve` discriminator: this is some other account.
    WrongDiscriminator,
    /// The curve has completed (sold out and migrated or about to); it no longer trades.
    CurveComplete,
    /// The curve's virtual reserves are zero, so it has no price.
    EmptyReserves,
    /// The amount is too small to trade: zero, or so small that nothing comes out after rounding and fees.
    AmountTooSmall,
    /// The trade asks for more than the curve holds: more tokens than its real token reserves on a buy, or more SOL
    /// than its real SOL reserves on a sell.
    InsufficientReserves,
    /// A basis-point value is out of range: a fee or slippage above 10,000 where that makes no sense.
    InvalidBasisPoints,
    /// A result does not fit in a `u64` (or an intermediate in a `u128`).
    Overflow,
    /// A character that is not in the base58 alphabet.
    InvalidBase58,
    /// Base58 text that does not decode to exactly 32 bytes.
    InvalidPubkeyLength,
    /// Too many seeds, a seed longer than 32 bytes, or seeds whose address lands on the curve.
    InvalidSeeds,
    /// No bump from 255 down to 1 gives an address off the curve (astronomically unlikely).
    NoViableBump,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AccountTooShort { len } => {
                write!(f, "{len} bytes is too short to be a pump.fun bonding curve (needs at least 49)")
            }
            Error::WrongDiscriminator => f.write_str("not a pump.fun bonding curve account (wrong discriminator)"),
            Error::CurveComplete => f.write_str("the bonding curve is complete and no longer trades"),
            Error::EmptyReserves => f.write_str("the bonding curve has no virtual reserves"),
            Error::AmountTooSmall => f.write_str("the amount is too small to trade"),
            Error::InsufficientReserves => f.write_str("the trade is larger than the curve's real reserves"),
            Error::InvalidBasisPoints => f.write_str("basis points out of range"),
            Error::Overflow => f.write_str("arithmetic overflow"),
            Error::InvalidBase58 => f.write_str("not base58"),
            Error::InvalidPubkeyLength => f.write_str("not a 32-byte Solana address"),
            Error::InvalidSeeds => f.write_str("invalid program-address seeds"),
            Error::NoViableBump => f.write_str("no bump gives an address off the curve"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
