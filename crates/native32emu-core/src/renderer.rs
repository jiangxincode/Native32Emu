// Renderer: draws the current frame's visible objects sorted by depth.

use std::path::Path;

use crate::file_loader::{FrameObject, Native32Reader, ObjectType};
use crate::image_decoder::RgbaImage;
use crate::sprite_system::SpriteSystem;

pub struct DrawEntry {
    pub image_index: u32,
    pub x: i32,
    pub y: i32,
    pub depth: u32,
}

pub struct Renderer {
    pub screen_x: i32,
    pub screen_y: i32,
    pub buffer: Vec<u32>,
    pub width: u32,
    pub height: u32,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            screen_x: 0,
            screen_y: 0,
            buffer: vec![0xFF000000; (width * height) as usize],
            width,
            height,
        }
    }

    /// Resize the display buffer (e.g., when scaling changes).
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.buffer = vec![0xFF000000; (width * height) as usize];
    }

    /// Draw the current frame to the internal buffer.
    pub fn draw_frame(
        &mut self,
        reader: &mut Native32Reader,
        sprites: &SpriteSystem,
        cur_frame: &[FrameObject],
    ) {
        // Clear to opaque black
        for pixel in self.buffer.iter_mut() {
            *pixel = 0xFF000000;
        }

        // Collect draw entries
        let mut drawlist: Vec<DrawEntry> = Vec::new();

        // Image objects from the current frame
        for obj in cur_frame {
            if obj.obj_type == ObjectType::Image {
                drawlist.push(DrawEntry {
                    image_index: obj.index as u32,
                    x: obj.x as i32,
                    y: obj.y as i32,
                    depth: obj.depth as u32,
                });
            }
        }

        // Visible movie instances
        for (_, movie) in sprites.iter() {
            if !movie.visible {
                continue;
            }
            let movie_frames = reader.get_movie(movie.movie);
            if movie.frame < movie_frames.len() {
                let frame = &movie_frames[movie.frame];
                drawlist.push(DrawEntry {
                    image_index: frame.image as u32,
                    x: movie.x as i32 + frame.x as i32,
                    y: movie.y as i32 + frame.y as i32,
                    depth: movie.depth as u32,
                });
            }
        }

        // Sort by depth (stable to preserve insertion order for equal depths)
        drawlist.sort_by_key(|d| d.depth);

        // Draw each entry
        for entry in &drawlist {
            if let Some(img) = reader.get_image(entry.image_index) {
                self.blit_image(&img, entry.x + self.screen_x, entry.y + self.screen_y);
            }
        }
    }

    /// Blit an ARGB image to the buffer with transparency.
    fn blit_image(&mut self, img: &RgbaImage, dst_x: i32, dst_y: i32) {
        for sy in 0..img.height as i32 {
            let dy = dst_y + sy;
            if dy < 0 || dy >= self.height as i32 {
                continue;
            }
            for sx in 0..img.width as i32 {
                let dx = dst_x + sx;
                if dx < 0 || dx >= self.width as i32 {
                    continue;
                }
                let src_pixel = img.pixels[(sy as u32 * img.width + sx as u32) as usize];
                // Skip fully transparent pixels (alpha == 0)
                if (src_pixel >> 24) == 0 {
                    continue;
                }
                self.buffer[(dy as u32 * self.width + dx as u32) as usize] = src_pixel;
            }
        }
    }

    /// Save the current buffer as a PNG screenshot.
    pub fn save_screenshot(&self, path: &Path) -> anyhow::Result<()> {
        let mut img = image::RgbaImage::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let pixel = self.buffer[(y * self.width + x) as usize];
                let r = ((pixel >> 16) & 0xFF) as u8;
                let g = ((pixel >> 8) & 0xFF) as u8;
                let b = (pixel & 0xFF) as u8;
                let a = ((pixel >> 24) & 0xFF) as u8;
                img.put_pixel(x, y, image::Rgba([r, g, b, a]));
            }
        }
        img.save(path)?;
        Ok(())
    }
}
