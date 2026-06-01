// Gamepad overlay: draws a virtual gamepad on top of the game frame.

use std::collections::HashSet;

/// Native32 keycodes for the default button mapping
const KEY_UP: u16 = 0x1c00;
const KEY_DOWN: u16 = 0x1e00;
const KEY_LEFT: u16 = 0x0200;
const KEY_RIGHT: u16 = 0x0400;
const KEY_A: u16 = 0x4000;
const KEY_B: u16 = 0x8800;

/// Idle button color (dim white)
const COLOR_IDLE: u32 = 0xFF505050;
/// Pressed D-pad color (bright cyan)
const COLOR_DPAD_PRESSED: u32 = 0xFF00DDFF;
/// Pressed A button color (bright green)
const COLOR_A_PRESSED: u32 = 0xFF00EE44;
/// Pressed B button color (bright orange)
const COLOR_B_PRESSED: u32 = 0xFFFF8800;
/// Background color for overlay pads
const COLOR_BG: u32 = 0xAA1A1A1A;
/// Label text color
const COLOR_LABEL: u32 = 0xFFCCCCCC;

/// Simple 5x7 bitmap font for characters used on the overlay (A, B, arrows).
/// Each char is 7 bytes, each byte's low 5 bits represent one row (MSB = left).
const FONT_A: [u8; 7] = [
    0b01110, //  .XXX.
    0b10001, //  X...X
    0b10001, //  X...X
    0b11111, //  XXXXX
    0b10001, //  X...X
    0b10001, //  X...X
    0b10001, //  X...X
];

const FONT_B: [u8; 7] = [
    0b11110, //  XXXX.
    0b10001, //  X...X
    0b10001, //  X...X
    0b11110, //  XXXX.
    0b10001, //  X...X
    0b10001, //  X...X
    0b11110, //  XXXX.
];

// Arrow glyphs (3x5 inside a 5x7 cell, centered)
const ARROW_UP: [u8; 7] = [
    0b00100, //  ..X..
    0b01110, //  .XXX.
    0b11111, //  XXXXX
    0b00100, //  ..X..
    0b00100, //  ..X..
    0b00100, //  ..X..
    0b00100, //  ..X..
];

const ARROW_DOWN: [u8; 7] = [
    0b00100, //  ..X..
    0b00100, //  ..X..
    0b00100, //  ..X..
    0b00100, //  ..X..
    0b11111, //  XXXXX
    0b01110, //  .XXX.
    0b00100, //  ..X..
];

const ARROW_LEFT: [u8; 7] = [
    0b00100, //  ..X..
    0b01000, //  .X...
    0b11111, //  XXXXX
    0b01000, //  .X...
    0b00100, //  ..X..
    0b00000, //  .....
    0b00000, //  .....
];

const ARROW_RIGHT: [u8; 7] = [
    0b00100, //  ..X..
    0b00010, //  ...X.
    0b11111, //  XXXXX
    0b00010, //  ...X.
    0b00100, //  ..X..
    0b00000, //  .....
    0b00000, //  .....
];

pub struct GamepadOverlay;

impl GamepadOverlay {
    /// Draw the virtual gamepad overlay on the pixel buffer.
    ///
    /// - `buffer`: the ARGB8888 pixel buffer
    /// - `width`, `height`: buffer dimensions in pixels
    /// - `scale`: the integer scale factor
    /// - `pressed`: set of currently pressed Native32 keycodes
    pub fn draw(buffer: &mut [u32], width: u32, height: u32, scale: u32, pressed: &HashSet<u16>) {
        let u = (3 * scale) as i32; // base unit size
        let margin = (2 * scale) as i32;

        // --- D-pad (bottom-left) ---
        let dpad_cx = margin + 5 * u;
        let dpad_cy = height as i32 - margin - 5 * u;

        // Background pad for D-pad area (11u x 11u)
        Self::fill_rect_alpha(
            buffer, width, height,
            dpad_cx - 5 * u, dpad_cy - 5 * u,
            11 * u, 11 * u,
            COLOR_BG,
        );

        // Center square
        Self::fill_rect(buffer, width, height, dpad_cx - u, dpad_cy - u, 3 * u, 3 * u, COLOR_IDLE);

        // Up arm
        let up_color = if pressed.contains(&KEY_UP) { COLOR_DPAD_PRESSED } else { COLOR_IDLE };
        Self::fill_rect(buffer, width, height, dpad_cx - u, dpad_cy - 5 * u, 3 * u, 4 * u, up_color);

        // Down arm
        let down_color = if pressed.contains(&KEY_DOWN) { COLOR_DPAD_PRESSED } else { COLOR_IDLE };
        Self::fill_rect(buffer, width, height, dpad_cx - u, dpad_cy + 2 * u, 3 * u, 4 * u, down_color);

        // Left arm
        let left_color = if pressed.contains(&KEY_LEFT) { COLOR_DPAD_PRESSED } else { COLOR_IDLE };
        Self::fill_rect(buffer, width, height, dpad_cx - 5 * u, dpad_cy - u, 4 * u, 3 * u, left_color);

        // Right arm
        let right_color = if pressed.contains(&KEY_RIGHT) { COLOR_DPAD_PRESSED } else { COLOR_IDLE };
        Self::fill_rect(buffer, width, height, dpad_cx + 2 * u, dpad_cy - u, 4 * u, 3 * u, right_color);

        // Draw arrow glyphs on each arm
        let glyph_scale = if scale >= 3 { 2 } else { 1 };
        Self::draw_glyph(buffer, width, height, &ARROW_UP, dpad_cx - u, dpad_cy - 4 * u, 3 * u, glyph_scale, COLOR_LABEL);
        Self::draw_glyph(buffer, width, height, &ARROW_DOWN, dpad_cx - u, dpad_cy + 2 * u, 3 * u, glyph_scale, COLOR_LABEL);
        Self::draw_glyph(buffer, width, height, &ARROW_LEFT, dpad_cx - 4 * u, dpad_cy - u, 3 * u, glyph_scale, COLOR_LABEL);
        Self::draw_glyph(buffer, width, height, &ARROW_RIGHT, dpad_cx + 2 * u, dpad_cy - u, 3 * u, glyph_scale, COLOR_LABEL);

        // --- A/B buttons (bottom-right) ---
        let btn_radius = 2 * u;
        let btn_a_cx = width as i32 - margin - 3 * u;
        let btn_a_cy = height as i32 - margin - 4 * u;
        let btn_b_cx = width as i32 - margin - 8 * u;
        let btn_b_cy = height as i32 - margin - 2 * u;

        // Background pad for button area
        Self::fill_rect_alpha(
            buffer, width, height,
            btn_b_cx - 3 * u, btn_a_cy - 3 * u,
            12 * u, 8 * u,
            COLOR_BG,
        );

        // A button
        let a_color = if pressed.contains(&KEY_A) { COLOR_A_PRESSED } else { COLOR_IDLE };
        Self::fill_circle(buffer, width, height, btn_a_cx, btn_a_cy, btn_radius, a_color);

        // B button
        let b_color = if pressed.contains(&KEY_B) { COLOR_B_PRESSED } else { COLOR_IDLE };
        Self::fill_circle(buffer, width, height, btn_b_cx, btn_b_cy, btn_radius, b_color);

        // Draw letter glyphs on buttons
        Self::draw_glyph(buffer, width, height, &FONT_A, btn_a_cx - u, btn_a_cy - u, 3 * u, glyph_scale, COLOR_LABEL);
        Self::draw_glyph(buffer, width, height, &FONT_B, btn_b_cx - u, btn_b_cy - u, 3 * u, glyph_scale, COLOR_LABEL);
    }

