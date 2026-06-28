// Software scaler: bilinear and bicubic interpolation on ARGB u32 buffers.
//
// Bilinear uses a 2x2 neighbourhood with fixed-point weights.
// Bicubic uses a *separable* two-pass Catmull-Rom filter (horizontal then
// vertical) to halve the per-pixel work vs. a direct 4x4 kernel. All weights
// and source indices are pre-computed; the inner loops are pure integer
// arithmetic with no floating-point or per-pixel function calls.

/// Scaling filter algorithm.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScaleFilter {
    /// Bilinear (2x2 neighbourhood, fixed-point weights).
    Bilinear,
    /// Bicubic Catmull-Rom (separable 4-tap, sharper edges).
    Bicubic,
}

// ── Bilateral axis map ───────────────────────────────────────────────────────

struct BiAxisMap {
    src: u32,
    frac: u16,
}

// ── Bicubic axis map (pre-computed weights + clamped source indices) ─────────

/// Pre-computed per-pixel data for one axis of the separable bicubic filter.
struct BcAxisMap {
    /// Clamped source indices for the 4 taps.
    idx: [usize; 4],
    /// Catmull-Rom weights in i32 fixed-point.
    w: [i32; 4],
}

/// Fixed-point fractional bits for bicubic weights.
const FRAC_BITS: i32 = 10;
const FRAC_UNIT: i32 = 1 << FRAC_BITS;

// ── Scaler ───────────────────────────────────────────────────────────────────

pub struct Scaler {
    filter: ScaleFilter,
    // Bilinear maps.
    bi_x: Vec<BiAxisMap>,
    bi_y: Vec<BiAxisMap>,
    // Bicubic maps (pre-computed weights + clamped indices).
    bc_x: Vec<BcAxisMap>,
    bc_y: Vec<BcAxisMap>,
    // Intermediate buffer for separable bicubic (dst_w × src_h).
    intermediate: Vec<u32>,
    // Current dimensions.
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    output: Vec<u32>,
}

impl Scaler {
    pub fn new() -> Self {
        Self {
            filter: ScaleFilter::Bilinear,
            bi_x: Vec::new(),
            bi_y: Vec::new(),
            bc_x: Vec::new(),
            bc_y: Vec::new(),
            intermediate: Vec::new(),
            src_w: 0,
            src_h: 0,
            dst_w: 0,
            dst_h: 0,
            output: Vec::new(),
        }
    }

    pub fn set_filter(&mut self, filter: ScaleFilter) {
        self.filter = filter;
    }

