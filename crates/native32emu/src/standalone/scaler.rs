// Software scaler: bilinear and bicubic interpolation on ARGB u32 buffers.
//
// Pre-computes per-pixel coordinate/weight mappings once when the output size
// changes, then each frame is just a lookup + integer multiply loop with no
// allocations and no floating-point in the hot path.

/// Scaling filter algorithm.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScaleFilter {
    /// Bilinear (2x2 neighborhood, fixed-point weights).
    Bilinear,
    /// Bicubic Catmull-Rom (4x4 neighborhood, sharper edges).
    Bicubic,
}

// ── Axis maps ────────────────────────────────────────────────────────────────

/// Bilinear per-axis entry: source index + fractional weight (0..256).
struct BiAxisMap {
    src: u32,
    frac: u16,
}

/// Bicubic per-axis entry: center source index + 4 pre-computed i32 weights
/// (fixed-point with FRAC_BITS fractional bits).
struct BicubicAxisMap {
    /// Center source pixel (the t=0 reference); actual taps are c-1, c, c+1, c+2.
    center: i32,
    /// Catmull-Rom weights for taps [-1, 0, +1, +2], scaled to i32 fixed-point.
    w: [i32; 4],
}

/// Fixed-point fractional bits for bicubic weights.
/// 16 bits gives ±32k range per channel × 4 taps = ±128k, fits i32 with margin.
const FRAC_BITS: i32 = 10;
const FRAC_UNIT: i32 = 1 << FRAC_BITS;

// ── Scaler ───────────────────────────────────────────────────────────────────

