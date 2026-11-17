//! A 32-byte Solana address with base58 parsing and printing, and program-derived addresses.

use core::fmt;
use core::str::FromStr;

use crate::sha256::Sha256;
use crate::Error;

const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Seeds per program-derived address, the bump included (Solana's `MAX_SEEDS`).
pub const MAX_SEEDS: usize = 16;
/// Bytes per seed (Solana's `MAX_SEED_LEN`).
pub const MAX_SEED_LEN: usize = 32;

/// A Solana address: 32 bytes, printed in base58.
///
/// ```
/// use pump_curve::Pubkey;
///
/// let mint: Pubkey = "DxYWLXgKJpQmfhTG7ikK6oX6oDhxMhN5wCs5BJekpump".parse()?;
/// assert_eq!(mint.to_string(), "DxYWLXgKJpQmfhTG7ikK6oX6oDhxMhN5wCs5BJekpump");
/// # Ok::<(), pump_curve::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Pubkey(pub [u8; 32]);

impl Pubkey {
    /// Wraps raw bytes.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Pubkey(bytes)
    }

    /// The raw bytes.
    pub const fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Parses a base58 address. Errors with [`Error::InvalidBase58`] on a character outside the alphabet and
    /// [`Error::InvalidPubkeyLength`] when the text does not decode to exactly 32 bytes.
    pub fn from_base58(text: &str) -> Result<Self, Error> {
        let text = text.as_bytes();
        if text.is_empty() || text.len() > 44 {
            return Err(Error::InvalidPubkeyLength);
        }
        let mut out = [0u8; 32];
        for &ch in text {
            let digit = ALPHABET.iter().position(|&a| a == ch).ok_or(Error::InvalidBase58)? as u32;
            let mut carry = digit;
            for byte in out.iter_mut().rev() {
                carry += (*byte as u32) * 58;
                *byte = carry as u8;
                carry >>= 8;
            }
            if carry != 0 {
                return Err(Error::InvalidPubkeyLength);
            }
        }
        let ones = text.iter().take_while(|&&c| c == b'1').count();
        let zeros = out.iter().take_while(|&&b| b == 0).count();
        if ones + (32 - zeros) != 32 {
            return Err(Error::InvalidPubkeyLength);
        }
        Ok(Pubkey(out))
    }

    /// True for the all-zero address (`11111111111111111111111111111111`), which pump.fun stores for "none".
    pub fn is_default(&self) -> bool {
        self.0 == [0u8; 32]
    }

    /// Whether these bytes are a point on the ed25519 curve. Wallet addresses are; program-derived addresses are
    /// not. Same answer as `solana_program::pubkey::Pubkey::is_on_curve`.
    ///
    /// ```
    /// use pump_curve::{bonding_curve_address, Pubkey};
    ///
    /// let wallet: Pubkey = "BsU7rYPX4UmefGdTpCQyLPCoVSh5L3WkdeKbU3oBg4CG".parse()?;
    /// assert!(wallet.is_on_curve());
    /// let (curve, _) = bonding_curve_address(&"DxYWLXgKJpQmfhTG7ikK6oX6oDhxMhN5wCs5BJekpump".parse()?)?;
    /// assert!(!curve.is_on_curve());
    /// # Ok::<(), pump_curve::Error>(())
    /// ```
    pub fn is_on_curve(&self) -> bool {
        crate::ed25519::is_on_curve(&self.0)
    }

    /// Solana's `create_program_address`: hash the seeds, the program id and `"ProgramDerivedAddress"` and refuse a
    /// result that lands on the curve ([`Error::InvalidSeeds`]). At most [`MAX_SEEDS`] seeds of at most
    /// [`MAX_SEED_LEN`] bytes each.
    pub fn create_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> Result<Pubkey, Error> {
        if seeds.len() > MAX_SEEDS || seeds.iter().any(|s| s.len() > MAX_SEED_LEN) {
            return Err(Error::InvalidSeeds);
        }
        let mut h = Sha256::new();
        for s in seeds {
            h.update(s);
        }
        h.update(&program_id.0);
        h.update(b"ProgramDerivedAddress");
        let key = Pubkey(h.finish());
        if key.is_on_curve() {
            return Err(Error::InvalidSeeds);
        }
        Ok(key)
    }

    /// Solana's `find_program_address`: the first bump from 255 down to 1 whose address is off the curve. Returns
    /// the address and its bump. At most `MAX_SEEDS - 1` seeds, since the bump takes the last slot.
    ///
    /// ```
    /// use pump_curve::{Pubkey, PUMP_PROGRAM_ID};
    ///
    /// // pump.fun's Global account, as documented by pump.fun.
    /// let (global, bump) = Pubkey::find_program_address(&[b"global"], &PUMP_PROGRAM_ID)?;
    /// assert_eq!(global.to_string(), "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf");
    /// assert_eq!(bump, 255);
    /// # Ok::<(), pump_curve::Error>(())
    /// ```
    pub fn find_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> Result<(Pubkey, u8), Error> {
        if seeds.len() >= MAX_SEEDS || seeds.iter().any(|s| s.len() > MAX_SEED_LEN) {
            return Err(Error::InvalidSeeds);
        }
        for bump in (1..=255u8).rev() {
            let mut h = Sha256::new();
            for s in seeds {
                h.update(s);
            }
            h.update(&[bump]);
            h.update(&program_id.0);
            h.update(b"ProgramDerivedAddress");
            let key = Pubkey(h.finish());
            if !key.is_on_curve() {
                return Ok((key, bump));
            }
        }
        Err(Error::NoViableBump)
    }
}

