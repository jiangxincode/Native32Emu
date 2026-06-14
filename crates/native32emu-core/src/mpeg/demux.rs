// MPEG-PS (Program Stream) demuxer.
//
// Ported to Rust from PL_MPEG (https://github.com/phoboslab/pl_mpeg) by
// Dominic Szablewski, originally MIT licensed. It splits an MPEG-1 program
// stream into its elementary video and audio streams.

use super::buffer::Buffer;

// Pack/system start codes.
const START_PACK: i32 = 0xBA;
const START_SYSTEM: i32 = 0xBB;

// Packet stream-id start codes.
pub const PACKET_PRIVATE: i32 = 0xBD;
pub const PACKET_AUDIO_1: i32 = 0xC0;
pub const PACKET_AUDIO_4: i32 = 0xC3;
pub const PACKET_VIDEO_1: i32 = 0xE0;

/// Sentinel for "no presentation timestamp".
pub const INVALID_TS: f64 = -1.0;

/// A demuxed PES packet: its stream id, optional PTS (seconds) and payload.
pub struct Packet {
    pub ty: i32,
    pub pts: f64,
    pub data: Vec<u8>,
}

/// The elementary streams extracted from a program stream.
#[derive(Default)]
pub struct DemuxedStreams {
    /// Concatenated video elementary stream (first video stream, 0xE0).
    pub video: Vec<u8>,
    /// Concatenated audio elementary stream (first audio stream, 0xC0).
    pub audio: Vec<u8>,
    pub num_video_streams: i32,
    pub num_audio_streams: i32,
}

/// Streaming demuxer state.
pub struct Demux {
    buffer: Buffer,
    start_code: i32,
    has_pack_header: bool,
    has_system_header: bool,
    has_headers: bool,
    num_audio_streams: i32,
    num_video_streams: i32,
    /// Length (bytes) of the packet currently positioned at the read head,
    /// which must be skipped before the next packet is decoded.
    current_length: usize,
    next_type: i32,
    next_length: usize,
    next_pts: f64,
}

impl Demux {
    pub fn new(data: Vec<u8>) -> Self {
        let mut s = Self {
            buffer: Buffer::new(data),
            start_code: -1,
            has_pack_header: false,
            has_system_header: false,
            has_headers: false,
            num_audio_streams: 0,
            num_video_streams: 0,
            current_length: 0,
            next_type: -1,
            next_length: 0,
            next_pts: INVALID_TS,
        };
        s.has_headers();
        s
    }

    pub fn num_video_streams(&self) -> i32 {
        self.num_video_streams
    }

    pub fn num_audio_streams(&self) -> i32 {
        self.num_audio_streams
    }

    /// Decode the pack and system headers. Returns true once both are parsed.
    pub fn has_headers(&mut self) -> bool {
        if self.has_headers {
            return true;
        }

        // Pack header.
        if !self.has_pack_header {
            if self.start_code != START_PACK && self.buffer.find_start_code(START_PACK) == -1 {
                return false;
            }
            self.start_code = START_PACK;
            if !self.buffer.has(64) {
                return false;
            }
            self.start_code = -1;
            if self.buffer.read(4) != 0x02 {
                return false;
            }
            self.decode_time(); // system clock reference
            self.buffer.skip(1);
            self.buffer.skip(22); // mux_rate * 50
            self.buffer.skip(1);
            self.has_pack_header = true;
        }

        // System header.
        if !self.has_system_header {
            if self.start_code != START_SYSTEM && self.buffer.find_start_code(START_SYSTEM) == -1 {
                return false;
            }
            self.start_code = START_SYSTEM;
            if !self.buffer.has(56) {
                return false;
            }
            self.start_code = -1;
            self.buffer.skip(16); // header_length
            self.buffer.skip(24); // rate bound
            self.num_audio_streams = self.buffer.read(6) as i32;
            self.buffer.skip(5); // misc flags
            self.num_video_streams = self.buffer.read(5) as i32;
            self.has_system_header = true;
        }

        self.has_headers = true;
        true
    }

    /// Decode a 33-bit MPEG timestamp into seconds.
    fn decode_time(&mut self) -> f64 {
        let mut clock = (self.buffer.read(3) as i64) << 30;
        self.buffer.skip(1);
        clock |= (self.buffer.read(15) as i64) << 15;
        self.buffer.skip(1);
        clock |= self.buffer.read(15) as i64;
        self.buffer.skip(1);
        clock as f64 / 90000.0
    }

    /// Decode the next packet of any audio/video/private stream.
    pub fn decode(&mut self) -> Option<Packet> {
        if !self.has_headers() {
            return None;
        }

        // Skip over the payload of the previously returned packet.
        if self.current_length > 0 {
            let bits = self.current_length << 3;
            if !self.buffer.has(bits) {
                return None;
            }
            self.buffer.skip(bits);
            self.current_length = 0;
        }

        // A header was already located on a previous call.
        if self.start_code != -1 {
            let code = self.start_code;
            return self.decode_packet(code);
        }

        loop {
            self.start_code = self.buffer.next_start_code();
            let sc = self.start_code;
            if sc == PACKET_VIDEO_1
                || sc == PACKET_PRIVATE
                || (PACKET_AUDIO_1..=PACKET_AUDIO_4).contains(&sc)
            {
                return self.decode_packet(sc);
            }
            if sc == -1 {
                return None;
            }
        }
    }

