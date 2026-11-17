//! Just enough ed25519 to answer one question: do these 32 bytes decompress to a point on the curve? Wallet addresses
//! do; program-derived addresses must not. Field arithmetic mod p = 2^255 - 19 in five 51-bit limbs. Not constant
//! time; it only ever looks at public data.

const MASK: u64 = (1 << 51) - 1;

#[derive(Clone, Copy)]
struct Fe([u64; 5]);

impl Fe {
    const ONE: Fe = Fe([1, 0, 0, 0, 0]);

    /// The low 255 bits of a little-endian value; bit 255 (the sign of x) is ignored.
    fn from_bytes(b: &[u8; 32]) -> Fe {
        let load = |i: usize| {
            let mut w = [0u8; 8];
            w.copy_from_slice(&b[i..i + 8]);
            u64::from_le_bytes(w)
        };
        Fe([
            load(0) & MASK,
            (load(6) >> 3) & MASK,
            (load(12) >> 6) & MASK,
            (load(19) >> 1) & MASK,
            (load(24) >> 12) & MASK,
        ])
    }

    fn small(n: u64) -> Fe {
        Fe([n, 0, 0, 0, 0])
    }

    fn weak_reduce(mut l: [u64; 5]) -> Fe {
        let c = [l[0] >> 51, l[1] >> 51, l[2] >> 51, l[3] >> 51, l[4] >> 51];
        for limb in l.iter_mut() {
            *limb &= MASK;
        }
        l[0] += c[4] * 19;
        l[1] += c[0];
        l[2] += c[1];
        l[3] += c[2];
        l[4] += c[3];
        Fe(l)
    }

    fn add(self, o: Fe) -> Fe {
        let mut l = self.0;
        for (a, b) in l.iter_mut().zip(o.0) {
            *a += b;
        }
        Fe::weak_reduce(l)
    }

    fn sub(self, o: Fe) -> Fe {
        // Add 16p first so no limb goes negative.
        const P16_0: u64 = 36_028_797_018_963_664;
        const P16_N: u64 = 36_028_797_018_963_952;
        let a = self.0;
        let b = o.0;
        Fe::weak_reduce([
            a[0] + P16_0 - b[0],
            a[1] + P16_N - b[1],
            a[2] + P16_N - b[2],
            a[3] + P16_N - b[3],
            a[4] + P16_N - b[4],
        ])
    }

    fn neg(self) -> Fe {
        Fe([0; 5]).sub(self)
    }

    fn mul(self, o: Fe) -> Fe {
        let a = self.0;
        let b = o.0;
        let m = |x: u64, y: u64| (x as u128) * (y as u128);
        let (b1, b2, b3, b4) = (b[1] * 19, b[2] * 19, b[3] * 19, b[4] * 19);
        let c0 = m(a[0], b[0]) + m(a[4], b1) + m(a[3], b2) + m(a[2], b3) + m(a[1], b4);
        let mut c1 = m(a[1], b[0]) + m(a[0], b[1]) + m(a[4], b2) + m(a[3], b3) + m(a[2], b4);
        let mut c2 = m(a[2], b[0]) + m(a[1], b[1]) + m(a[0], b[2]) + m(a[4], b3) + m(a[3], b4);
        let mut c3 = m(a[3], b[0]) + m(a[2], b[1]) + m(a[1], b[2]) + m(a[0], b[3]) + m(a[4], b4);
        let mut c4 = m(a[4], b[0]) + m(a[3], b[1]) + m(a[2], b[2]) + m(a[1], b[3]) + m(a[0], b[4]);
        let mask = MASK as u128;
        c1 += c0 >> 51;
        c2 += c1 >> 51;
        c3 += c2 >> 51;
        c4 += c3 >> 51;
        let r0 = (c0 & mask) + (c4 >> 51) * 19;
        let r1 = (c1 & mask) + (r0 >> 51);
        Fe([(r0 & mask) as u64, r1 as u64, (c2 & mask) as u64, (c3 & mask) as u64, (c4 & mask) as u64])
    }

