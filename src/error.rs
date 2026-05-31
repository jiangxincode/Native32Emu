// Error types for the Native32 emulator.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EmuError {
    #[error("Native32 header not found in file")]
    HeaderNotFound,

    #[error("Header decryption failed - no valid key found")]
    DecryptionFailed,

    #[error("Invalid game file: {0}")]
    InvalidFile(String),

    #[error("Corrupted image data at offset 0x{offset:08x}: {message}")]
    CorruptedImage { offset: usize, message: String },

    #[error("Unknown action opcode 0x{opcode:02x} at instruction {pc}")]
    UnknownOpcode { opcode: u32, pc: usize },

    #[error("Resource offset 0x{offset:08x} exceeds file size for {resource_type}")]
    OffsetOutOfBounds {
        offset: usize,
        resource_type: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, EmuError>;
