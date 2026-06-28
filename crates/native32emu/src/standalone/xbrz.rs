// xBRZ pixel-art scaler.
//
// A simplified implementation inspired by the xBRZ algorithm (by Zenju).
// Detects edges and diagonals in a 3×3 neighbourhood and selectively blends
// corner sub-pixels to smooth diagonal lines while preserving sharp
// horizontal/vertical edges.
//
// Supports integer scale factors 2..=4. Output dimensions are
// (src_w × factor, src_h × factor).

/// Color distance threshold for "similar" pixels (0..765 range, i.e. sum of
/// absolute R/G/B differences).  Tuned for 5-bit color (ARGB1555) where
/// channel differences are quantised to multiples of 8.
const DIST_THRESHOLD: u32 = 80;

/// Blend a sub-pixel toward `blend_to` by the given weight (0..255).
/// `weight = 255` produces `blend_to` fully; `weight = 0` keeps `base`.
#[inline]
fn blend_pixel(base: u32, blend_to: u32, weight: u32) -> u32 {
    let inv = 255 - weight;
    let a = ((base >> 24) & 0xFF) * inv + ((blend_to >> 24) & 0xFF) * weight;
    let r = ((base >> 16) & 0xFF) * inv + ((blend_to >> 16) & 0xFF) * weight;
    let g = ((base >> 8) & 0xFF) * inv + ((blend_to >> 8) & 0xFF) * weight;
    let b = (base & 0xFF) * inv + (blend_to & 0xFF) * weight;
    ((a / 255) << 24) | ((r / 255) << 16) | ((g / 255) << 8) | (b / 255)
}

/// Absolute colour distance (sum of |R| + |G| + |B|).
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

/// Detect diagonal blend for a corner of the output block.
///
/// For the top-left corner the diagonals are `d1 = NW`, `d2 = NE`,
/// `d3 = SW`, `adj_h = N`, `adj_v = W`.
///
/// Returns `true` when the corner should be blended toward the opposite
/// diagonal (`d3`).
fn should_blend_corner(c: u32, d1: u32, d2: u32, d3: u32, adj_h: u32, adj_v: u32) -> bool {
    let similar_d1 = color_distance(c, d1) <= DIST_THRESHOLD;
    let similar_d2 = color_distance(c, d2) <= DIST_THRESHOLD;
    let similar_d3 = color_distance(c, d3) <= DIST_THRESHOLD;
    let similar_h = color_distance(c, adj_h) <= DIST_THRESHOLD;
    let similar_v = color_distance(c, adj_v) <= DIST_THRESHOLD;

    // The opposite diagonal (d3) differs from c → edge exists there.
    if !similar_d3 {
        return false;
    }

    // d1 similar and adjacent differ → classic pixel-art diagonal.
    if similar_d1 && !similar_h && !similar_v {
        return true;
    }

    // Gradient along the diagonal: d1≠d2 but both ≈c.
    if similar_d1 && similar_d2 && color_distance(d1, d2) > DIST_THRESHOLD {
        return true;
    }

    false
}

/// Scale a source image using the xBRZ algorithm.
///
/// `src` is a row-major `u32` ARGB buffer.  `factor` must be 2, 3, or 4.
/// Returns a newly allocated `Vec<u32>` of size `(src_w × factor) × (src_h × factor)`.
pub fn scale_xbrz(src: &[u32], src_w: u32, src_h: u32, factor: u32) -> Vec<u32> {
    let sw = src_w as usize;
    let sh = src_h as usize;
    let f = factor as usize;
    let dw = sw * f;
    let dh = sh * f;
    let mut out = vec![0u32; dw * dh];

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

            // 3×3 neighbourhood (0 = transparent for out-of-bounds).
            let nw = get(x.wrapping_sub(1), y.wrapping_sub(1));
            let n = get(x, y.wrapping_sub(1));
            let ne = get(x + 1, y.wrapping_sub(1));
            let w = get(x.wrapping_sub(1), y);
            let e = get(x + 1, y);
            let sw_p = get(x.wrapping_sub(1), y + 1);
            let s = get(x, y + 1);
            let se = get(x + 1, y + 1);

            // ── Blend decisions for the 4 output sub-blocks ──────────────
            //
            // Top-left corner: diagonals NW/SE, adjacents N/W.
            let blend_tl = should_blend_corner(c, nw, ne, se, n, w);
            // Top-right: diagonals NE/SW, adjacents N/E.
            let blend_tr = should_blend_corner(c, ne, nw, sw_p, n, e);
            // Bottom-left: diagonals SW/NE, adjacents S/W.
            let blend_bl = should_blend_corner(c, sw_p, se, ne, s, w);
            // Bottom-right: diagonals SE/NW, adjacents S/E.
            let blend_br = should_blend_corner(c, se, sw_p, nw, s, e);

            // ── Write the factor×factor output block ─────────────────────
            let base_x = x * f;
            let base_y = y * f;

            if f == 2 {
                out[base_y * dw + base_x] = if blend_tl { blend_pixel(c, se, 128) } else { c };
                out[base_y * dw + base_x + 1] = if blend_tr {
                    blend_pixel(c, sw_p, 128)
                } else {
                    c
                };
                out[(base_y + 1) * dw + base_x] =
                    if blend_bl { blend_pixel(c, ne, 128) } else { c };
                out[(base_y + 1) * dw + base_x + 1] =
                    if blend_br { blend_pixel(c, nw, 128) } else { c };
            } else {
                // For 3x/4x: fill the entire block with c, then blend only
                // the corner sub-pixels (positions 0 and f-1 on each edge).
                for dy in 0..f {
                    for dx in 0..f {
                        out[(base_y + dy) * dw + base_x + dx] = c;
                    }
                }
                // Blend the 4 corner pixels.
                if blend_tl {
                    out[base_y * dw + base_x] = blend_pixel(c, se, 128);
                }
                if blend_tr {
                    out[base_y * dw + base_x + f - 1] = blend_pixel(c, sw_p, 128);
                }
                if blend_bl {
                    out[(base_y + f - 1) * dw + base_x] = blend_pixel(c, ne, 128);
                }
                if blend_br {
                    out[(base_y + f - 1) * dw + base_x + f - 1] = blend_pixel(c, nw, 128);
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xbrz_2x_uniform() {
        let src = vec![0xFFFF0000_u32; 4]; // 2x2 all red
        let out = scale_xbrz(&src, 2, 2, 2);
        assert_eq!(out.len(), 16);
        // All output pixels should be red (no edges to blend).
        for &p in &out {
            assert_eq!(p, 0xFFFF0000);
        }
    }

    #[test]
    fn xbrz_2x_diagonal() {
        // 3x3 with a diagonal edge: top-left quadrant red, bottom-right blue.
        let r = 0xFFFF0000_u32;
        let b = 0xFF0000FF_u32;
        let src = vec![r, r, b, r, b, b, b, b, b];
        let out = scale_xbrz(&src, 3, 3, 2);
        assert_eq!(out.len(), 36);
        // The centre pixel (1,1) should be blended at its corners.
        // Just verify no panic and output is reasonable.
        assert!(out.iter().all(|&p| (p >> 24) == 0xFF));
    }

    #[test]
    fn color_distance_same() {
        assert_eq!(color_distance(0xFFABCDEF, 0xFFABCDEF), 0);
    }

    #[test]
    fn color_distance_different() {
        let d = color_distance(0xFFFF0000, 0xFF0000FF);
        assert_eq!(d, 255 + 255); // R diff + B diff
    }
}
