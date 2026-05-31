// Image decoder for Native32 YUV and ARGB image formats.
// Decodes compressed image data into ARGB pixel buffers.

#[derive(Debug, Clone)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>, // ARGB format, row-major
}

fn read_u16_le_slice(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u32_le_slice(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn clip(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// Interpolate U/V vertically (2x upsampling).
fn interpolate_y(data: &[u8], w: usize, h: usize) -> Vec<u8> {
    let h1 = h * 2;
    let mut result = vec![0u8; w * h1];
    for y in 0..h {
        for dy in 0..2 {
            let y1 = y * 2 + dy;
            for x in 0..w {
                let val = if dy == 0 {
                    if y == 0 || data[y * w + x] != 0 {
                        data[y * w + x]
                    } else {
                        data[(y - 1) * w + x]
                    }
                } else {
                    if y == h - 1 || data[y * w + x] != 0 {
                        data[y * w + x]
                    } else {
                        data[(y + 1) * w + x]
                    }
                };
                result[y1 * w + x] = val;
            }
        }
    }
    result
}

/// Interpolate U/V horizontally (2x upsampling).
fn interpolate_x(data: &[u8], w: usize, h: usize) -> Vec<u8> {
    let w1 = w * 2;
    let mut result = vec![0u8; w1 * h];
    for y in 0..h {
        for x in 0..w {
            for dx in 0..2 {
                let x1 = x * 2 + dx;
                let val = if dx == 0 {
                    if x == 0 || data[y * w + x] != 0 {
                        data[y * w + x]
                    } else {
                        data[y * w + (x - 1)]
                    }
                } else {
                    if x == w - 1 || data[y * w + x] != 0 {
                        data[y * w + x]
                    } else {
                        data[y * w + (x + 1)]
                    }
                };
                result[y * w1 + x1] = val;
            }
        }
    }
    result
}

/// Decode a YUV 4:2:0 image with packbits-like compression.
pub fn decode_image_yuv(data: &[u8]) -> Option<RgbaImage> {
    if data.len() < 8 {
        return None;
    }

    let width = read_u16_le_slice(data, 0) as usize;
    let height = read_u16_le_slice(data, 2) as usize;
    let img_size = read_u32_le_slice(data, 4) as usize;

    if width == 0 || height == 0 {
        return None;
    }

    let mut y_2_2 = vec![0u8; width * height];
    let uv_w = width / 2;
    let uv_h = height / 2;
    let mut u_1_1 = vec![0u8; uv_w * uv_h];
    let mut v_1_1 = vec![0u8; uv_w * uv_h];

    // Helper to put a 2x2 quad of Y values + U/V
    let putquad =
        |pix: usize, y_buf: &mut [u8], u_buf: &mut [u8], v_buf: &mut [u8], chunk: &[u8]| {
            let y_coord = pix / uv_w;
            let x_coord = pix % uv_w;
            // Y values: x0y0, x0y1, x1y0, x1y1
            y_buf[(2 * y_coord) * width + (2 * x_coord)] = chunk[0];
            y_buf[(2 * y_coord + 1) * width + (2 * x_coord)] = chunk[1];
            y_buf[(2 * y_coord) * width + (2 * x_coord + 1)] = chunk[2];
            y_buf[(2 * y_coord + 1) * width + (2 * x_coord + 1)] = chunk[3];
            // V is byte 4, U is byte 5
            v_buf[pix] = chunk[4];
            u_buf[pix] = chunk[5];
        };

    let mut pixel: usize = 0;
    let mut i: usize = 8;
    let max_pixels = uv_w * uv_h;

    while i + 2 <= data.len() && i < img_size + 8 && pixel < max_pixels {
        let op = read_u16_le_slice(data, i) as usize;
        if op == 0 {
            log::error!("Corrupted YUV image: command value 0 at offset 0x{:x}", i);
            return None;
        }
        i += 2;

        if op & 0x8000 != 0 {
            // N quads of literal data
            let count = op & 0x7FFF;
            for _ in 0..count {
                if i + 6 > data.len() || pixel >= max_pixels {
                    break;
                }
                putquad(pixel, &mut y_2_2, &mut u_1_1, &mut v_1_1, &data[i..i + 6]);
                pixel += 1;
                i += 6;
            }
        } else {
            // Repeat 1 quad N times
            if i + 6 > data.len() {
                break;
            }
            let chunk = [
                data[i],
                data[i + 1],
                data[i + 2],
                data[i + 3],
                data[i + 4],
                data[i + 5],
            ];
            i += 6;
            for _ in 0..op {
                if pixel >= max_pixels {
                    break;
                }
                putquad(pixel, &mut y_2_2, &mut u_1_1, &mut v_1_1, &chunk);
                pixel += 1;
            }
        }
    }

    // Upsample U/V: vertical first, then horizontal
    let u_2_2 = interpolate_x(&interpolate_y(&u_1_1, uv_w, uv_h), uv_w, height);
    let v_2_2 = interpolate_x(&interpolate_y(&v_1_1, uv_w, uv_h), uv_w, height);

    // Convert YUV to ARGB
    let mut pixels = vec![0u32; width * height];
    for i in 0..(width * height) {
        if y_2_2[i] == 0 {
            // Transparent
            pixels[i] = 0x00000000;
        } else {
            let c = y_2_2[i] as i32 - 16;
            let d = u_2_2[i] as i32 - 128;
            let e = v_2_2[i] as i32 - 128;
            let r = clip((298 * c + 409 * e + 128) >> 8);
            let g = clip((298 * c - 100 * d - 208 * e + 128) >> 8);
            let b = clip((298 * c + 516 * d + 128) >> 8);
            pixels[i] = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        }
    }

    Some(RgbaImage {
        width: width as u32,
        height: height as u32,
        pixels,
    })
}

/// Decode an ARGB1555 image with run-length encoding.
pub fn decode_image_argb(data: &[u8]) -> Option<RgbaImage> {
    if data.len() < 8 {
        return None;
    }

    let width = read_u16_le_slice(data, 0) as usize;
    let height = read_u16_le_slice(data, 2) as usize;
    let img_size = read_u32_le_slice(data, 4) as usize;

    if width == 0 || height == 0 {
        return None;
    }

    let total_pixels = width * height;
    let mut pixels = vec![0u32; total_pixels];
    let mut pixel: usize = 0;
    let mut i: usize = 8;

    while i + 2 <= data.len() && i < img_size + 8 && pixel < total_pixels {
        let op = read_u16_le_slice(data, i);

        if op == 0x0000 {
            // Literal transparent pixel
            pixels[pixel] = 0x00000000;
            pixel += 1;
            i += 2;
        } else if op & 0xC000 == 0xC000 {
            // Repeat pixel N times
            let count = (op & 0x3FFF) as usize;
            if i + 4 > data.len() {
                break;
            }
            let value = read_u16_le_slice(data, i + 2);
            let argb = argb1555_to_argb(value);
            for _ in 0..count {
                if pixel >= total_pixels {
                    break;
                }
                pixels[pixel] = argb;
                pixel += 1;
            }
            i += 4;
        } else {
            log::error!("Unknown ARGB command 0x{:04x} at offset 0x{:06x}", op, i);
            return None;
        }
    }

    Some(RgbaImage {
        width: width as u32,
        height: height as u32,
        pixels,
    })
}

/// Convert ARGB1555 pixel to ARGB8888.
fn argb1555_to_argb(value: u16) -> u32 {
    if value & 0x8000 == 0 {
        // Transparent
        0x00000000
    } else {
        let r = ((value >> 0) & 0x1F) << 3;
        let g = ((value >> 5) & 0x1F) << 3;
        let b = ((value >> 10) & 0x1F) << 3;
        0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
}

fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}
