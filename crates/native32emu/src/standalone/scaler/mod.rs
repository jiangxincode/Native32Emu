// Scaler dispatcher: routes scale() calls to the selected algorithm.
//
// Each algorithm lives in its own file under `scalers/` and exposes a struct
// with a `scale()` method.  Adding a new algorithm means:
//   1. Create `scalers/newalgo.rs`
//   2. Add a `ScaleFilter::Newalgo` variant
//   3. Add the dispatch arm here

pub mod scalers;

use scalers::bicubic::BicubicScaler;
use scalers::bilinear::BilinearScaler;
use scalers::xbrz;

/// Scaling filter algorithm.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScaleFilter {
    /// Bilinear (2x2 neighbourhood, fixed-point weights).
    Bilinear,
    /// Bicubic Catmull-Rom (separable 4-tap, sharper edges).
    Bicubic,
    /// xBRZ pixel-art scaler (integer 2x-4x, then bilinear for remainder).
    Xbrz,
}

/// Scaler state that persists across frames to avoid re-allocations.
pub struct Scaler {
    filter: ScaleFilter,
    bilinear: BilinearScaler,
    bicubic: BicubicScaler,
    output: Vec<u32>,
}

impl Scaler {
    pub fn new() -> Self {
        Self {
            filter: ScaleFilter::Bilinear,
            bilinear: BilinearScaler::new(),
            bicubic: BicubicScaler::new(),
            output: Vec::new(),
        }
    }

    pub fn set_filter(&mut self, filter: ScaleFilter) {
        self.filter = filter;
    }

    /// Scale `src` to `dst_w × dst_h`.  Returns a reference to an internal
    /// buffer that is reused across frames.
    pub fn scale(&mut self, src: &[u32], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> &[u32] {
        let len = (dst_w * dst_h) as usize;
        if self.output.len() != len {
            self.output = vec![0u32; len];
        }

        match self.filter {
            ScaleFilter::Bilinear => {
                self.bilinear
                    .scale(src, src_w, src_h, dst_w, dst_h, &mut self.output);
            }
            ScaleFilter::Bicubic => {
                self.bicubic
                    .scale(src, src_w, src_h, dst_w, dst_h, &mut self.output);
            }
            ScaleFilter::Xbrz => {
                xbrz::scale_with_bilinear(src, src_w, src_h, dst_w, dst_h, &mut self.output);
            }
        }

        &self.output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bilinear_1x1_is_identity() {
        let src = vec![0xFFABCD12_u32];
        let mut scaler = Scaler::new();
        let out = scaler.scale(&src, 1, 1, 1, 1);
        assert_eq!(out, &[0xFFABCD12]);
    }

    #[test]
    fn bilinear_2x_to_4x4() {
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
    fn xbrz_2x_uniform() {
        let src = vec![0xFFFF0000_u32; 4];
        let mut scaler = Scaler::new();
        scaler.set_filter(ScaleFilter::Xbrz);
        let out = scaler.scale(&src, 2, 2, 4, 4);
        for &p in out.iter().take(16) {
            assert_eq!(p, 0xFFFF0000);
        }
    }
}