    fn decode_packet(&mut self, ty: i32) -> Option<Packet> {
        if !self.buffer.has(16 << 3) {
            return None;
        }
        self.start_code = -1;

        self.next_type = ty;
        let mut length = self.buffer.read(16) as i64;
        length -= self.buffer.skip_bytes(0xff) as i64; // stuffing

        // Skip P-STD buffer info if present.
        if self.buffer.read(2) == 0x01 {
            self.buffer.skip(16);
            length -= 2;
        }

        let pts_dts_marker = self.buffer.read(2);
        if pts_dts_marker == 0x03 {
            self.next_pts = self.decode_time();
            self.buffer.skip(40); // skip DTS
            length -= 10;
        } else if pts_dts_marker == 0x02 {
            self.next_pts = self.decode_time();
            length -= 5;
        } else if pts_dts_marker == 0x00 {
            self.next_pts = INVALID_TS;
            self.buffer.skip(4);
            length -= 1;
        } else {
            return None; // invalid
        }

        if length < 0 {
            return None;
        }
        self.next_length = length as usize;
        self.get_packet()
    }

    fn get_packet(&mut self) -> Option<Packet> {
        let bits = self.next_length << 3;
        if !self.buffer.has(bits) {
            return None;
        }
        let data = self.buffer.peek_bytes(self.next_length)?.to_vec();
        let packet = Packet {
            ty: self.next_type,
            pts: self.next_pts,
            data,
        };
        self.current_length = self.next_length;
        self.next_length = 0;
        Some(packet)
    }
}

/// Demux a whole in-memory program stream into its first video and audio
/// elementary streams.
pub fn demux_all(data: Vec<u8>) -> DemuxedStreams {
    let mut demux = Demux::new(data);
    let mut out = DemuxedStreams {
        num_video_streams: demux.num_video_streams(),
        num_audio_streams: demux.num_audio_streams(),
        ..Default::default()
    };

    while let Some(packet) = demux.decode() {
        if packet.ty == PACKET_VIDEO_1 {
            out.video.extend_from_slice(&packet.data);
        } else if packet.ty == PACKET_AUDIO_1 {
            out.audio.extend_from_slice(&packet.data);
        }
        // Other audio streams / private streams are ignored for now.
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal MPEG-PS: pack header, system header, one video PES and
    /// one audio PES, then an end code.
    fn build_stream() -> Vec<u8> {
        let mut d = Vec::new();

        // Pack header: 00 00 01 BA, then "0010" + SCR + mux_rate.
        d.extend_from_slice(&[0x00, 0x00, 0x01, 0xBA]);
        // 4 bits 0010 marker + 33-bit SCR + markers + 22-bit mux_rate + marker.
        // Use a known-good 8-byte pack body (all zero SCR, mux_rate arbitrary).
        // byte0: 0010 001 0 -> 0x21 (matches real files)
        d.extend_from_slice(&[0x21, 0x00, 0x01, 0x00, 0x01, 0x80, 0x80, 0x01]);

        // System header: 00 00 01 BB, length(16), then fields.
        d.extend_from_slice(&[0x00, 0x00, 0x01, 0xBB]);
        // header_length = 6
        d.extend_from_slice(&[0x00, 0x06]);
        d.extend_from_slice(&[0x80, 0x00, 0x00]); // rate bound (24)
                                                  // num_audio_streams (6 bits) = 1 -> 0b000001_xx; pack as 0x04 then flags
                                                  // byte: aaaaaa ff -> audio=1 (000001), 2 misc bits = 00 -> 0b00000100 = 0x04
        d.push(0x04);
        d.push(0x00); // remaining misc flags (3) + video streams start...
                      // video_streams (5 bits): place 1 in next byte top bits -> 0b00001_xxx
        d.push(0x08);

        // Video PES: 00 00 01 E0, length, no PTS, payload.
        let payload_v = [0xAAu8, 0xBB, 0xCC, 0xDD];
        d.extend_from_slice(&[0x00, 0x00, 0x01, 0xE0]);
        // length = 1 (marker byte) + payload
        let len_v = 1 + payload_v.len();
        d.push((len_v >> 8) as u8);
        d.push((len_v & 0xff) as u8);
        d.push(0x0f); // pts_dts marker bits 00 -> top 2 bits 0, lower bits arbitrary; read(2)=00
        d.extend_from_slice(&payload_v);

        // Audio PES: 00 00 01 C0, length, no PTS, payload.
        let payload_a = [0x11u8, 0x22, 0x33];
        d.extend_from_slice(&[0x00, 0x00, 0x01, 0xC0]);
        let len_a = 1 + payload_a.len();
        d.push((len_a >> 8) as u8);
        d.push((len_a & 0xff) as u8);
        d.push(0x0f);
        d.extend_from_slice(&payload_a);

        // End code.
        d.extend_from_slice(&[0x00, 0x00, 0x01, 0xB9]);
        // PL_MPEG needs 16 bytes of lookahead before decoding a packet, so pad
        // the tail (real files are large enough that this never matters).
        d.extend_from_slice(&[0xFF; 16]);
        d
    }

    #[test]
    fn test_demux_splits_streams() {
        let stream = build_stream();
        let out = demux_all(stream);
        assert_eq!(out.video, vec![0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(out.audio, vec![0x11, 0x22, 0x33]);
    }

    #[test]
    fn test_demux_real_file_header() {
        // A real MPEG-1 PS begins with 00 00 01 BA and an 0x21 marker byte.
        let data = vec![
            0x00, 0x00, 0x01, 0xBA, 0x21, 0x00, 0x01, 0x00, 0x01, 0x80, 0x80, 0x01,
        ];
        // Should at least parse the pack header without panicking.
        let mut demux = Demux::new(data);
        // No system header / packets present, so decode yields nothing.
        assert!(demux.decode().is_none());
    }
}
