// Software scaler: bilinear interpolation on ARGB u32 buffers.
//
// Pre-computes a per-pixel coordinate/weight mapping once when the output size
// changes, then each frame is just a simple lerp loop with no allocations.

/// Per-axis mapping entry: index into the source line and the interpolation
/// weight (0..256) for the "next" pixel.
struct AxisMap {
    /// Source pixel index (integer part).
    src: u32,
    /// Blend weight for the next pixel (0 = exact pixel, 256 = fully next).
    /// Stored as u16 to keep the mapping table compact.
    frac: u16,
}

/// Thread-local scaler state that persists across frames.
pub struct Scaler {
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
            x_map: Vec::new(),
            y_map: Vec::new(),
            src_w: 0,
            src_h: 0,
            dst_w: 0,
            dst_h: 0,
            output: Vec::new(),
        }
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

        let sw = src_w as usize;
        let dw = dst_w as usize;

        for dy in 0..dst_h as usize {
            let ym = &self.y_map[dy];
            let sy0 = ym.src as usize;
            let sy1 = (sy0 + 1).min(src_h as usize - 1);
            let fy = ym.frac as u32;
            let fy_inv = 256 - fy;

            let row0 = &src[sy0 * sw..sy0 * sw + sw];
            let row1 = &src[sy1 * sw..sy1 * sw + sw];
            let dst_row = &mut self.output[dy * dw..dy * dw + dw];

            for dx in 0..dw {
                let xm = &self.x_map[dx];
                let sx0 = xm.src as usize;
                let sx1 = (sx0 + 1).min(sw - 1);
                let fx = xm.frac as u32;
                let fx_inv = 256 - fx;

                let p00 = row0[sx0];
                let p10 = row0[sx1];
                let p01 = row1[sx0];
                let p11 = row1[sx1];

                // Bilinear blend: a*(1-fx)*(1-fy) + b*fx*(1-fy) +
                //                 c*(1-fx)*fy     + d*fx*fy
                // Factor out per-channel to avoid per-pixel divides.
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

                dst_row[dx] = ((a >> 16) << 24) | ((r >> 16) << 16) | ((g >> 16) << 8) | (b >> 16);
            }
        }

        &self.output
    }
}

/// Build the per-axis mapping table: for each destination pixel, store the
/// source index and the fractional weight.
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
        // 2x2 source → 4x4 output, bilinear should produce a smooth gradient.
        let src = vec![
            0xFFFF0000, 0xFF00FF00, //
            0xFF0000FF, 0xFFFFFF00, //
        ];
        let mut scaler = Scaler::new();
        let out = scaler.scale(&src, 2, 2, 4, 4);
        assert_eq!(out.len(), 16);
        // Top-left should be close to pure red.
        assert_eq!(out[0] & 0xFF000000, 0xFF000000); // alpha preserved
    }

    #[test]
    fn build_axis_map_integer_scale() {
        let map = build_axis_map(2, 4);
        // 2→4 is a 2x scale. First half should map to source 0, second to 1.
        assert_eq!(map[0].src, 0);
        assert_eq!(map[1].src, 0);
        // Pixels 2,3 straddle the boundary; frac determines the blend.
        assert!(map[2].src <= 1);
        assert_eq!(map[3].src, 1);
    }
}