/// Scaler state that persists across frames to avoid re-allocations.
pub struct Scaler {
    filter: ScaleFilter,
    // Bilinear maps.
    bi_x: Vec<BiAxisMap>,
    bi_y: Vec<BiAxisMap>,
    // Bicubic maps.
    bc_x: Vec<BicubicAxisMap>,
    bc_y: Vec<BicubicAxisMap>,
    // Current dimensions (used to detect when maps need rebuilding).
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
            src_w: 0,
            src_h: 0,
            dst_w: 0,
            dst_h: 0,
            output: Vec::new(),
        }
    }

    /// Change the scaling filter.
    pub fn set_filter(&mut self, filter: ScaleFilter) {
        self.filter = filter;
    }

    /// Scale `src` (ARGB u32, row-major) to the target dimensions.
    /// Returns a reference to an internal buffer (reused across frames).
    pub fn scale(&mut self, src: &[u32], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> &[u32] {
        if src_w != self.src_w || src_h != self.src_h || dst_w != self.dst_w || dst_h != self.dst_h
        {
            self.bi_x = build_bi_axis_map(src_w, dst_w);
            self.bi_y = build_bi_axis_map(src_h, dst_h);
            self.bc_x = build_bc_axis_map(src_w, dst_w);
            self.bc_y = build_bc_axis_map(src_h, dst_h);
            self.src_w = src_w;
            self.src_h = src_h;
            self.dst_w = dst_w;
            self.dst_h = dst_h;
            self.output = vec![0u32; (dst_w * dst_h) as usize];
        }

        match self.filter {
            ScaleFilter::Bilinear => self.scale_bilinear(src, src_w, src_h),
            ScaleFilter::Bicubic => self.scale_bicubic(src, src_w, src_h),
        }

        &self.output
    }

    // ── Bilinear ─────────────────────────────────────────────────────────

    /// Bilinear scaling: 2x2 neighborhood with fixed-point weights.
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

    // ── Bicubic ──────────────────────────────────────────────────────────

    /// Bicubic scaling: 4x4 Catmull-Rom with pre-computed fixed-point weights.
    fn scale_bicubic(&mut self, src: &[u32], src_w: u32, src_h: u32) {
        let sw = src_w as usize;
        let sh = src_h as usize;
        let dw = self.dst_w as usize;
        let sh_i = sh as i32;
        let sw_i = sw as i32;

        for dy in 0..self.dst_h as usize {
            let ym = &self.bc_y[dy];
            let cy = ym.center;
            let wy = &ym.w;

            // Pre-fetch the 4 source rows (clamped).
            let sy = [
                (cy - 1).clamp(0, sh_i - 1) as usize * sw,
                cy.clamp(0, sh_i - 1) as usize * sw,
                (cy + 1).clamp(0, sh_i - 1) as usize * sw,
                (cy + 2).clamp(0, sh_i - 1) as usize * sw,
            ];
            let rows = [
                &src[sy[0]..sy[0] + sw],
                &src[sy[1]..sy[1] + sw],
                &src[sy[2]..sy[2] + sw],
                &src[sy[3]..sy[3] + sw],
            ];

            let dst_row = &mut self.output[dy * dw..dy * dw + dw];

            for (dx, pixel) in dst_row.iter_mut().enumerate() {
                let xm = &self.bc_x[dx];
                let cx = xm.center;
                let wx = &xm.w;

                // Source column indices (clamped).
                let sx = [
                    (cx - 1).clamp(0, sw_i - 1) as usize,
                    cx.clamp(0, sw_i - 1) as usize,
                    (cx + 1).clamp(0, sw_i - 1) as usize,
                    (cx + 2).clamp(0, sw_i - 1) as usize,
                ];

                // 4×4 weighted sum using fixed-point i32.
                let mut ra: i32 = 0;
                let mut rr: i32 = 0;
                let mut rg: i32 = 0;
                let mut rb: i32 = 0;

                for j in 0..4 {
                    let wy_j = wy[j];
                    let row = rows[j];
                    let p0 = row[sx[0]];
                    let p1 = row[sx[1]];
                    let p2 = row[sx[2]];
                    let p3 = row[sx[3]];

                    let w0 = (wx[0] * wy_j) >> FRAC_BITS;
                    let w1 = (wx[1] * wy_j) >> FRAC_BITS;
                    let w2 = (wx[2] * wy_j) >> FRAC_BITS;
                    let w3 = (wx[3] * wy_j) >> FRAC_BITS;

                    ra += ((p0 >> 24) & 0xFF) as i32 * w0
                        + ((p1 >> 24) & 0xFF) as i32 * w1
                        + ((p2 >> 24) & 0xFF) as i32 * w2
                        + ((p3 >> 24) & 0xFF) as i32 * w3;
                    rr += ((p0 >> 16) & 0xFF) as i32 * w0
                        + ((p1 >> 16) & 0xFF) as i32 * w1
                        + ((p2 >> 16) & 0xFF) as i32 * w2
                        + ((p3 >> 16) & 0xFF) as i32 * w3;
                    rg += ((p0 >> 8) & 0xFF) as i32 * w0
                        + ((p1 >> 8) & 0xFF) as i32 * w1
                        + ((p2 >> 8) & 0xFF) as i32 * w2
                        + ((p3 >> 8) & 0xFF) as i32 * w3;
                    rb += (p0 & 0xFF) as i32 * w0
                        + (p1 & 0xFF) as i32 * w1
                        + (p2 & 0xFF) as i32 * w2
                        + (p3 & 0xFF) as i32 * w3;
                }

                // Shift back from fixed-point and clamp to [0, 255].
                *pixel = (clamp_i32_u8(ra >> FRAC_BITS) << 24)
                    | (clamp_i32_u8(rr >> FRAC_BITS) << 16)
                    | (clamp_i32_u8(rg >> FRAC_BITS) << 8)
                    | clamp_i32_u8(rb >> FRAC_BITS);
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

fn build_bc_axis_map(src_size: u32, dst_size: u32) -> Vec<BicubicAxisMap> {
    let mut map = Vec::with_capacity(dst_size as usize);
    for d in 0..dst_size {
        let src_f = (d as f64 + 0.5) * src_size as f64 / dst_size as f64 - 0.5;
        let center = src_f.floor() as i32;
        let t = (src_f - center as f64) as f32;
        map.push(BicubicAxisMap {
            center,
            w: catmull_rom_weights_fixed(t),
        });
    }
    map
}

// ── Catmull-Rom kernel ───────────────────────────────────────────────────────

/// Catmull-Rom weights for fractional offset `t` in [0, 1), returned as
/// i32 fixed-point with `FRAC_BITS` fractional bits.
/// Weights correspond to source positions [-1, 0, +1, +2] relative to center.
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

/// Clamp an i32 to [0, 255] and return as u32.
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
        let src = vec![
            0xFFFF0000, 0xFF00FF00, //
            0xFF0000FF, 0xFFFFFF00, //
        ];
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
        let src = vec![
            0xFFFF0000, 0xFF00FF00, //
            0xFF0000FF, 0xFFFFFF00, //
        ];
        let mut scaler = Scaler::new();
        scaler.set_filter(ScaleFilter::Bicubic);
        let out = scaler.scale(&src, 2, 2, 4, 4);
        assert_eq!(out.len(), 16);
        // Alpha may have small rounding error from fixed-point bicubic.
        assert!((out[0] >> 24) >= 0xFC, "alpha too low: {:08x}", out[0]);
    }

    #[test]
    fn catmull_rom_fixed_at_zero() {
        let w = catmull_rom_weights_fixed(0.0);
        // At t=0: w(0)=1.0 → FRAC_UNIT, others ≈0
        assert!((w[1] - FRAC_UNIT).unsigned_abs() <= 1);
        assert!(w[0].unsigned_abs() <= 1);
        assert!(w[2].unsigned_abs() <= 1);
        assert!(w[3].unsigned_abs() <= 1);
    }

    #[test]
    fn catmull_rom_fixed_sum() {
        // Sum of weights should be ≈ FRAC_UNIT for any t.
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
