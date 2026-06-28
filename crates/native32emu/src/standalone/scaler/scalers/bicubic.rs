// Bicubic scaler: separable two-pass Catmull-Rom (horizontal → vertical).

use super::{build_bc_axis_map, clamp_i32_u8, BcAxisMap, FRAC_BITS};

pub struct BicubicScaler {
    x_map: Vec<BcAxisMap>,
    y_map: Vec<BcAxisMap>,
    intermediate: Vec<u32>,
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
}

impl BicubicScaler {
    pub fn new() -> Self {
        Self {
            x_map: Vec::new(),
            y_map: Vec::new(),
            intermediate: Vec::new(),
            src_w: 0,
            src_h: 0,
            dst_w: 0,
            dst_h: 0,
        }
    }

    pub fn scale(
        &mut self,
        src: &[u32],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
        dst: &mut [u32],
    ) {
        if src_w != self.src_w || src_h != self.src_h || dst_w != self.dst_w || dst_h != self.dst_h
        {
            self.x_map = build_bc_axis_map(src_w, dst_w, src_w);
            self.y_map = build_bc_axis_map(src_h, dst_h, src_h);
            self.intermediate = vec![0u32; (dst_w * src_h) as usize];
            self.src_w = src_w;
            self.src_h = src_h;
            self.dst_w = dst_w;
            self.dst_h = dst_h;
        }

        let sw = src_w as usize;
        let dw = dst_w as usize;

        // Pass 1: horizontal  (src_w × src_h → dst_w × src_h)
        for sy in 0..src_h as usize {
            let src_row = &src[sy * sw..sy * sw + sw];
            let dst_row = &mut self.intermediate[sy * dw..sy * dw + dw];

            for (dx, pixel) in dst_row.iter_mut().enumerate() {
                let xm = &self.x_map[dx];
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
        for (dy, ym) in self.y_map.iter().enumerate() {
            let idx = &ym.idx;
            let wy = &ym.w;

            let row0 = &mid[idx[0] * dw..idx[0] * dw + dw];
            let row1 = &mid[idx[1] * dw..idx[1] * dw + dw];
            let row2 = &mid[idx[2] * dw..idx[2] * dw + dw];
            let row3 = &mid[idx[3] * dw..idx[3] * dw + dw];
            let dst_row = &mut dst[dy * dw..dy * dw + dw];

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
