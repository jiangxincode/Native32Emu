// Pure-Rust MPEG-1 playback for SSL_PlayNext pre-content cutscenes/logos.
//
// Ported from PL_MPEG (https://github.com/phoboslab/pl_mpeg) by Dominic
// Szablewski, which is MIT licensed. This avoids any C/FFI dependency so the
// decoder builds the same way on every target the emulator supports.
//
// Module layout (built up incrementally):
//   - buffer: MSB-first bit reader over an in-memory stream
//   - demux:  MPEG-PS demuxer (splits video/audio elementary streams)
//   - video:  MPEG-1 video decoder (TODO)
//   - audio:  MP2 audio decoder (TODO)

pub mod audio;
pub mod buffer;
pub mod demux;
pub mod player;
pub mod video;

pub use audio::{Audio, Samples};
pub use demux::{demux_all, DemuxedStreams};
pub use player::VideoPlayer;
pub use video::{Frame, Plane, Video};
