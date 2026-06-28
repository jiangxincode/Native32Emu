// xBRZ pixel-art scaler.
//
// Detects edges and diagonals in a 3×3 neighbourhood and selectively blends
// corner sub-pixels to smooth diagonal lines while preserving sharp
// horizontal/vertical edges.
// Supports integer scale factors 2..=4.

use super::build_bi_axis_map;

const DIST_THRESHOLD: u32 = 80;

#[inline]
fn blend_pixel(base: u32, blend_to: u32, weight: u32) -> u32 {
    let inv = 255 - weight;
    let a = ((base >> 24) & 0xFF) * inv + ((blend_to >> 24) & 0xFF) * weight;
    let r = ((base >> 16) & 0xFF) * inv + ((blend_to >> 16) & 0xFF) * weight;
    let g = ((base >> 8) & 0xFF) * inv + ((blend_to >> 8) & 0xFF) * weight;
    let b = (base & 0xFF) * inv + (blend_to & 0xFF) * weight;
    ((a / 255) << 24) | ((r / 255) << 16) | ((g / 255) << 8) | (b / 255)
}

#[inline]
fn color_distance(c1: u32, c2: u32) -> u32 {
    let r1 = ((c1 >> 16) & 0xFF) as i32;
    let g1 = ((c1 >> 8) & 0xFF) as i32;
    let b1 = (c1 & 0xFF) as i32;
    let r2 = ((c2 >> 16) & 0xFF) as i32;
    let g2 = ((c2 >> 8) & 0xFF) as i32;
    let b2 = (c2 & 0xFF) as i32;
    (r1 - r2).unsigned_abs() + (g1 - g2).unsigned_abs() + (b1 - b2).unsigned_abs()
}

fn should_blend_corner(c: u32, d1: u32, d2: u32, d3: u32, adj_h: u32, adj_v: u32) -> bool {
    let similar_d1 = color_distance(c, d1) <= DIST_THRESHOLD;
    let similar_d2 = color_distance(c, d2) <= DIST_THRESHOLD;
    let similar_d3 = color_distance(c, d3) <= DIST_THRESHOLD;
    let similar_h = color_distance(c, adj_h) <= DIST_THRESHOLD;
    let similar_v = color_distance(c, adj_v) <= DIST_THRESHOLD;

    if !similar_d3 {
        return false;
    }
    if similar_d1 && !similar_h && !similar_v {
        return true;
    }
    if similar_d1 && similar_d2 && color_distance(d1, d2) > DIST_THRESHOLD {
        return true;
    }
    false
}

/// xBRZ integer-factor scaling.  Writes into `dst` which must be
/// `(src_w * factor) × (src_h * factor)` elements.
pub fn scale(src: &[u32], src_w: u32, src_h: u32, factor: u32, dst: &mut [u32]) {
    let sw = src_w as usize;
    let sh = src_h as usize;
    let f = factor as usize;
    let dw = sw * f;

    let get = |x: usize, y: usize| -> u32 {
        if x < sw && y < sh {
            src[y * sw + x]
        } else {
            0x00000000
        }
    };

    for y in 0..sh {
        for x in 0..sw {
            let c = get(x, y);

            let nw = get(x.wrapping_sub(1), y.wrapping_sub(1));
            let n = get(x, y.wrapping_sub(1));
            let ne = get(x + 1, y.wrapping_sub(1));
            let w = get(x.wrapping_sub(1), y);
            let e = get(x + 1, y);
            let sw_p = get(x.wrapping_sub(1), y + 1);
            let s = get(x, y + 1);
            let se = get(x + 1, y + 1);

            let blend_tl = should_blend_corner(c, nw, ne, se, n, w);
            let blend_tr = should_blend_corner(c, ne, nw, sw_p, n, e);
            let blend_bl = should_blend_corner(c, sw_p, se, ne, s, w);
            let blend_br = should_blend_corner(c, se, sw_p, nw, s, e);

            let base_x = x * f;
            let base_y = y * f;

            if f == 2 {
                dst[base_y * dw + base_x] = if blend_tl { blend_pixel(c, se, 128) } else { c };
                dst[base_y * dw + base_x + 1] = if blend_tr {
                    blend_pixel(c, sw_p, 128)
                } else {
                    c
                };
                dst[(base_y + 1) * dw + base_x] =
                    if blend_bl { blend_pixel(c, ne, 128) } else { c };
                dst[(base_y + 1) * dw + base_x + 1] =
                    if blend_br { blend_pixel(c, nw, 128) } else { c };
            } else {
                for dy in 0..f {
                    for dx in 0..f {
                        dst[(base_y + dy) * dw + base_x + dx] = c;
                    }
                }
                if blend_tl {
                    dst[base_y * dw + base_x] = blend_pixel(c, se, 128);
                }
                if blend_tr {
                    dst[base_y * dw + base_x + f - 1] = blend_pixel(c, sw_p, 128);
                }
                if blend_bl {
                    dst[(base_y + f - 1) * dw + base_x] = blend_pixel(c, ne, 128);
                }
                if blend_br {
                    dst[(base_y + f - 1) * dw + base_x + f - 1] = blend_pixel(c, nw, 128);
                }
            }
        }
    }
}

