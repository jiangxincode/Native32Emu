// Shared types and axis-map builders used by all scaling algorithms.

pub mod bicubic;
pub mod bilinear;
pub mod xbrz;

// ── Bilinear axis map ────────────────────────────────────────────────────────

/// Per-axis entry for bilinear: source index + fractional weight (0..256).
pub struct BiAxisMap {
    pub src: u32,
    pub frac: u16,
}

pub fn build_bi_axis_map(src_size: u32, dst_size: u32) -> Vec<BiAxisMap> {
    let mut map = Vec::with_capacity(dst_size as usize);
    for d in 0..dst_size {
        let src_f = (d as f64 + 0.5) * src_size as f64 / dst_size as f64 - 0.5;
        let src_i = src_f.floor().max(0.0) as u32;
        let frac = ((src_f - src_i as f64) * 256.0).round().min(255.0) as u16;
        map.push(BiAxisMap {
            src: src_i.min(src_size - 1),
            frac,
        });
    }
    map
}

// ── Bicubic axis map ─────────────────────────────────────────────────────────

/// Fixed-point fractional bits for bicubic weights.
pub const FRAC_BITS: i32 = 10;
pub const FRAC_UNIT: i32 = 1 << FRAC_BITS;

/// Per-axis entry for bicubic: pre-computed clamped indices + fixed-point weights.
pub struct BcAxisMap {
    pub idx: [usize; 4],
    pub w: [i32; 4],
}

pub fn build_bc_axis_map(src_size: u32, dst_size: u32, clamp_size: u32) -> Vec<BcAxisMap> {
    let max_idx = clamp_size as usize - 1;
    let mut map = Vec::with_capacity(dst_size as usize);
    for d in 0..dst_size {
        let src_f = (d as f64 + 0.5) * src_size as f64 / dst_size as f64 - 0.5;
        let center = src_f.floor() as i32;
        let t = (src_f - center as f64) as f32;
        let w = catmull_rom_weights_fixed(t);
        let idx = [
            (center - 1).clamp(0, max_idx as i32) as usize,
            center.clamp(0, max_idx as i32) as usize,
            (center + 1).clamp(0, max_idx as i32) as usize,
            (center + 2).clamp(0, max_idx as i32) as usize,
        ];
        map.push(BcAxisMap { idx, w });
    }
    map
}

// ── Catmull-Rom kernel ───────────────────────────────────────────────────────

pub fn catmull_rom_weights_fixed(t: f32) -> [i32; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        ((-0.5 * t3 + t2 - 0.5 * t) * FRAC_UNIT as f32) as i32,
        ((1.5 * t3 - 2.5 * t2 + 1.0) * FRAC_UNIT as f32) as i32,
        ((-1.5 * t3 + 2.0 * t2 + 0.5 * t) * FRAC_UNIT as f32) as i32,
        ((0.5 * t3 - 0.5 * t2) * FRAC_UNIT as f32) as i32,
    ]
}

// ── Helpers ──────────────────────────────────────────────────────────────────

#[inline]
pub fn clamp_i32_u8(v: i32) -> u32 {
    v.clamp(0, 255) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catmull_rom_fixed_at_zero() {
        let w = catmull_rom_weights_fixed(0.0);
        assert!((w[1] - FRAC_UNIT).unsigned_abs() <= 1);
        assert!(w[0].unsigned_abs() <= 1);
        assert!(w[2].unsigned_abs() <= 1);
        assert!(w[3].unsigned_abs() <= 1);
    }

    #[test]
    fn catmull_rom_fixed_sum() {
        for t_int in 0..100 {
            let t = t_int as f32 / 100.0;
            let w = catmull_rom_weights_fixed(t);
            let sum = w[0] + w[1] + w[2] + w[3];
            assert!(
                (sum - FRAC_UNIT).unsigned_abs() <= 2,
                "sum={sum} expected={FRAC_UNIT} at t={t}"
            );
        }
    }

    #[test]
    fn build_axis_map_integer_scale() {
        let map = build_bi_axis_map(2, 4);
        assert_eq!(map[0].src, 0);
        assert_eq!(map[1].src, 0);
        assert!(map[2].src <= 1);
        assert_eq!(map[3].src, 1);
    }
}
