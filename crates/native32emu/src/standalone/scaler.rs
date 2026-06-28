// Software scaler: bilinear and bicubic interpolation on ARGB u32 buffers.
//
// Pre-computes per-pixel coordinate mappings once when the output size changes,
// then each frame is a simple interpolation loop with no allocations.

/// Scaling filter algorithm.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScaleFilter {
    /// Bilinear (2x2 neighborhood, integer weights).
    Bilinear,
    /// Bicubic Catmull-Rom (4x4 neighborhood, sharper edges).
    Bicubic,
}

/// Per-axis mapping entry: index into the source line and the fractional offset.
struct AxisMap {
    /// Source pixel index (integer part).
    src: u32,
    /// Fractional offset within [0, 256) for interpolation weight.
    frac: u16,
}

/// Scaler state that persists across frames to avoid re-allocations.
pub struct Scaler {
    filter: ScaleFilter,
    x_map: Vec<AxisMap>,
    y_map: Vec<AxisMap>,
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
            x_map: Vec::new(),
            y_map: Vec::new(),
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
        // Rebuild mapping tables when dimensions change.
        if src_w != self.src_w || src_h != self.src_h || dst_w != self.dst_w || dst_h != self.dst_h
        {
            self.x_map = build_axis_map(src_w, dst_w);
            self.y_map = build_axis_map(src_h, dst_h);
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

    /// Bilinear scaling: 2x2 neighborhood with integer fixed-point weights.
    fn scale_bilinear(&mut self, src: &[u32], src_w: u32, src_h: u32) {
        let sw = src_w as usize;
        let sh = src_h as usize;
        let dw = self.dst_w as usize;

        for dy in 0..self.dst_h as usize {
            let ym = &self.y_map[dy];
            let sy0 = ym.src as usize;
            let sy1 = (sy0 + 1).min(sh - 1);
            let fy = ym.frac as u32;
            let fy_inv = 256 - fy;

            let row0 = &src[sy0 * sw..sy0 * sw + sw];
            let row1 = &src[sy1 * sw..sy1 * sw + sw];
            let dst_row = &mut self.output[dy * dw..dy * dw + dw];

            for (dx, pixel) in dst_row.iter_mut().enumerate() {
                let xm = &self.x_map[dx];
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

    /// Bicubic scaling: 4x4 Catmull-Rom neighborhood for sharper edges.
    fn scale_bicubic(&mut self, src: &[u32], src_w: u32, src_h: u32) {
        let sw = src_w as usize;
        let sh = src_h as usize;
        let dw = self.dst_w as usize;

        for dy in 0..self.dst_h as usize {
            let ym = &self.y_map[dy];
            let ty = ym.frac as f32 / 256.0;
            let cy = ym.src as i32;
            let dst_row = &mut self.output[dy * dw..dy * dw + dw];

            // Pre-compute vertical kernel weights.
            let wy = catmull_rom_weights(ty);

            for (dx, pixel) in dst_row.iter_mut().enumerate() {
                let xm = &self.x_map[dx];
                let tx = xm.frac as f32 / 256.0;
                let cx = xm.src as i32;

                // Horizontal kernel weights.
                let wx = catmull_rom_weights(tx);

                // Accumulate over the 4x4 neighborhood.
                let (mut ra, mut rr, mut rg, mut rb) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);

                for j in 0..4i32 {
                    let sy = (cy + j - 1).clamp(0, sh as i32 - 1) as usize;
                    let wy_j = wy[j as usize];
                    let row = &src[sy * sw..sy * sw + sw];

                    for i in 0..4i32 {
                        let sx = (cx + i - 1).clamp(0, sw as i32 - 1) as usize;
                        let w = wx[i as usize] * wy_j;
                        let p = row[sx];

                        ra += ((p >> 24) & 0xFF) as f32 * w;
                        rr += ((p >> 16) & 0xFF) as f32 * w;
                        rg += ((p >> 8) & 0xFF) as f32 * w;
                        rb += (p & 0xFF) as f32 * w;
                    }
                }

                *pixel = (clamp_u8(ra) << 24)
                    | (clamp_u8(rr) << 16)
                    | (clamp_u8(rg) << 8)
                    | clamp_u8(rb);
            }
        }
    }
}

/// Catmull-Rom basis weights for fractional offset `t` in [0, 1).
/// Returns weights for source positions [-1, 0, +1, +2] relative to the
/// integer source index.
fn catmull_rom_weights(t: f32) -> [f32; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        -0.5 * t3 + t2 - 0.5 * t,       // w(-1)
        1.5 * t3 - 2.5 * t2 + 1.0,      // w( 0)
        -1.5 * t3 + 2.0 * t2 + 0.5 * t, // w(+1)
        0.5 * t3 - 0.5 * t2,            // w(+2)
    ]
}

/// Clamp a float to [0, 255] and return as u32.
fn clamp_u8(v: f32) -> u32 {
    v.clamp(0.0, 255.0) as u32
}

/// Build the per-axis mapping table: for each destination pixel, store the
/// source index and the fractional offset.
fn build_axis_map(src_size: u32, dst_size: u32) -> Vec<AxisMap> {
    let mut map = Vec::with_capacity(dst_size as usize);
    for d in 0..dst_size {
        // Map destination pixel centre to source coordinate.
        let src_f = (d as f64 + 0.5) * src_size as f64 / dst_size as f64 - 0.5;
        let src_i = src_f.floor().max(0.0) as u32;
        let frac = ((src_f - src_i as f64) * 256.0).round().min(255.0) as u16;
        map.push(AxisMap {
            src: src_i.min(src_size - 1),
            frac,
        });
    }
    map
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
        // Alpha should be preserved.
        assert_eq!(out[0] & 0xFF000000, 0xFF000000);
    }

    #[test]
    fn catmull_rom_weights_at_zero() {
        let w = catmull_rom_weights(0.0);
        // At t=0: w(0)=1, others=0
        assert!((w[1] - 1.0).abs() < 0.001);
        assert!(w[0].abs() < 0.001);
        assert!(w[2].abs() < 0.001);
        assert!(w[3].abs() < 0.001);
    }

    #[test]
    fn catmull_rom_weights_sum_to_one() {
        for t_int in 0..100 {
            let t = t_int as f32 / 100.0;
            let w = catmull_rom_weights(t);
            let sum = w[0] + w[1] + w[2] + w[3];
            assert!((sum - 1.0).abs() < 0.01, "sum={sum} at t={t}");
        }
    }

    #[test]
    fn build_axis_map_integer_scale() {
        let map = build_axis_map(2, 4);
        assert_eq!(map[0].src, 0);
        assert_eq!(map[1].src, 0);
        assert!(map[2].src <= 1);
        assert_eq!(map[3].src, 1);
    }
}