/// Combined xBRZ + bilinear scaling for arbitrary output sizes.
pub fn scale_with_bilinear(
    src: &[u32],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    dst: &mut [u32],
) {
    let factor_x = dst_w / src_w;
    let factor_y = dst_h / src_h;
    let factor = factor_x.max(factor_y).clamp(2, 4);

    let xbrz_w = src_w * factor;
    let xbrz_h = src_h * factor;

    if xbrz_w == dst_w && xbrz_h == dst_h {
        scale(src, src_w, src_h, factor, dst);
        return;
    }

    // xBRZ at integer factor, then bilinear for the remainder.
    let mut xbrz_buf = vec![0u32; (xbrz_w * xbrz_h) as usize];
    scale(src, src_w, src_h, factor, &mut xbrz_buf);

    let bi_x = build_bi_axis_map(xbrz_w, dst_w);
    let bi_y = build_bi_axis_map(xbrz_h, dst_h);
    let sw = xbrz_w as usize;
    let sh = xbrz_h as usize;
    let dw = dst_w as usize;

    for (dy, ym) in bi_y.iter().enumerate() {
        let sy0 = ym.src as usize;
        let sy1 = (sy0 + 1).min(sh - 1);
        let fy = ym.frac as u32;
        let fy_inv = 256 - fy;

        let row0 = &xbrz_buf[sy0 * sw..sy0 * sw + sw];
        let row1 = &xbrz_buf[sy1 * sw..sy1 * sw + sw];
        let dst_row = &mut dst[dy * dw..dy * dw + dw];

        for (dx, pixel) in dst_row.iter_mut().enumerate() {
            let xm = &bi_x[dx];
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
            let b =
                (p00 & 0xFF) * w00 + (p10 & 0xFF) * w10 + (p01 & 0xFF) * w01 + (p11 & 0xFF) * w11;

            *pixel = ((a >> 16) << 24) | ((r >> 16) << 16) | ((g >> 16) << 8) | (b >> 16);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xbrz_2x_uniform() {
        let src = vec![0xFFFF0000_u32; 4];
        let mut out = vec![0u32; 16];
        scale(&src, 2, 2, 2, &mut out);
        for &p in &out {
            assert_eq!(p, 0xFFFF0000);
        }
    }

    #[test]
    fn xbrz_2x_diagonal() {
        let r = 0xFFFF0000_u32;
        let b = 0xFF0000FF_u32;
        let src = vec![r, r, b, r, b, b, b, b, b];
        let mut out = vec![0u32; 36];
        scale(&src, 3, 3, 2, &mut out);
        assert!(out.iter().all(|&p| (p >> 24) == 0xFF));
    }

    #[test]
    fn color_distance_same() {
        assert_eq!(color_distance(0xFFABCDEF, 0xFFABCDEF), 0);
    }

    #[test]
    fn color_distance_different() {
        let d = color_distance(0xFFFF0000, 0xFF0000FF);
        assert_eq!(d, 255 + 255);
    }
}
