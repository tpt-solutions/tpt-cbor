//! Canonical floating-point helpers (RFC 8949 §4.2.2 "preferred serialization").
//!
//! A float is encoded at the smallest width that preserves its value exactly.
//! NaN is always encoded as the half-precision quiet NaN `0xf97e00`.

use crate::encode::Encoder;
use crate::error::EncodeError;

/// A float in its wire form: width plus raw bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Canon {
    Half(u16),
    Single(u32),
    Double(u64),
}

/// Returns the canonical wire form for `v`.
pub(crate) fn canonical(v: f64) -> Canon {
    if v.is_nan() {
        return Canon::Half(0x7e00);
    }
    if let Some(h) = to_half_exact(v) {
        return Canon::Half(h);
    }
    let single = v as f32;
    if f64::from(single) == v {
        return Canon::Single(single.to_bits());
    }
    Canon::Double(v.to_bits())
}

/// Converts `v` to IEEE 754 binary16 bits if the conversion is exact.
///
/// Returns `None` for NaN, for values that need more than 11 significant
/// bits, and for values outside the half-precision range (including f64
/// subnormals, which are far below the smallest half subnormal).
pub(crate) fn to_half_exact(v: f64) -> Option<u16> {
    if v.is_nan() {
        return None;
    }
    let bits = v.to_bits();
    let sign = ((bits >> 48) & 0x8000) as u16;
    if v.is_infinite() {
        return Some(sign | 0x7c00);
    }
    if v == 0.0 {
        return Some(sign);
    }
    let exp_field = ((bits >> 52) & 0x7ff) as i64;
    if exp_field == 0 {
        // f64 subnormal: magnitude < 2^-1022, far below half's 2^-24 minimum.
        return None;
    }
    let mantissa = bits & 0x000f_ffff_ffff_ffff;
    // exp_field is in 1..=2046 here, so this subtraction cannot overflow.
    let e = exp_field.checked_sub(1023)?;

    if (-14..=15).contains(&e) {
        // Normal half: the low 42 mantissa bits must be zero.
        if mantissa.trailing_zeros() < 42 {
            return None;
        }
        let biased = u16::try_from(e.checked_add(15)?).ok()?;
        let frac = (mantissa >> 42) as u16;
        return Some(sign | (biased << 10) | frac);
    }

    if (-24..=-15).contains(&e) {
        // Subnormal half: value = m * 2^-24 with 1 <= m <= 1023.
        // full * 2^(e-52) = m * 2^-24  =>  m = full >> (28 - e).
        let full = mantissa | (1u64 << 52);
        let shift = u32::try_from(28i64.checked_sub(e)?).ok()?;
        if full.trailing_zeros() < shift {
            return None;
        }
        let m = (full >> shift) as u16;
        return Some(sign | m);
    }

    None
}

/// Expands half-precision bits to `f64`. Exact for every input.
pub(crate) fn half_to_f64(h: u16) -> f64 {
    let sign_bit = (u64::from(h) & 0x8000) << 48;
    let exp = (h >> 10) & 0x1f;
    let frac = h & 0x3ff;
    match exp {
        0 => {
            // Subnormal or zero: frac * 2^-24. The scale is 2^-24 with biased
            // exponent 1023 - 24 = 999. Multiplying by a power of two is exact.
            let scale = f64::from_bits(999u64 << 52);
            let magnitude = f64::from(frac) * scale;
            if sign_bit == 0 {
                magnitude
            } else {
                -magnitude
            }
        }
        31 => {
            if frac == 0 {
                f64::from_bits(sign_bit | 0x7ff0_0000_0000_0000)
            } else {
                f64::NAN
            }
        }
        _ => {
            // Normal: biased f64 exponent = exp - 15 + 1023 = exp + 1008.
            let biased = u64::from(exp).wrapping_add(1008);
            f64::from_bits(sign_bit | (biased << 52) | (u64::from(frac) << 42))
        }
    }
}

/// Writes `v` in its canonical width.
pub(crate) fn encode(encoder: &mut Encoder<'_>, v: f64) -> Result<(), EncodeError> {
    match canonical(v) {
        Canon::Half(h) => {
            encoder.write_raw(&[0xf9])?;
            encoder.write_raw(&h.to_be_bytes())
        }
        Canon::Single(s) => {
            encoder.write_raw(&[0xfa])?;
            encoder.write_raw(&s.to_be_bytes())
        }
        Canon::Double(d) => {
            encoder.write_raw(&[0xfb])?;
            encoder.write_raw(&d.to_be_bytes())
        }
    }
}
