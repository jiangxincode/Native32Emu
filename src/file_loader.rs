// File loader: reads game files, skips thumbnails, locates headers, parses resource tables.

use std::collections::HashMap;

use crate::error::{EmuError, Result};
use crate::header_decryptor::decrypt_header;
use crate::image_decoder::{decode_image_argb, decode_image_yuv, RgbaImage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Image = 1,
    Movie = 2,
    Button = 3,
    Action = 4,
    Sound = 5,
}

impl ObjectType {
    pub fn from_u16(val: u16) -> Option<Self> {
        match val {
            1 => Some(ObjectType::Image),
            2 => Some(ObjectType::Movie),
            3 => Some(ObjectType::Button),
            4 => Some(ObjectType::Action),
            5 => Some(ObjectType::Sound),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameObject {
    pub obj_type: ObjectType,
    pub index: u16,
    pub x: i16,
    pub y: i16,
    pub depth: u16,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct MovieFrame {
    pub image: u16,
    pub x: i16,
    pub y: i16,
    pub action: u16,
    pub sound: u16,
    pub reserved: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    MP3,
    Raw,
}

#[derive(Debug, Clone)]
pub struct SoundData {
    pub format: AudioFormat,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colorspace {
    YUV,
    ARGB,
}

pub struct Native32Reader {
    pub data: Vec<u8>,
    pub idx: usize,
    pub colorspace: Colorspace,
    pub resolution: (u32, u32),
    pub base: usize,
    // Resource table offsets (relative to base)
    pub frame_idx: u32,
    pub image_idx: u32,
    pub action_idx: u32,
    pub movie_idx: u32,
    pub button_idx: u32,
    pub button_cond_idx: u32,
    pub mp3_offset: u32,
    pub sound_table: usize,
    // Caches
    actions_cache: Vec<Option<(crate::actions::Action, Option<ActionPayload>)>>,
    images_cache: HashMap<u32, Option<RgbaImage>>,
    frames_cache: HashMap<u32, Vec<FrameObject>>,
    movies_cache: HashMap<u32, Vec<MovieFrame>>,
    sound_cache: HashMap<u32, SoundData>,
    button_events_cache: HashMap<u32, Vec<(u16, u16)>>,
}

#[derive(Debug, Clone)]
pub enum ActionPayload {
    String(String),
    Integer(i16),
}

fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_i16_le(data: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

impl Native32Reader {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            idx: 0,
            colorspace: Colorspace::YUV,
            resolution: (320, 240),
            base: 0,
            frame_idx: 0,
            image_idx: 0,
            action_idx: 0,
            movie_idx: 0,
            button_idx: 0,
            button_cond_idx: 0,
            mp3_offset: 0,
            sound_table: 0,
            actions_cache: vec![None], // 1-based index, slot 0 unused
            images_cache: HashMap::new(),
            frames_cache: HashMap::new(),
            movies_cache: HashMap::new(),
            sound_cache: HashMap::new(),
            button_events_cache: HashMap::new(),
        }
    }

    fn get_str(&self, offset: usize) -> String {
        let mut s = String::new();
        let mut pos = offset;
        while pos < self.data.len() && self.data[pos] != 0 {
            s.push(self.data[pos] as char);
            pos += 1;
        }
        s
    }

    /// Skip the optional SWFT thumbnail at the beginning of the file.
    pub fn skip_thumbnail(&mut self) {
        if self.idx + 4 <= self.data.len() && &self.data[self.idx..self.idx + 4] == b"SWFT" {
            self.idx += 4;
            // Read thumbnail header: colorspace(4) + flags(4) + width(2) + height(2) + size(4)
            if self.idx + 0x10 <= self.data.len() {
                let size = read_u32_le(&self.data, self.idx + 0x0c) as usize;
                log::info!("Skipping SWFT thumbnail ({} bytes)", 0x10 + size);
                self.idx += 0x10 + size;
            }
        }
    }

    /// Scan for the `_YUV` or `ARGB` colorspace marker.
    pub fn find_header(&mut self) -> Result<()> {
        while self.idx + 4 <= self.data.len() {
            let magic = &self.data[self.idx..self.idx + 4];
            if magic == b"_YUV" {
                self.colorspace = Colorspace::YUV;
                log::info!("Found _YUV Native32 header at 0x{:x}", self.idx);
                return Ok(());
            } else if magic == b"ARGB" {
                self.colorspace = Colorspace::ARGB;
                log::info!("Found ARGB Native32 header at 0x{:x}", self.idx);
                return Ok(());
            }
            self.idx += 1;
        }
        Err(EmuError::HeaderNotFound)
    }

    /// Parse the header: generator string, base offset, encrypted section, resource table offsets.
    pub fn process_header(&mut self) -> Result<()> {
        // Parse generator string (48 bytes starting at colorspace+4)
        let gen_start = self.idx + 0x04;
        if gen_start + 0x30 <= self.data.len() {
            let gen_bytes = &self.data[gen_start..gen_start + 0x30];
            let gen_str = String::from_utf8_lossy(gen_bytes)
                .trim_end_matches('\0')
                .to_string();
            log::info!("Generator: {}", gen_str);

            // Try to parse Resolution_WxH
            if let Some((w, h)) = parse_resolution(&gen_str) {
                if w > 0 && h > 0 {
                    self.resolution = (w, h);
                    log::info!("Resolution: {}x{}", w, h);
                }
            }
        }

        // Base offset is at colorspace + 0x60
        self.base = self.idx + 0x60;
        if self.base + 0x38 > self.data.len() {
            return Err(EmuError::InvalidFile("Header extends beyond file".into()));
        }

        // Read header fields before encrypted section:
        // 8 bytes: fps_color_size(2) + action_stack_var(2) + button_movieclip(2) + buffer_sound(2)
        // 16 bytes: load_addr(4) + binary_size(4) + mp3_offset(4) + mp3_length(4)
        // Total: 24 = 0x18 bytes
        self.mp3_offset = read_u32_le(&self.data, self.base + 0x18);

        // Decrypt the 32-byte encrypted header at base + 0x18
        let enc_start = self.base + 0x18;
        if enc_start + 0x20 > self.data.len() {
            return Err(EmuError::InvalidFile("Encrypted header extends beyond file".into()));
        }
        let encrypted = &self.data[enc_start..enc_start + 0x20];
        let decrypted = decrypt_header(encrypted)
            .ok_or(EmuError::DecryptionFailed)?;

        // Parse resource table offsets from decrypted header
        // Layout: unkh(4) + magic(4) + frame(4) + image(4) + action(4) + movie(4) + button(4) + button_cond(4)
        self.frame_idx = read_u32_le(&decrypted, 0x08);
        self.image_idx = read_u32_le(&decrypted, 0x0c);
        self.action_idx = read_u32_le(&decrypted, 0x10);
        self.movie_idx = read_u32_le(&decrypted, 0x14);
        self.button_idx = read_u32_le(&decrypted, 0x18);
        self.button_cond_idx = read_u32_le(&decrypted, 0x1c);

        log::info!("Frame table: 0x{:08x}", self.frame_idx);
        log::info!("Image table: 0x{:08x}", self.image_idx);
        log::info!("Action table: 0x{:08x}", self.action_idx);
        log::info!("Movie table: 0x{:08x}", self.movie_idx);
        log::info!("Button table: 0x{:08x}", self.button_idx);
        log::info!("Button cond table: 0x{:08x}", self.button_cond_idx);

        // Skip cursor data (2 bytes width + 2 bytes height + pixel data)
        let cursor_pos = enc_start + 0x20; // after the 32-byte encrypted header
        if cursor_pos + 4 <= self.data.len() {
            let cursor_w = read_u16_le(&self.data, cursor_pos) as usize;
            let cursor_h = read_u16_le(&self.data, cursor_pos + 2) as usize;
            let cursor_size = 2 * cursor_w * cursor_h;
            self.sound_table = cursor_pos + 4 + cursor_size;
            log::info!("Sound table at 0x{:x}", self.sound_table);
        }

        Ok(())
    }

    /// Initialize: skip thumbnail, find header, process header.
    pub fn init(&mut self) -> Result<()> {
        self.skip_thumbnail();
        self.find_header()?;
        self.process_header()?;
        Ok(())
    }

    /// Disassemble a single action instruction at the given 1-based index.
    fn disassemble_action(&self, index: u32) -> Option<(crate::actions::Action, Option<ActionPayload>)> {
        let ptr = self.base + self.action_idx as usize + ((index - 1) * 8) as usize;
        if ptr + 8 > self.data.len() {
            return None;
        }
        let opcode = read_u32_le(&self.data, ptr);
        let payload_val = read_u32_le(&self.data, ptr + 4);

        let act = crate::actions::Action::from_u32(opcode)?;

        if payload_val == 0 || act == crate::actions::Action::End {
            return Some((act, None));
        }

        let payload_idx = self.base + payload_val as usize;
        if payload_idx >= self.data.len() {
            return Some((act, None));
        }

        match act {
            crate::actions::Action::If
            | crate::actions::Action::GotoFrame
            | crate::actions::Action::GotoFrame2
            | crate::actions::Action::Jump => {
                if payload_idx + 2 <= self.data.len() {
                    let val = read_i16_le(&self.data, payload_idx);
                    Some((act, Some(ActionPayload::Integer(val))))
                } else {
                    Some((act, None))
                }
            }
            _ => {
                let s = self.get_str(payload_idx);
                Some((act, Some(ActionPayload::String(s))))
            }
        }
    }

    /// Get an action by 1-based index (with lazy caching).
    pub fn get_action(&mut self, index: u32) -> Option<(crate::actions::Action, Option<ActionPayload>)> {
        while index as usize >= self.actions_cache.len() {
            let i = self.actions_cache.len() as u32;
            let action = self.disassemble_action(i);
            self.actions_cache.push(action);
        }
        self.actions_cache[index as usize].clone()
    }

    /// Get an action by 1-based index (immutable, assumes already cached).
    pub fn get_action_cached(&self, index: u32) -> Option<&(crate::actions::Action, Option<ActionPayload>)> {
        if (index as usize) < self.actions_cache.len() {
            self.actions_cache[index as usize].as_ref()
        } else {
            None
        }
    }

    /// Parse a frame's object list by 1-based frame index.
    pub fn get_frame(&mut self, frame: u32) -> Option<Vec<FrameObject>> {
        if let Some(cached) = self.frames_cache.get(&frame) {
            return Some(cached.clone());
        }

        let ptr_idx = self.base + self.frame_idx as usize + 4 * (frame - 1) as usize;
        if ptr_idx + 4 > self.data.len() {
            return None;
        }
        let offset = read_u32_le(&self.data, ptr_idx);
        if offset == 0 || offset as usize > self.data.len() {
            return None;
        }

        let mut objects = Vec::new();
        let mut i = self.base + offset as usize;
        while i + 0x10 <= self.data.len() {
            let obj_type = read_u16_le(&self.data, i);
            if obj_type == 0x0000 || obj_type == 0xFFFF {
                break;
            }
            let index = read_u16_le(&self.data, i + 2);
            let x = read_i16_le(&self.data, i + 4);
            let y = read_i16_le(&self.data, i + 6);
            let depth = read_u16_le(&self.data, i + 8);
            let name_ptr = read_u32_le(&self.data, i + 12);

            let name = if name_ptr != 0 {
                Some(self.get_str(self.base + name_ptr as usize))
            } else {
                None
            };

            if let Some(ot) = ObjectType::from_u16(obj_type) {
                objects.push(FrameObject {
                    obj_type: ot,
                    index,
                    x,
                    y,
                    depth,
                    name,
                });
            }
            i += 0x10;
        }

        self.frames_cache.insert(frame, objects.clone());
        Some(objects)
    }

    /// Parse a movie's frame list by 1-based movie index.
    pub fn get_movie(&mut self, movie: u32) -> Vec<MovieFrame> {
        if let Some(cached) = self.movies_cache.get(&movie) {
            return cached.clone();
        }

        let idx_ptr = self.base + self.movie_idx as usize + 4 * (movie - 1) as usize;
        if idx_ptr + 4 > self.data.len() {
            return Vec::new();
        }
        let ptr = read_u32_le(&self.data, idx_ptr) as usize + self.base;

        let mut frames = Vec::new();
        let mut pos = ptr;
        while pos + 0x0C <= self.data.len() {
            let img = read_u16_le(&self.data, pos);
            if img == 0xFFFF || img == 0x0000 {
                break;
            }
            if img as usize >= self.data.len() {
                break;
            }
            frames.push(MovieFrame {
                image: img,
                x: read_i16_le(&self.data, pos + 2),
                y: read_i16_le(&self.data, pos + 4),
                action: read_u16_le(&self.data, pos + 6),
                sound: read_u16_le(&self.data, pos + 8),
                reserved: read_u16_le(&self.data, pos + 10),
            });
            pos += 0x0C;
        }

        self.movies_cache.insert(movie, frames.clone());
        frames
    }

    /// Get decoded image by 1-based index (with lazy caching).
    pub fn get_image(&mut self, index: u32) -> Option<RgbaImage> {
        if let Some(cached) = self.images_cache.get(&index) {
            return cached.clone();
        }

        let ptr = self.base + self.image_idx as usize + 4 * (index - 1) as usize;
        if ptr + 4 > self.data.len() {
            self.images_cache.insert(index, None);
            return None;
        }
        let img_offset = read_u32_le(&self.data, ptr);
        if img_offset == 0xFFFFFFFF {
            self.images_cache.insert(index, None);
            return None;
        }

        let img_start = self.base + img_offset as usize;
        if img_start + 8 > self.data.len() {
            self.images_cache.insert(index, None);
            return None;
        }

        let img_size = read_u32_le(&self.data, img_start + 4) as usize;
        let img_end = std::cmp::min(img_start + 8 + img_size, self.data.len());

        let img_data = &self.data[img_start..img_end];
        let result = match self.colorspace {
            Colorspace::ARGB => decode_image_argb(img_data),
            Colorspace::YUV => decode_image_yuv(img_data),
        };

        self.images_cache.insert(index, result.clone());
        result
    }

    /// Get sound data by 1-based index.
    pub fn get_sound(&mut self, idx: u32) -> Option<SoundData> {
        if let Some(cached) = self.sound_cache.get(&idx) {
            return Some(cached.clone());
        }

        let table_idx = self.sound_table + (idx - 1) as usize * 4;
        if table_idx + 4 > self.data.len() {
            return None;
        }
        let ptr = read_u32_le(&self.data, table_idx);
        let flags = ptr & 0xF0000000;
        let addr = (ptr & 0x0FFFFFFF) as usize;

        let result = if flags == 0xF0000000 {
            // MP3 audio
            let begin = self.base + self.mp3_offset as usize + addr;
            if begin + 6 > self.data.len() {
                return None;
            }
            let size = read_u32_le(&self.data, begin) as usize;
            let data_start = begin + 6;
            let data_end = std::cmp::min(data_start + size, self.data.len());
            SoundData {
                format: AudioFormat::MP3,
                data: self.data[data_start..data_end].to_vec(),
            }
        } else if flags == 0x00000000 {
            // Raw PCM
            let begin = self.base + addr;
            if begin + 4 > self.data.len() {
                return None;
            }
            let size = read_u32_le(&self.data, begin) as usize;
            let data_start = begin + 4;
            let data_end = std::cmp::min(data_start + size, self.data.len());
            let raw_data = &self.data[data_start..data_end];

            if self.colorspace == Colorspace::YUV {
                // Big-endian to little-endian conversion
                SoundData {
                    format: AudioFormat::Raw,
                    data: endian_swap_resample(raw_data),
                }
            } else {
                SoundData {
                    format: AudioFormat::Raw,
                    data: raw_data.to_vec(),
                }
            }
        } else {
            return None;
        };

        self.sound_cache.insert(idx, result.clone());
        Some(result)
    }

    /// Get button events by 1-based button index.
    pub fn get_button_events(&mut self, button: u32) -> Vec<(u16, u16)> {
        if let Some(cached) = self.button_events_cache.get(&button) {
            return cached.clone();
        }

        let cond_table_idx = self.base + self.button_cond_idx as usize + (button - 1) as usize * 4;
        if cond_table_idx + 4 > self.data.len() {
            return Vec::new();
        }
        let ptr = read_u32_le(&self.data, cond_table_idx) as usize + self.base;
        if ptr + 2 > self.data.len() {
            return Vec::new();
        }

        let total_act_len = read_u16_le(&self.data, ptr) as usize;
        let mut pos = ptr + 2;
        let mut i = 0;
        let mut events = Vec::new();

        while i < total_act_len && pos + 6 <= self.data.len() {
            let keycode = read_u16_le(&self.data, pos);
            let act_len = read_u16_le(&self.data, pos + 2);
            let event = read_u16_le(&self.data, pos + 4);
            events.push((keycode, event));
            i += act_len as usize;
            pos += 6;
        }

        self.button_events_cache.insert(button, events.clone());
        events
    }

    /// Pre-cache all actions for the entire file.
    pub fn cache_all_actions(&mut self) {
        let mut i = 1u32;
        loop {
            if self.disassemble_action(i).is_none() {
                break;
            }
            // Already cached by disassemble_action call above
            i += 1;
            // Ensure cache is populated
            while (i as usize) >= self.actions_cache.len() {
                let idx = self.actions_cache.len() as u32;
                let action = self.disassemble_action(idx);
                self.actions_cache.push(action);
            }
            // Check if the last one was None
            if self.actions_cache.last().map_or(true, |a| a.is_none()) {
                break;
            }
        }
    }
}

fn parse_resolution(gen_str: &str) -> Option<(u32, u32)> {
    // Match "Resolution_<width>_<height>"
    if let Some(rest) = gen_str.strip_prefix("Resolution_") {
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.len() >= 2 {
            if let (Ok(w), Ok(h)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                return Some((w, h));
            }
        }
    }
    None
}

fn endian_swap_resample(data: &[u8]) -> Vec<u8> {
    let len = data.len() & 0xFFFFFFFE;
    let mut result = vec![0u8; len * 2];
    for i in 0..(2 * len) {
        result[i] = data[(2 * (i / 4)) | ((i & 0x1) ^ 0x1)];
    }
    result
}