    fn square(self) -> Fe {
        self.mul(self)
    }

    /// self^e, with e given as 32 little-endian bytes.
    fn pow(self, e: &[u8; 32]) -> Fe {
        let mut r = Fe::ONE;
        for i in (0..256).rev() {
            r = r.square();
            if (e[i / 8] >> (i % 8)) & 1 == 1 {
                r = r.mul(self);
            }
        }
        r
    }

    /// The unique representative in [0, p).
    fn canonical(self) -> [u64; 5] {
        let mut l = Fe::weak_reduce(Fe::weak_reduce(self.0).0).0;
        let mut q = (l[0] + 19) >> 51;
        q = (l[1] + q) >> 51;
        q = (l[2] + q) >> 51;
        q = (l[3] + q) >> 51;
        q = (l[4] + q) >> 51;
        l[0] += 19 * q;
        l[1] += l[0] >> 51;
        l[0] &= MASK;
        l[2] += l[1] >> 51;
        l[1] &= MASK;
        l[3] += l[2] >> 51;
        l[2] &= MASK;
        l[4] += l[3] >> 51;
        l[3] &= MASK;
        l[4] &= MASK;
        l
    }

    fn equals(self, o: Fe) -> bool {
        self.canonical() == o.canonical()
    }
}

/// p - 2, little endian: the exponent that inverts.
const P_MINUS_2: [u8; 32] = {
    let mut e = [0xffu8; 32];
    e[0] = 0xeb;
    e[31] = 0x7f;
    e
};

/// (p - 5) / 8 = 2^252 - 3, little endian: the exponent of the square-root candidate.
const P_MINUS_5_OVER_8: [u8; 32] = {
    let mut e = [0xffu8; 32];
    e[0] = 0xfd;
    e[31] = 0x0f;
    e
};

/// Whether `bytes`, read as a compressed Edwards y coordinate, decompress to a point on ed25519. Mirrors
/// `curve25519-dalek`'s `CompressedEdwardsY::decompress().is_some()`, which is what Solana's `Pubkey::is_on_curve`
/// calls.
pub(crate) fn is_on_curve(bytes: &[u8; 32]) -> bool {
    // d = -121665 / 121666
    let d = Fe::small(121_665).neg().mul(Fe::small(121_666).pow(&P_MINUS_2));
    let y = Fe::from_bytes(bytes);
    let y2 = y.square();
    let u = y2.sub(Fe::ONE);
    let v = d.mul(y2).add(Fe::ONE);
    // x^2 = u / v has a root exactly when the candidate below squares back to u or -u.
    let v3 = v.square().mul(v);
    let v7 = v3.square().mul(v);
    let x = u.mul(v3).mul(u.mul(v7).pow(&P_MINUS_5_OVER_8));
    let vx2 = v.mul(x.square());
    vx2.equals(u) || vx2.equals(u.neg())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_basics() {
        let a = Fe::small(123_456_789);
        let inv = a.pow(&P_MINUS_2);
        assert!(a.mul(inv).equals(Fe::ONE));
        assert!(a.sub(a).equals(Fe([0; 5])));
        assert!(a.add(a.neg()).equals(Fe([0; 5])));
        // p itself reads as zero.
        let mut p = [0xffu8; 32];
        p[0] = 0xed;
        p[31] = 0x7f;
        assert!(Fe::from_bytes(&p).equals(Fe([0; 5])));
    }

    #[test]
    fn known_points() {
        // The ed25519 base point's compressed y (RFC 8032).
        let base = [
            0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
            0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
        ];
        assert!(is_on_curve(&base));
        // y = 1 is the identity; y = 0 has x^2 = -1, which has a root.
        let mut one = [0u8; 32];
        one[0] = 1;
        assert!(is_on_curve(&one));
        assert!(is_on_curve(&[0u8; 32]));
        // y = 2 is not on the curve.
        let mut two = [0u8; 32];
        two[0] = 2;
        assert!(!is_on_curve(&two));
    }
}
