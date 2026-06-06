// libretro callback management.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use super::types::*;
use std::ffi::c_void;

/// Global callback storage
pub struct Callbacks {
    pub environment: Option<retro_environment_t>,
    pub video_refresh: Option<retro_video_refresh_t>,
    pub audio_sample: Option<retro_audio_sample_t>,
    pub audio_sample_batch: Option<retro_audio_sample_batch_t>,
    pub input_poll: Option<retro_input_poll_t>,
    pub input_state: Option<retro_input_state_t>,
    pub log: Option<retro_log_printf_t>,
}

impl Default for Callbacks {
    fn default() -> Self {
        Self::new()
    }
}

impl Callbacks {
    pub const fn new() -> Self {
        Self {
            environment: None,
            video_refresh: None,
            audio_sample: None,
            audio_sample_batch: None,
            input_poll: None,
            input_state: None,
            log: None,
        }
    }
}

/// Global static callbacks storage
pub static mut CALLBACKS: Callbacks = Callbacks::new();

/// Initialize callbacks from retro_set_* calls
pub fn set_environment(cb: retro_environment_t) {
    unsafe {
        CALLBACKS.environment = Some(cb);
    }
}

pub fn set_video_refresh(cb: retro_video_refresh_t) {
    unsafe {
        CALLBACKS.video_refresh = Some(cb);
    }
}

pub fn set_audio_sample(cb: retro_audio_sample_t) {
    unsafe {
        CALLBACKS.audio_sample = Some(cb);
    }
}

pub fn set_audio_sample_batch(cb: retro_audio_sample_batch_t) {
    unsafe {
        CALLBACKS.audio_sample_batch = Some(cb);
    }
}

pub fn set_input_poll(cb: retro_input_poll_t) {
    unsafe {
        CALLBACKS.input_poll = Some(cb);
    }
}

pub fn set_input_state(cb: retro_input_state_t) {
    unsafe {
        CALLBACKS.input_state = Some(cb);
    }
}

/// Get log interface from frontend
pub fn init_log() {
    unsafe {
        if let Some(env) = CALLBACKS.environment {
            let mut log_cb = retro_log_callback { log: fallback_log };
            let success = env(
                super::constants::RETRO_ENVIRONMENT_GET_LOG_INTERFACE,
                &mut log_cb as *mut _ as *mut c_void,
            );
            if success {
                CALLBACKS.log = Some(log_cb.log);
            }
        }
    }
}

/// Fallback log function that writes to stderr
unsafe extern "C" fn fallback_log(level: retro_log_level, fmt: *const std::ffi::c_char) {
    let level_str = match level {
        retro_log_level::RETRO_LOG_DEBUG => "DEBUG",
        retro_log_level::RETRO_LOG_INFO => "INFO",
        retro_log_level::RETRO_LOG_WARN => "WARN",
        retro_log_level::RETRO_LOG_ERROR => "ERROR",
    };
    // Simple fallback: just print the level
    eprintln!("[Native32Emu] {}", level_str);
    let _ = fmt;
}

/// Call environment callback
pub fn environment(cmd: u32, data: *mut c_void) -> bool {
    unsafe {
        match CALLBACKS.environment {
            Some(env) => env(cmd, data),
            None => false,
        }
    }
}

/// Call video refresh callback
pub fn video_refresh(data: *const c_void, width: u32, height: u32, pitch: usize) {
    unsafe {
        if let Some(cb) = CALLBACKS.video_refresh {
            cb(data, width, height, pitch);
        }
    }
}

/// Call single audio sample callback
pub fn audio_sample(left: i16, right: i16) {
    unsafe {
        if let Some(cb) = CALLBACKS.audio_sample {
            cb(left, right);
        }
    }
}

/// Call batch audio sample callback
pub fn audio_sample_batch(data: *const i16, frames: usize) -> usize {
    unsafe {
        match CALLBACKS.audio_sample_batch {
            Some(cb) => cb(data, frames),
            None => 0,
        }
    }
}

/// Call input poll callback
pub fn input_poll() {
    unsafe {
        if let Some(cb) = CALLBACKS.input_poll {
            cb();
        }
    }
}

/// Call input state callback
pub fn input_state(port: u32, device: u32, index: u32, id: u32) -> i16 {
    unsafe {
        match CALLBACKS.input_state {
            Some(cb) => cb(port, device, index, id),
            None => 0,
        }
    }
}

/// Log a message using the frontend's log interface
pub fn log_message(level: retro_log_level, msg: &str) {
    unsafe {
        if let Some(log_fn) = CALLBACKS.log {
            // The frontend's log function is printf-style and treats its string
            // argument as a format string. Our message is already fully
            // formatted, so any literal '%' must be escaped to '%%' to avoid
            // format-string interpretation (which would read non-existent
            // variadic arguments and may crash the frontend).
            let escaped = msg.replace('%', "%%");
            let c_msg = std::ffi::CString::new(escaped).unwrap_or_default();
            log_fn(level, c_msg.as_ptr());
        }
    }
}