    pub fn scale(&mut self, src: &[u32], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> &[u32] {
        if src_w != self.src_w || src_h != self.src_h || dst_w != self.dst_w || dst_h != self.dst_h
        {
            self.bi_x = build_bi_axis_map(src_w, dst_w);
            self.bi_y = build_bi_axis_map(src_h, dst_h);
            self.bc_x = build_bc_axis_map(src_w, dst_w, src_w);
            self.bc_y = build_bc_axis_map(src_h, dst_h, src_h);
            self.intermediate = vec![0u32; (dst_w * src_h) as usize];
            self.src_w = src_w;
            self.src_h = src_h;
            self.dst_w = dst_w;
            self.dst_h = dst_h;
            self.output = vec![0u32; (dst_w * dst_h) as usize];
        }

        match self.filter {
            ScaleFilter::Bilinear => self.scale_bilinear(src, src_w, src_h),
            ScaleFilter::Bicubic => self.scale_bicubic_sep(src, src_w, src_h),
        }

        &self.output
    }

    // ── Bilinear (unchanged) ──────────────────────────────────────────────

    fn scale_bilinear(&mut self, src: &[u32], src_w: u32, src_h: u32) {
        let sw = src_w as usize;
        let sh = src_h as usize;
        let dw = self.dst_w as usize;

        for dy in 0..self.dst_h as usize {
            let ym = &self.bi_y[dy];
            let sy0 = ym.src as usize;
            let sy1 = (sy0 + 1).min(sh - 1);
            let fy = ym.frac as u32;
            let fy_inv = 256 - fy;

            let row0 = &src[sy0 * sw..sy0 * sw + sw];
            let row1 = &src[sy1 * sw..sy1 * sw + sw];
            let dst_row = &mut self.output[dy * dw..dy * dw + dw];

            for (dx, pixel) in dst_row.iter_mut().enumerate() {
                let xm = &self.bi_x[dx];
                let sx0 = xm.src as usize;
                let sx1 = (sx0 + 1).min(sw - 1);
                let fx = xm.frac as u32;
                let fx_inv = 256 - fx;

                let p00 = row0[sx0];
                let p10 = row0[sx1];
                let p01 = row1[sx0];
                let p11 = row1[sx1];

                let w00 = fx_inv * fy_inv;
                let w10 = fx * fy_inv;
                let w01 = fx_inv * fy;
                let w11 = fx * fy;

                let a = ((p00 >> 24) & 0xFF) * w00
                    + ((p10 >> 24) & 0xFF) * w10
                    + ((p01 >> 24) & 0xFF) * w01
                    + ((p11 >> 24) & 0xFF) * w11;
                let r = ((p00 >> 16) & 0xFF) * w00
                    + ((p10 >> 16) & 0xFF) * w10
                    + ((p01 >> 16) & 0xFF) * w01
                    + ((p11 >> 16) & 0xFF) * w11;
                let g = ((p00 >> 8) & 0xFF) * w00
                    + ((p10 >> 8) & 0xFF) * w10
                    + ((p01 >> 8) & 0xFF) * w01
                    + ((p11 >> 8) & 0xFF) * w11;
                let b = (p00 & 0xFF) * w00
                    + (p10 & 0xFF) * w10
                    + (p01 & 0xFF) * w01
                    + (p11 & 0xFF) * w11;

                *pixel = ((a >> 16) << 24) | ((r >> 16) << 16) | ((g >> 16) << 8) | (b >> 16);
            }
        }
    }

    // ── Bicubic separable (horizontal → vertical) ─────────────────────────

    fn scale_bicubic_sep(&mut self, src: &[u32], src_w: u32, src_h: u32) {
        let sw = src_w as usize;
        let dw = self.dst_w as usize;

        // Pass 1: horizontal  (src_w × src_h → dst_w × src_h)
        for sy in 0..src_h as usize {
            let src_row = &src[sy * sw..sy * sw + sw];
            let dst_row = &mut self.intermediate[sy * dw..sy * dw + dw];

            for (dx, pixel) in dst_row.iter_mut().enumerate() {
                let xm = &self.bc_x[dx];
                let idx = &xm.idx;
                let wx = &xm.w;

                let p0 = src_row[idx[0]];
                let p1 = src_row[idx[1]];
                let p2 = src_row[idx[2]];
                let p3 = src_row[idx[3]];

                let a = ((p0 >> 24) & 0xFF) as i32 * wx[0]
                    + ((p1 >> 24) & 0xFF) as i32 * wx[1]
                    + ((p2 >> 24) & 0xFF) as i32 * wx[2]
                    + ((p3 >> 24) & 0xFF) as i32 * wx[3];
                let r = ((p0 >> 16) & 0xFF) as i32 * wx[0]
                    + ((p1 >> 16) & 0xFF) as i32 * wx[1]
                    + ((p2 >> 16) & 0xFF) as i32 * wx[2]
                    + ((p3 >> 16) & 0xFF) as i32 * wx[3];
                let g = ((p0 >> 8) & 0xFF) as i32 * wx[0]
                    + ((p1 >> 8) & 0xFF) as i32 * wx[1]
                    + ((p2 >> 8) & 0xFF) as i32 * wx[2]
                    + ((p3 >> 8) & 0xFF) as i32 * wx[3];
                let b = (p0 & 0xFF) as i32 * wx[0]
                    + (p1 & 0xFF) as i32 * wx[1]
                    + (p2 & 0xFF) as i32 * wx[2]
                    + (p3 & 0xFF) as i32 * wx[3];

                *pixel = (clamp_i32_u8(a >> FRAC_BITS) << 24)
                    | (clamp_i32_u8(r >> FRAC_BITS) << 16)
                    | (clamp_i32_u8(g >> FRAC_BITS) << 8)
                    | clamp_i32_u8(b >> FRAC_BITS);
            }
        }

        // Pass 2: vertical  (dst_w × src_h → dst_w × dst_h)
        let mid = &self.intermediate;
        for dy in 0..self.dst_h as usize {
            let ym = &self.bc_y[dy];
            let idx = &ym.idx;
            let wy = &ym.w;

            let row0 = &mid[idx[0] * dw..idx[0] * dw + dw];
            let row1 = &mid[idx[1] * dw..idx[1] * dw + dw];
            let row2 = &mid[idx[2] * dw..idx[2] * dw + dw];
            let row3 = &mid[idx[3] * dw..idx[3] * dw + dw];
            let dst_row = &mut self.output[dy * dw..dy * dw + dw];

            for (dx, pixel) in dst_row.iter_mut().enumerate() {
                let p0 = row0[dx];
                let p1 = row1[dx];
                let p2 = row2[dx];
                let p3 = row3[dx];

                let a = ((p0 >> 24) & 0xFF) as i32 * wy[0]
                    + ((p1 >> 24) & 0xFF) as i32 * wy[1]
                    + ((p2 >> 24) & 0xFF) as i32 * wy[2]
                    + ((p3 >> 24) & 0xFF) as i32 * wy[3];
                let r = ((p0 >> 16) & 0xFF) as i32 * wy[0]
                    + ((p1 >> 16) & 0xFF) as i32 * wy[1]
                    + ((p2 >> 16) & 0xFF) as i32 * wy[2]
                    + ((p3 >> 16) & 0xFF) as i32 * wy[3];
                let g = ((p0 >> 8) & 0xFF) as i32 * wy[0]
                    + ((p1 >> 8) & 0xFF) as i32 * wy[1]
                    + ((p2 >> 8) & 0xFF) as i32 * wy[2]
                    + ((p3 >> 8) & 0xFF) as i32 * wy[3];
                let b = (p0 & 0xFF) as i32 * wy[0]
                    + (p1 & 0xFF) as i32 * wy[1]
                    + (p2 & 0xFF) as i32 * wy[2]
                    + (p3 & 0xFF) as i32 * wy[3];

                *pixel = (clamp_i32_u8(a >> FRAC_BITS) << 24)
                    | (clamp_i32_u8(r >> FRAC_BITS) << 16)
                    | (clamp_i32_u8(g >> FRAC_BITS) << 8)
                    | clamp_i32_u8(b >> FRAC_BITS);
            }
        }
    }
}

// ── Map builders ─────────────────────────────────────────────────────────────

fn build_bi_axis_map(src_size: u32, dst_size: u32) -> Vec<BiAxisMap> {
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

fn build_bc_axis_map(src_size: u32, dst_size: u32, clamp_size: u32) -> Vec<BcAxisMap> {
    let max_idx = clamp_size as usize - 1;
    let mut map = Vec::with_capacity(dst_size as usize);
    for d in 0..dst_size {
        let src_f = (d as f64 + 0.5) * src_size as f64 / dst_size as f64 - 0.5;
        let center = src_f.floor() as i32;
        let t = (src_f - center as f64) as f32;
        let w = catmull_rom_weights_fixed(t);
        // Pre-compute clamped source indices for the 4 taps.
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

fn catmull_rom_weights_fixed(t: f32) -> [i32; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        ((-0.5 * t3 + t2 - 0.5 * t) * FRAC_UNIT as f32) as i32,
        ((1.5 * t3 - 2.5 * t2 + 1.0) * FRAC_UNIT as f32) as i32,
        ((-1.5 * t3 + 2.0 * t2 + 0.5 * t) * FRAC_UNIT as f32) as i32,
        ((0.5 * t3 - 0.5 * t2) * FRAC_UNIT as f32) as i32,
    ]
}

#[inline]
fn clamp_i32_u8(v: i32) -> u32 {
    v.clamp(0, 255) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaler_1x1_is_identity() {
        let src = vec![0xFFABCD12_u32];
        let mut scaler = Scaler::new();
        let out = scaler.scale(&src, 1, 1, 1, 1);
        assert_eq!(out, &[0xFFABCD12]);
    }

    #[test]
    fn scaler_2x_to_4x4() {
        let src = vec![0xFFFF0000, 0xFF00FF00, 0xFF0000FF, 0xFFFFFF00];
        let mut scaler = Scaler::new();
        let out = scaler.scale(&src, 2, 2, 4, 4);
        assert_eq!(out.len(), 16);
        assert_eq!(out[0] & 0xFF000000, 0xFF000000);
    }

    #[test]
    fn bicubic_1x1_is_identity() {
        let src = vec![0xFFABCD12_u32];
        let mut scaler = Scaler::new();
        scaler.set_filter(ScaleFilter::Bicubic);
        let out = scaler.scale(&src, 1, 1, 1, 1);
        assert_eq!(out, &[0xFFABCD12]);
    }

    #[test]
    fn bicubic_2x_to_4x4() {
        let src = vec![0xFFFF0000, 0xFF00FF00, 0xFF0000FF, 0xFFFFFF00];
        let mut scaler = Scaler::new();
        scaler.set_filter(ScaleFilter::Bicubic);
        let out = scaler.scale(&src, 2, 2, 4, 4);
        assert_eq!(out.len(), 16);
        assert!((out[0] >> 24) >= 0xFC, "alpha too low: {:08x}", out[0]);
    }

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
