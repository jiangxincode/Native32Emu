// Loader for `.dat` game-metadata files used by the FHUI front-end menu.
//
// Each game has a companion `<name>.dat` file with an "INFO" header that embeds
// two YUV image blocks: a small name/title graphic and a larger preview
// thumbnail. The menu's LoadImage host call uses the preview thumbnail as the
// game-list image.
//
// Observed layout (little-endian):
//   0x00  "INFO" magic
//   0x0C  version (= 2)
//   0x38  u32  offset of image block #1 (small title graphic) -> always 0x60
//   0x3C  u32  byte size of image block #1
//   0x58  u32  offset of image block #2 (preview thumbnail)
//   0x5C  u32  byte size of image block #2
//
// An image block is a standard Native32 image: [w u16][h u16][size u32][rle...].

use crate::file_loader::Colorspace;
use crate::image_decoder::{decode_image_argb, decode_image_yuv, RgbaImage};

const INFO_MAGIC: &[u8; 4] = b"INFO";
/// Offset of the u32 pointer to the small name/title banner image block.
const NAME_OFFSET_PTR: usize = 0x38;
/// Offset of the u32 pointer to the large preview screenshot image block.
const PREVIEW_OFFSET_PTR: usize = 0x58;

/// Which embedded image to decode from a `.dat` file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatImage {
    /// The small game-name/title banner (LoadImage flag "D", grid items).
    Name,
    /// The large preview screenshot (LoadImage flag "J", info pane).
    Preview,
}

impl DatImage {
    /// Map a LoadImage flag character to the image it selects.
    pub fn from_flag(flag: &str) -> Self {
        match flag {
            "J" => DatImage::Preview,
            // "D" (and any unknown flag) selects the name banner.
            _ => DatImage::Name,
        }
    }

    fn offset_ptr(self) -> usize {
        match self {
            DatImage::Name => NAME_OFFSET_PTR,
            DatImage::Preview => PREVIEW_OFFSET_PTR,
        }
    }
}

fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Decode one of the images embedded in an `INFO` `.dat` file.
///
/// Returns `None` if the file is not a recognized `.dat` or the image cannot be
/// decoded. `colorspace` selects the pixel decoder (Native32 menus are YUV).
pub fn decode_image(data: &[u8], colorspace: Colorspace, which: DatImage) -> Option<RgbaImage> {
    let ptr = which.offset_ptr();
    if data.len() < ptr + 4 || &data[0..4] != INFO_MAGIC {
        return None;
    }

    let offset = read_u32_le(data, ptr)? as usize;
    let block = data.get(offset..)?;
    if block.len() < 8 {
        return None;
    }

    match colorspace {
        Colorspace::ARGB => decode_image_argb(block),
        Colorspace::YUV => decode_image_yuv(block),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_info_files() {
        let data = vec![0u8; 256];
        assert!(decode_image(&data, Colorspace::YUV, DatImage::Preview).is_none());
    }

    #[test]
    fn rejects_truncated_files() {
        let data = b"INFO".to_vec();
        assert!(decode_image(&data, Colorspace::YUV, DatImage::Name).is_none());
    }

    #[test]
    fn flag_selects_image() {
        assert_eq!(DatImage::from_flag("D"), DatImage::Name);
        assert_eq!(DatImage::from_flag("J"), DatImage::Preview);
        // Unknown flags fall back to the name banner.
        assert_eq!(DatImage::from_flag("X"), DatImage::Name);
    }

    #[test]
    fn decodes_embedded_image() {
        // Build a minimal .dat whose name-banner pointer (0x38) references a
        // tiny 2x2 YUV image block.
        let mut data = vec![0u8; 0x60];
        data[0..4].copy_from_slice(INFO_MAGIC);
        let block_offset = 0x60u32;
        data[NAME_OFFSET_PTR..NAME_OFFSET_PTR + 4].copy_from_slice(&block_offset.to_le_bytes());

        // 2x2 YUV image: header + one literal quad (6 bytes).
        let mut block = vec![0u8; 16];
        block[0..2].copy_from_slice(&2u16.to_le_bytes()); // width
        block[2..4].copy_from_slice(&2u16.to_le_bytes()); // height
        block[4..8].copy_from_slice(&12u32.to_le_bytes()); // img_size
        block[8..10].copy_from_slice(&0x8001u16.to_le_bytes()); // literal 1 quad
        for (i, v) in block.iter_mut().enumerate().skip(10).take(6) {
            *v = 128u8.wrapping_add(i as u8);
        }
        data.extend_from_slice(&block);

        let img = decode_image(&data, Colorspace::YUV, DatImage::Name).expect("decode");
        assert_eq!(img.width, 2);
        assert_eq!(img.height, 2);
    }
}