    /// Fill a rectangle with a solid color.
    fn fill_rect(buffer: &mut [u32], bw: u32, bh: u32, x: i32, y: i32, w: i32, h: i32, color: u32) {
        for py in y..y + h {
            if py < 0 || py >= bh as i32 { continue; }
            for px in x..x + w {
                if px < 0 || px >= bw as i32 { continue; }
                buffer[(py as u32 * bw + px as u32) as usize] = color;
            }
        }
    }

    /// Fill a rectangle with alpha blending (semi-transparent).
    fn fill_rect_alpha(buffer: &mut [u32], bw: u32, bh: u32, x: i32, y: i32, w: i32, h: i32, color: u32) {
        let a = ((color >> 24) & 0xFF) as u32;
        let inv_a = 255 - a;
        let sr = ((color >> 16) & 0xFF) as u32;
        let sg = ((color >> 8) & 0xFF) as u32;
        let sb = (color & 0xFF) as u32;

        for py in y..y + h {
            if py < 0 || py >= bh as i32 { continue; }
            for px in x..x + w {
                if px < 0 || px >= bw as i32 { continue; }
                let idx = (py as u32 * bw + px as u32) as usize;
                let dst = buffer[idx];
                let dr = ((dst >> 16) & 0xFF) as u32;
                let dg = ((dst >> 8) & 0xFF) as u32;
                let db = (dst & 0xFF) as u32;
                let r = (sr * a + dr * inv_a) / 255;
                let g = (sg * a + dg * inv_a) / 255;
                let b = (sb * a + db * inv_a) / 255;
                buffer[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
            }
        }
    }

    /// Fill a circle with a solid color.
    fn fill_circle(buffer: &mut [u32], bw: u32, bh: u32, cx: i32, cy: i32, r: i32, color: u32) {
        let r2 = r * r;
        for dy in -r..=r {
            let py = cy + dy;
            if py < 0 || py >= bh as i32 { continue; }
            for dx in -r..=r {
                let px = cx + dx;
                if px < 0 || px >= bw as i32 { continue; }
                if dx * dx + dy * dy <= r2 {
                    buffer[(py as u32 * bw + px as u32) as usize] = color;
                }
            }
        }
    }

    /// Draw a 5x7 bitmap glyph scaled to fit within a cell of `cell_size` pixels.
    /// The glyph is centered within the cell.
    fn draw_glyph(
        buffer: &mut [u32],
        bw: u32,
        bh: u32,
        glyph: &[u8; 7],
        cell_x: i32,
        cell_y: i32,
        cell_size: i32,
        pixel_scale: i32,
        color: u32,
    ) {
        let glyph_w = 5 * pixel_scale;
        let glyph_h = 7 * pixel_scale;
        let ox = cell_x + (cell_size - glyph_w) / 2;
        let oy = cell_y + (cell_size - glyph_h) / 2;

        for row in 0i32..7 {
            let bits = glyph[row as usize];
            for col in 0i32..5 {
                if bits & (1 << (4 - col)) != 0 {
                    // Draw a pixel_scale x pixel_scale block
                    for sy in 0..pixel_scale {
                        let py = oy + row * pixel_scale + sy;
                        if py < 0 || py >= bh as i32 { continue; }
                        for sx in 0..pixel_scale {
                            let px = ox + col * pixel_scale + sx;
                            if px < 0 || px >= bw as i32 { continue; }
                            buffer[(py as u32 * bw + px as u32) as usize] = color;
                        }
                    }
                }
            }
        }
    }
}
