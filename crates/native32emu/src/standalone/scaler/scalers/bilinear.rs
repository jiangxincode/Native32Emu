// Bilinear scaler: 2x2 neighbourhood with fixed-point weights.

use super::{build_bi_axis_map, BiAxisMap};

pub struct BilinearScaler {
    x_map: Vec<BiAxisMap>,
    y_map: Vec<BiAxisMap>,
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
}

impl BilinearScaler {
    pub fn new() -> Self {
        Self {
            x_map: Vec::new(),
            y_map: Vec::new(),
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
            self.x_map = build_bi_axis_map(src_w, dst_w);
            self.y_map = build_bi_axis_map(src_h, dst_h);
            self.src_w = src_w;
            self.src_h = src_h;
            self.dst_w = dst_w;
            self.dst_h = dst_h;
        }

        let sw = src_w as usize;
        let sh = src_h as usize;
        let dw = dst_w as usize;

        for (dy, ym) in self.y_map.iter().enumerate() {
            let sy0 = ym.src as usize;
            let sy1 = (sy0 + 1).min(sh - 1);
            let fy = ym.frac as u32;
            let fy_inv = 256 - fy;

            let row0 = &src[sy0 * sw..sy0 * sw + sw];
            let row1 = &src[sy1 * sw..sy1 * sw + sw];
            let dst_row = &mut dst[dy * dw..dy * dw + dw];

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
}