impl FromStr for Pubkey {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        Pubkey::from_base58(s)
    }
}

impl From<[u8; 32]> for Pubkey {
    fn from(bytes: [u8; 32]) -> Self {
        Pubkey(bytes)
    }
}

impl AsRef<[u8]> for Pubkey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut digits = [0u8; 44];
        let mut len = 0;
        for &byte in &self.0 {
            let mut carry = byte as u32;
            for d in digits[..len].iter_mut() {
                carry += (*d as u32) << 8;
                *d = (carry % 58) as u8;
                carry /= 58;
            }
            while carry > 0 {
                digits[len] = (carry % 58) as u8;
                len += 1;
                carry /= 58;
            }
        }
        for _ in self.0.iter().take_while(|&&b| b == 0) {
            f.write_str("1")?;
        }
        for &d in digits[..len].iter().rev() {
            fmt::Write::write_char(f, ALPHABET[d as usize] as char)?;
        }
        Ok(())
    }
}

impl fmt::Debug for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pubkey({})", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base58_round_trip_and_errors() {
        for s in [
            "DxYWLXgKJpQmfhTG7ikK6oX6oDhxMhN5wCs5BJekpump",
            "11111111111111111111111111111111",
            "So11111111111111111111111111111111111111112",
            "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
        ] {
            let k: Pubkey = s.parse().unwrap();
            let mut buf = [0u8; 64];
            let mut w = Buf(&mut buf, 0);
            fmt::Write::write_fmt(&mut w, format_args!("{}", k)).unwrap();
            let n = w.1;
            assert_eq!(&buf[..n], s.as_bytes());
        }
        assert!(Pubkey::from_base58("11111111111111111111111111111111").unwrap().is_default());
        assert_eq!(Pubkey::from_base58("0OIl"), Err(Error::InvalidBase58));
        assert_eq!(Pubkey::from_base58("abc"), Err(Error::InvalidPubkeyLength));
        assert_eq!(Pubkey::from_base58(""), Err(Error::InvalidPubkeyLength));
        // 33 ones = 33 zero bytes.
        assert_eq!(Pubkey::from_base58("111111111111111111111111111111111"), Err(Error::InvalidPubkeyLength));
        // A 44-character string that overflows 32 bytes.
        assert_eq!(
            Pubkey::from_base58("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"),
            Err(Error::InvalidPubkeyLength)
        );
    }

    struct Buf<'a>(&'a mut [u8], usize);
    impl fmt::Write for Buf<'_> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let b = s.as_bytes();
            self.0[self.1..self.1 + b.len()].copy_from_slice(b);
            self.1 += b.len();
            Ok(())
        }
    }

    #[test]
    fn seeds_are_checked() {
        let p = Pubkey::default();
        assert_eq!(Pubkey::find_program_address(&[&[0u8; 33]], &p), Err(Error::InvalidSeeds));
        let many: [&[u8]; 16] = [b"a"; 16];
        assert_eq!(Pubkey::find_program_address(&many, &p), Err(Error::InvalidSeeds));
    }
}
