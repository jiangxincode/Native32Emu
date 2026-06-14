// MPEG-1 Audio Layer II ("mp2") decoder.
//
// Ported to Rust from PL_MPEG (https://github.com/phoboslab/pl_mpeg) by Dominic
// Szablewski, originally MIT licensed; itself based on kjmp2 by Martin J.
// Fiedler. Decodes an MP2 elementary stream into normalized stereo float PCM.

use super::buffer::Buffer;

pub const SAMPLES_PER_FRAME: usize = 1152;

const FRAME_SYNC: u32 = 0x7ff;

const MPEG_1: u32 = 0x3;
const LAYER_II: u32 = 0x2;

const MODE_JOINT_STEREO: u32 = 0x1;
const MODE_MONO: u32 = 0x3;

const SAMPLE_RATE: [u32; 8] = [
    44100, 48000, 32000, 0, // MPEG-1
    22050, 24000, 16000, 0, // MPEG-2
];

const BIT_RATE: [i32; 28] = [
    32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, // MPEG-1
    8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, // MPEG-2
];

const SCALEFACTOR_BASE: [i32; 3] = [0x0200_0000, 0x0196_5FEA, 0x0142_8A30];

// Quantizer lookup, step 1: bitrate classes (mono / stereo).
const QUANT_LUT_STEP_1: [[u8; 16]; 2] = [
    [0, 0, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 0],
    [0, 0, 0, 0, 0, 0, 1, 1, 1, 2, 2, 2, 2, 2, 0, 0],
];

// Quantizer lookup, step 2: bitrate class, sample rate -> B2 table idx, sblimit.
const QUANT_TAB_A: u8 = 27 | 64; // sblimit 27
const QUANT_TAB_B: u8 = 30 | 64; // sblimit 30
const QUANT_TAB_C: u8 = 8; // sblimit 8
const QUANT_TAB_D: u8 = 12; // sblimit 12

const QUANT_LUT_STEP_2: [[u8; 3]; 3] = [
    [QUANT_TAB_C, QUANT_TAB_C, QUANT_TAB_D],
    [QUANT_TAB_A, QUANT_TAB_A, QUANT_TAB_A],
    [QUANT_TAB_B, QUANT_TAB_A, QUANT_TAB_B],
];

// Quantizer lookup, step 3: B2 table, subband -> nbal (upper 4 bits), row index
// (lower 4 bits).
const QUANT_LUT_STEP_3: [[u8; 32]; 3] = [
    // Low-rate table.
    [
        0x44, 0x44, 0x34, 0x34, 0x34, 0x34, 0x34, 0x34, 0x34, 0x34, 0x34, 0x34, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // High-rate table.
    [
        0x43, 0x43, 0x43, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x31, 0x31, 0x31, 0x31,
        0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0, 0,
    ],
    // MPEG-2 LSR table (unused for MPEG-1 but kept for fidelity).
    [
        0x45, 0x45, 0x45, 0x45, 0x34, 0x34, 0x34, 0x34, 0x34, 0x34, 0x34, 0x24, 0x24, 0x24, 0x24,
        0x24, 0x24, 0x24, 0x24, 0x24, 0x24, 0x24, 0x24, 0x24, 0x24, 0x24, 0x24, 0x24, 0x24, 0x24,
        0x24, 0,
    ],
];

// Quantizer lookup, step 4: table row, allocation value -> quant table index.
const QUANT_LUT_STEP_4: [[u8; 16]; 6] = [
    [0, 1, 2, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 4, 5, 6, 17, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 17],
    [0, 1, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17],
    [0, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 17],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
];

#[derive(Clone, Copy)]
struct QuantSpec {
    levels: u16,
    group: u8,
    bits: u8,
}

const QUANT_TAB: [QuantSpec; 17] = [
    QuantSpec {
        levels: 3,
        group: 1,
        bits: 5,
    },
    QuantSpec {
        levels: 5,
        group: 1,
        bits: 7,
    },
    QuantSpec {
        levels: 7,
        group: 0,
        bits: 3,
    },
    QuantSpec {
        levels: 9,
        group: 1,
        bits: 10,
    },
    QuantSpec {
        levels: 15,
        group: 0,
        bits: 4,
    },
    QuantSpec {
        levels: 31,
        group: 0,
        bits: 5,
    },
    QuantSpec {
        levels: 63,
        group: 0,
        bits: 6,
    },
    QuantSpec {
        levels: 127,
        group: 0,
        bits: 7,
    },
    QuantSpec {
        levels: 255,
        group: 0,
        bits: 8,
    },
    QuantSpec {
        levels: 511,
        group: 0,
        bits: 9,
    },
    QuantSpec {
        levels: 1023,
        group: 0,
        bits: 10,
    },
    QuantSpec {
        levels: 2047,
        group: 0,
        bits: 11,
    },
    QuantSpec {
        levels: 4095,
        group: 0,
        bits: 12,
    },
    QuantSpec {
        levels: 8191,
        group: 0,
        bits: 13,
    },
    QuantSpec {
        levels: 16383,
        group: 0,
        bits: 14,
    },
    QuantSpec {
        levels: 32767,
        group: 0,
        bits: 15,
    },
    QuantSpec {
        levels: 65535,
        group: 0,
        bits: 16,
    },
];

const SYNTHESIS_WINDOW: [f32; 512] = [
    0.0, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -1.0, -1.0, -1.0, -1.0, -1.5, -1.5, -2.0, -2.0, -2.5,
    -2.5, -3.0, -3.5, -3.5, -4.0, -4.5, -5.0, -5.5, -6.5, -7.0, -8.0, -8.5, -9.5, -10.5, -12.0,
    -13.0, -14.5, -15.5, -17.5, -19.0, -20.5, -22.5, -24.5, -26.5, -29.0, -31.5, -34.0, -36.5,
    -39.5, -42.5, -45.5, -48.5, -52.0, -55.5, -58.5, -62.5, -66.0, -69.5, -73.5, -77.0, -80.5,
    -84.5, -88.0, -91.5, -95.0, -98.0, -101.0, -104.0, 106.5, 109.0, 111.0, 112.5, 113.5, 114.0,
    114.0, 113.5, 112.0, 110.5, 107.5, 104.0, 100.0, 94.5, 88.5, 81.5, 73.0, 63.5, 53.0, 41.5,
    28.5, 14.5, -1.0, -18.0, -36.0, -55.5, -76.5, -98.5, -122.0, -147.0, -173.5, -200.5, -229.5,
    -259.5, -290.5, -322.5, -355.5, -389.5, -424.0, -459.5, -495.5, -532.0, -568.5, -605.0, -641.5,
    -678.0, -714.0, -749.0, -783.5, -817.0, -849.0, -879.5, -908.5, -935.0, -959.5, -981.0,
    -1000.5, -1016.0, -1028.5, -1037.5, -1042.5, -1043.5, -1040.0, -1031.5, 1018.5, 1000.0, 976.0,
    946.5, 911.0, 869.5, 822.0, 767.5, 707.0, 640.0, 565.5, 485.0, 397.0, 302.5, 201.0, 92.5,
    -22.5, -144.0, -272.5, -407.0, -547.5, -694.0, -846.0, -1003.0, -1165.0, -1331.5, -1502.0,
    -1675.5, -1852.5, -2031.5, -2212.5, -2394.0, -2576.5, -2758.5, -2939.5, -3118.5, -3294.5,
    -3467.5, -3635.5, -3798.5, -3955.0, -4104.5, -4245.5, -4377.5, -4499.0, -4609.5, -4708.0,
    -4792.5, -4863.5, -4919.0, -4958.0, -4979.5, -4983.0, -4967.5, -4931.5, -4875.0, -4796.0,
    -4694.5, -4569.5, -4420.0, -4246.0, -4046.0, -3820.0, -3567.0, 3287.0, 2979.5, 2644.0, 2280.5,
    1888.0, 1467.5, 1018.5, 541.0, 35.0, -499.0, -1061.0, -1650.0, -2266.5, -2909.0, -3577.0,
    -4270.0, -4987.5, -5727.5, -6490.0, -7274.0, -8077.5, -8899.5, -9739.0, -10594.5, -11464.5,
    -12347.0, -13241.0, -14144.5, -15056.0, -15973.5, -16895.5, -17820.0, -18744.5, -19668.0,
    -20588.0, -21503.0, -22410.5, -23308.5, -24195.0, -25068.5, -25926.5, -26767.0, -27589.0,
    -28389.0, -29166.5, -29919.0, -30644.5, -31342.0, -32009.5, -32645.0, -33247.0, -33814.5,
    -34346.0, -34839.5, -35295.0, -35710.0, -36084.5, -36417.5, -36707.5, -36954.0, -37156.5,
    -37315.0, -37428.0, -37496.0, 37519.0, 37496.0, 37428.0, 37315.0, 37156.5, 36954.0, 36707.5,
    36417.5, 36084.5, 35710.0, 35295.0, 34839.5, 34346.0, 33814.5, 33247.0, 32645.0, 32009.5,
    31342.0, 30644.5, 29919.0, 29166.5, 28389.0, 27589.0, 26767.0, 25926.5, 25068.5, 24195.0,
    23308.5, 22410.5, 21503.0, 20588.0, 19668.0, 18744.5, 17820.0, 16895.5, 15973.5, 15056.0,
    14144.5, 13241.0, 12347.0, 11464.5, 10594.5, 9739.0, 8899.5, 8077.5, 7274.0, 6490.0, 5727.5,
    4987.5, 4270.0, 3577.0, 2909.0, 2266.5, 1650.0, 1061.0, 499.0, -35.0, -541.0, -1018.5, -1467.5,
    -1888.0, -2280.5, -2644.0, -2979.5, 3287.0, 3567.0, 3820.0, 4046.0, 4246.0, 4420.0, 4569.5,
    4694.5, 4796.0, 4875.0, 4931.5, 4967.5, 4983.0, 4979.5, 4958.0, 4919.0, 4863.5, 4792.5, 4708.0,
    4609.5, 4499.0, 4377.5, 4245.5, 4104.5, 3955.0, 3798.5, 3635.5, 3467.5, 3294.5, 3118.5, 2939.5,
    2758.5, 2576.5, 2394.0, 2212.5, 2031.5, 1852.5, 1675.5, 1502.0, 1331.5, 1165.0, 1003.0, 846.0,
    694.0, 547.5, 407.0, 272.5, 144.0, 22.5, -92.5, -201.0, -302.5, -397.0, -485.0, -565.5, -640.0,
    -707.0, -767.5, -822.0, -869.5, -911.0, -946.5, -976.0, -1000.0, 1018.5, 1031.5, 1040.0,
    1043.5, 1042.5, 1037.5, 1028.5, 1016.0, 1000.5, 981.0, 959.5, 935.0, 908.5, 879.5, 849.0,
    817.0, 783.5, 749.0, 714.0, 678.0, 641.5, 605.0, 568.5, 532.0, 495.5, 459.5, 424.0, 389.5,
    355.5, 322.5, 290.5, 259.5, 229.5, 200.5, 173.5, 147.0, 122.0, 98.5, 76.5, 55.5, 36.0, 18.0,
    1.0, -14.5, -28.5, -41.5, -53.0, -63.5, -73.0, -81.5, -88.5, -94.5, -100.0, -104.0, -107.5,
    -110.5, -112.0, -113.5, -114.0, -114.0, -113.5, -112.5, -111.0, -109.0, 106.5, 104.0, 101.0,
    98.0, 95.0, 91.5, 88.0, 84.5, 80.5, 77.0, 73.5, 69.5, 66.0, 62.5, 58.5, 55.5, 52.0, 48.5, 45.5,
    42.5, 39.5, 36.5, 34.0, 31.5, 29.0, 26.5, 24.5, 22.5, 20.5, 19.0, 17.5, 15.5, 14.5, 13.0, 12.0,
    10.5, 9.5, 8.5, 8.0, 7.0, 6.5, 5.5, 5.0, 4.5, 4.0, 3.5, 3.5, 3.0, 2.5, 2.5, 2.0, 2.0, 1.5, 1.5,
    1.0, 1.0, 1.0, 1.0, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5,
];

/// A decoded MP2 frame: 1152 normalized stereo samples, interleaved L/R.
pub struct Samples {
    pub time: f64,
    pub interleaved: Vec<f32>,
}

/// MP2 audio decoder over an in-memory elementary stream.
pub struct Audio {
    buffer: Buffer,
    time: f64,
    samples_decoded: i64,
    samplerate_index: usize,
    bitrate_index: usize,
    version: u32,
    layer: u32,
    mode: u32,
    bound: usize,
    v_pos: i32,
    next_frame_data_size: usize,
    has_header: bool,

    allocation: [[Option<&'static QuantSpec>; 32]; 2],
    scale_factor_info: [[u8; 32]; 2],
    scale_factor: [[[i32; 3]; 32]; 2],
    sample: [[[i32; 3]; 32]; 2],

    interleaved: Vec<f32>,
    d: Vec<f32>,
    v: [Vec<f32>; 2],
    u: [f32; 32],
}

impl Audio {
    pub fn new(audio_es: Vec<u8>) -> Self {
        let mut d = vec![0.0f32; 1024];
        d[..512].copy_from_slice(&SYNTHESIS_WINDOW);
        d[512..].copy_from_slice(&SYNTHESIS_WINDOW);

        let mut s = Self {
            buffer: Buffer::new(audio_es),
            time: 0.0,
            samples_decoded: 0,
            samplerate_index: 3, // indicates 0
            bitrate_index: 0,
            version: 0,
            layer: 0,
            mode: 0,
            bound: 0,
            v_pos: 0,
            next_frame_data_size: 0,
            has_header: false,
            allocation: [[None; 32]; 2],
            scale_factor_info: [[0; 32]; 2],
            scale_factor: [[[0; 3]; 32]; 2],
            sample: [[[0; 3]; 32]; 2],
            interleaved: vec![0.0; SAMPLES_PER_FRAME * 2],
            d,
            v: [vec![0.0; 1024], vec![0.0; 1024]],
            u: [0.0; 32],
        };
        s.next_frame_data_size = s.decode_header();
        s
    }

    pub fn has_header(&mut self) -> bool {
        if self.has_header {
            return true;
        }
        self.next_frame_data_size = self.decode_header();
        self.has_header
    }

    pub fn samplerate(&self) -> u32 {
        if self.has_header {
            SAMPLE_RATE[self.samplerate_index]
        } else {
            0
        }
    }

    /// Decode the next MP2 frame, or `None` at end of stream.
    pub fn decode(&mut self) -> Option<Samples> {
        if self.next_frame_data_size == 0 {
            if !self.buffer.has(48) {
                return None;
            }
            self.next_frame_data_size = self.decode_header();
        }
        if self.next_frame_data_size == 0 || !self.buffer.has(self.next_frame_data_size << 3) {
            return None;
        }

        self.decode_frame();
        self.next_frame_data_size = 0;

        let time = self.time;
        self.samples_decoded += SAMPLES_PER_FRAME as i64;
        self.time = self.samples_decoded as f64 / SAMPLE_RATE[self.samplerate_index] as f64;

        Some(Samples {
            time,
            interleaved: self.interleaved.clone(),
        })
    }

    fn decode_header(&mut self) -> usize {
        if !self.buffer.has(48) {
            return 0;
        }
        self.buffer.skip_bytes(0x00);
        let sync = self.buffer.read(11);

        if sync != FRAME_SYNC && !self.buffer.find_frame_sync() {
            return 0;
        }

        self.version = self.buffer.read(2);
        self.layer = self.buffer.read(2);
        let has_crc = self.buffer.read(1) == 0;

        if self.version != MPEG_1 || self.layer != LAYER_II {
            return 0;
        }

        let bitrate_index = self.buffer.read(4) as i32 - 1;
        if !(0..=13).contains(&bitrate_index) {
            return 0;
        }
        let bitrate_index = bitrate_index as usize;

        let samplerate_index = self.buffer.read(2) as usize;
        if samplerate_index == 3 {
            return 0;
        }

        let padding = self.buffer.read(1) as usize;
        self.buffer.skip(1); // f_private
        let mode = self.buffer.read(2);

        // If we already have a header, the format must match or we lost sync.
        if self.has_header
            && (self.bitrate_index != bitrate_index
                || self.samplerate_index != samplerate_index
                || self.mode != mode)
        {
            return 0;
        }

        self.bitrate_index = bitrate_index;
        self.samplerate_index = samplerate_index;
        self.mode = mode;
        self.has_header = true;

        if mode == MODE_JOINT_STEREO {
            self.bound = ((self.buffer.read(2) + 1) << 2) as usize;
        } else {
            self.buffer.skip(2);
            self.bound = if mode == MODE_MONO { 0 } else { 32 };
        }

        // Discard copyright/original/emphasis and the optional CRC.
        self.buffer.skip(4);
        if has_crc {
            self.buffer.skip(16);
        }

        let bitrate = BIT_RATE[self.bitrate_index];
        let samplerate = SAMPLE_RATE[self.samplerate_index] as i32;
        let frame_size = (144000 * bitrate / samplerate) as usize + padding;
        frame_size - if has_crc { 6 } else { 4 }
    }

    fn read_allocation(&mut self, sb: usize, tab3: usize) -> Option<&'static QuantSpec> {
        let tab4 = QUANT_LUT_STEP_3[tab3][sb];
        let nbal = (tab4 >> 4) as usize;
        let row = (tab4 & 15) as usize;
        let qtab = QUANT_LUT_STEP_4[row][self.buffer.read(nbal) as usize];
        if qtab != 0 {
            Some(&QUANT_TAB[(qtab - 1) as usize])
        } else {
            None
        }
    }

    fn read_samples(&mut self, ch: usize, sb: usize, part: usize) {
        let q = match self.allocation[ch][sb] {
            Some(q) => q,
            None => {
                self.sample[ch][sb] = [0; 3];
                return;
            }
        };

        // Resolve scalefactor.
        let mut sf = self.scale_factor[ch][sb][part];
        if sf == 63 {
            sf = 0;
        } else {
            let shift = sf / 3;
            sf = (SCALEFACTOR_BASE[(sf % 3) as usize] + ((1 << shift) >> 1)) >> shift;
        }

        // Decode samples.
        let mut adj = q.levels as i32;
        if q.group != 0 {
            let mut val = self.buffer.read(q.bits as usize) as i32;
            self.sample[ch][sb][0] = val % adj;
            val /= adj;
            self.sample[ch][sb][1] = val % adj;
            self.sample[ch][sb][2] = val / adj;
        } else {
            for i in 0..3 {
                self.sample[ch][sb][i] = self.buffer.read(q.bits as usize) as i32;
            }
        }

        // Postmultiply samples.
        let scale = 65536 / (adj + 1);
        adj = ((adj + 1) >> 1) - 1;
        for i in 0..3 {
            let val = (adj - self.sample[ch][sb][i]) * scale;
            self.sample[ch][sb][i] = (val * (sf >> 12) + ((val * (sf & 4095) + 2048) >> 12)) >> 12;
        }
    }

    fn decode_frame(&mut self) {
        let tab1 = if self.mode == MODE_MONO { 0 } else { 1 };
        let tab2 = QUANT_LUT_STEP_1[tab1][self.bitrate_index] as usize;
        let mut tab3 = QUANT_LUT_STEP_2[tab2][self.samplerate_index] as usize;
        let sblimit = tab3 & 63;
        tab3 >>= 6;

        if self.bound > sblimit {
            self.bound = sblimit;
        }
        let bound = self.bound;

        // Allocation information.
        for sb in 0..bound {
            self.allocation[0][sb] = self.read_allocation(sb, tab3);
            self.allocation[1][sb] = self.read_allocation(sb, tab3);
        }
        for sb in bound..sblimit {
            let a = self.read_allocation(sb, tab3);
            self.allocation[0][sb] = a;
            self.allocation[1][sb] = a;
        }

        // Scale factor selector information.
        let channels = if self.mode == MODE_MONO { 1 } else { 2 };
        for sb in 0..sblimit {
            for ch in 0..channels {
                if self.allocation[ch][sb].is_some() {
                    self.scale_factor_info[ch][sb] = self.buffer.read(2) as u8;
                }
            }
            if self.mode == MODE_MONO {
                self.scale_factor_info[1][sb] = self.scale_factor_info[0][sb];
            }
        }

        // Scale factors.
        for sb in 0..sblimit {
            for ch in 0..channels {
                if self.allocation[ch][sb].is_some() {
                    let info = self.scale_factor_info[ch][sb];
                    let sf = &mut self.scale_factor[ch][sb];
                    match info {
                        0 => {
                            sf[0] = self.buffer.read(6) as i32;
                            sf[1] = self.buffer.read(6) as i32;
                            sf[2] = self.buffer.read(6) as i32;
                        }
                        1 => {
                            let a = self.buffer.read(6) as i32;
                            sf[0] = a;
                            sf[1] = a;
                            sf[2] = self.buffer.read(6) as i32;
                        }
                        2 => {
                            let a = self.buffer.read(6) as i32;
                            sf[0] = a;
                            sf[1] = a;
                            sf[2] = a;
                        }
                        _ => {
                            sf[0] = self.buffer.read(6) as i32;
                            let a = self.buffer.read(6) as i32;
                            sf[1] = a;
                            sf[2] = a;
                        }
                    }
                }
            }
            if self.mode == MODE_MONO {
                self.scale_factor[1][sb] = self.scale_factor[0][sb];
            }
        }

        // Coefficient input and reconstruction.
        let mut out_pos = 0usize;
        for part in 0..3 {
            for _granule in 0..4 {
                for sb in 0..bound {
                    self.read_samples(0, sb, part);
                    self.read_samples(1, sb, part);
                }
                for sb in bound..sblimit {
                    self.read_samples(0, sb, part);
                    self.sample[1][sb] = self.sample[0][sb];
                }
                for sb in sblimit..32 {
                    self.sample[0][sb] = [0; 3];
                    self.sample[1][sb] = [0; 3];
                }

                // Synthesis.
                for p in 0..3 {
                    self.v_pos = (self.v_pos - 64) & 1023;

                    for ch in 0..2 {
                        idct36(&self.sample[ch], p, &mut self.v[ch], self.v_pos as usize);

                        self.u = [0.0; 32];
                        let mut d_index = 512 - (self.v_pos >> 1);
                        let mut v_index = (self.v_pos % 128) >> 1;
                        while v_index < 1024 {
                            for i in 0..32 {
                                self.u[i] +=
                                    self.d[d_index as usize] * self.v[ch][v_index as usize];
                                d_index += 1;
                                v_index += 1;
                            }
                            v_index += 128 - 32;
                            d_index += 64 - 32;
                        }

                        d_index -= 512 - 32;
                        v_index = (128 - 32 + 1024) - v_index;
                        while v_index < 1024 {
                            for i in 0..32 {
                                self.u[i] +=
                                    self.d[d_index as usize] * self.v[ch][v_index as usize];
                                d_index += 1;
                                v_index += 1;
                            }
                            v_index += 128 - 32;
                            d_index += 64 - 32;
                        }

                        for j in 0..32 {
                            self.interleaved[((out_pos + j) << 1) + ch] =
                                self.u[j] / -1_090_519_040.0f32;
                        }
                    }
                    out_pos += 32;
                }
            }
        }

        self.buffer.align();
    }
}

#[allow(clippy::needless_range_loop)]
#[allow(clippy::excessive_precision)]
#[allow(clippy::approx_constant)]
fn idct36(s: &[[i32; 3]; 32], ss: usize, d: &mut [f32], dp: usize) {
    let g = |i: usize| s[i][ss] as f32;

    let mut t01 = g(0) + g(31);
    let mut t02 = (g(0) - g(31)) * 0.500602998235;
    let mut t03 = g(1) + g(30);
    let mut t04 = (g(1) - g(30)) * 0.505470959898;
    let mut t05 = g(2) + g(29);
    let mut t06 = (g(2) - g(29)) * 0.515447309923;
    let mut t07 = g(3) + g(28);
    let mut t08 = (g(3) - g(28)) * 0.53104259109;
    let mut t09 = g(4) + g(27);
    let mut t10 = (g(4) - g(27)) * 0.553103896034;
    let mut t11 = g(5) + g(26);
    let mut t12 = (g(5) - g(26)) * 0.582934968206;
    let mut t13 = g(6) + g(25);
    let mut t14 = (g(6) - g(25)) * 0.622504123036;
    let mut t15 = g(7) + g(24);
    let mut t16 = (g(7) - g(24)) * 0.674808341455;
    let mut t17 = g(8) + g(23);
    let mut t18 = (g(8) - g(23)) * 0.744536271002;
    let mut t19 = g(9) + g(22);
    let mut t20 = (g(9) - g(22)) * 0.839349645416;
    let mut t21 = g(10) + g(21);
    let mut t22 = (g(10) - g(21)) * 0.972568237862;
    let mut t23 = g(11) + g(20);
    let mut t24 = (g(11) - g(20)) * 1.16943993343;
    let mut t25 = g(12) + g(19);
    let mut t26 = (g(12) - g(19)) * 1.48416461631;
    let mut t27 = g(13) + g(18);
    let mut t28 = (g(13) - g(18)) * 2.05778100995;
    let mut t29 = g(14) + g(17);
    let mut t30 = (g(14) - g(17)) * 3.40760841847;
    let mut t31 = g(15) + g(16);
    let mut t32 = (g(15) - g(16)) * 10.1900081235;

    let mut t33;

    t33 = t01 + t31;
    t31 = (t01 - t31) * 0.502419286188;
    t01 = t03 + t29;
    t29 = (t03 - t29) * 0.52249861494;
    t03 = t05 + t27;
    t27 = (t05 - t27) * 0.566944034816;
    t05 = t07 + t25;
    t25 = (t07 - t25) * 0.64682178336;
    t07 = t09 + t23;
    t23 = (t09 - t23) * 0.788154623451;
    t09 = t11 + t21;
    t21 = (t11 - t21) * 1.06067768599;
    t11 = t13 + t19;
    t19 = (t13 - t19) * 1.72244709824;
    t13 = t15 + t17;
    t17 = (t15 - t17) * 5.10114861869;

    t15 = t33 + t13;
    t13 = (t33 - t13) * 0.509795579104;
    t33 = t01 + t11;
    t01 = (t01 - t11) * 0.601344886935;
    t11 = t03 + t09;
    t09 = (t03 - t09) * 0.899976223136;
    t03 = t05 + t07;
    t07 = (t05 - t07) * 2.56291544774;
    t05 = t15 + t03;
    t15 = (t15 - t03) * 0.541196100146;
    t03 = t33 + t11;
    t11 = (t33 - t11) * 1.30656296488;
    t33 = t05 + t03;
    t05 = (t05 - t03) * 0.707106781187;
    t03 = t15 + t11;
    t15 = (t15 - t11) * 0.707106781187;
    t03 += t15;
    t11 = t13 + t07;
    t13 = (t13 - t07) * 0.541196100146;
    t07 = t01 + t09;
    t09 = (t01 - t09) * 1.30656296488;
    t01 = t11 + t07;
    t07 = (t11 - t07) * 0.707106781187;
    t11 = t13 + t09;
    t13 = (t13 - t09) * 0.707106781187;
    t11 += t13;
    t01 += t11;
    t11 += t07;
    t07 += t13;
    t09 = t31 + t17;
    t31 = (t31 - t17) * 0.509795579104;
    t17 = t29 + t19;
    t29 = (t29 - t19) * 0.601344886935;
    t19 = t27 + t21;
    t21 = (t27 - t21) * 0.899976223136;
    t27 = t25 + t23;
    t23 = (t25 - t23) * 2.56291544774;
    t25 = t09 + t27;
    t09 = (t09 - t27) * 0.541196100146;
    t27 = t17 + t19;
    t19 = (t17 - t19) * 1.30656296488;
    t17 = t25 + t27;
    t27 = (t25 - t27) * 0.707106781187;
    t25 = t09 + t19;
    t19 = (t09 - t19) * 0.707106781187;
    t25 += t19;
    t09 = t31 + t23;
    t31 = (t31 - t23) * 0.541196100146;
    t23 = t29 + t21;
    t21 = (t29 - t21) * 1.30656296488;
    t29 = t09 + t23;
    t23 = (t09 - t23) * 0.707106781187;
    t09 = t31 + t21;
    t31 = (t31 - t21) * 0.707106781187;
    t09 += t31;
    t29 += t09;
    t09 += t23;
    t23 += t31;
    t17 += t29;
    t29 += t25;
    t25 += t09;
    t09 += t27;
    t27 += t23;
    t23 += t19;
    t19 += t31;

    t21 = t02 + t32;
    t02 = (t02 - t32) * 0.500602998235;
    t32 = t04 + t30;
    t04 = (t04 - t30) * 0.52249861494;
    t30 = t06 + t28;
    t28 = (t06 - t28) * 0.566944034816;
    t06 = t08 + t26;
    t08 = (t08 - t26) * 0.64682178336;
    t26 = t10 + t24;
    t10 = (t10 - t24) * 0.788154623451;
    t24 = t12 + t22;
    t22 = (t12 - t22) * 1.06067768599;
    t12 = t14 + t20;
    t20 = (t14 - t20) * 1.72244709824;
    t14 = t16 + t18;
    t16 = (t16 - t18) * 5.10114861869;

    t18 = t21 + t14;
    t14 = (t21 - t14) * 0.509795579104;
    t21 = t32 + t12;
    t32 = (t32 - t12) * 0.601344886935;
    t12 = t30 + t24;
    t24 = (t30 - t24) * 0.899976223136;
    t30 = t06 + t26;
    t26 = (t06 - t26) * 2.56291544774;
    t06 = t18 + t30;
    t18 = (t18 - t30) * 0.541196100146;
    t30 = t21 + t12;
    t12 = (t21 - t12) * 1.30656296488;
    t21 = t06 + t30;
    t30 = (t06 - t30) * 0.707106781187;
    t06 = t18 + t12;
    t12 = (t18 - t12) * 0.707106781187;
    t06 += t12;
    t18 = t14 + t26;
    t26 = (t14 - t26) * 0.541196100146;
    t14 = t32 + t24;
    t24 = (t32 - t24) * 1.30656296488;
    t32 = t18 + t14;
    t14 = (t18 - t14) * 0.707106781187;
    t18 = t26 + t24;
    t24 = (t26 - t24) * 0.707106781187;
    t18 += t24;
    t32 += t18;
    t18 += t14;
    t26 = t14 + t24;
    t14 = t02 + t16;
    t02 = (t02 - t16) * 0.509795579104;
    t16 = t04 + t20;
    t04 = (t04 - t20) * 0.601344886935;
    t20 = t28 + t22;
    t22 = (t28 - t22) * 0.899976223136;
    t28 = t08 + t10;
    t10 = (t08 - t10) * 2.56291544774;
    t08 = t14 + t28;
    t14 = (t14 - t28) * 0.541196100146;
    t28 = t16 + t20;
    t20 = (t16 - t20) * 1.30656296488;
    t16 = t08 + t28;
    t28 = (t08 - t28) * 0.707106781187;
    t08 = t14 + t20;
    t20 = (t14 - t20) * 0.707106781187;
    t08 += t20;
    t14 = t02 + t10;
    t02 = (t02 - t10) * 0.541196100146;
    t10 = t04 + t22;
    t22 = (t04 - t22) * 1.30656296488;
    t04 = t14 + t10;
    t10 = (t14 - t10) * 0.707106781187;
    t14 = t02 + t22;
    t02 = (t02 - t22) * 0.707106781187;
    t14 += t02;
    t04 += t14;
    t14 += t10;
    t10 += t02;
    t16 += t04;
    t04 += t08;
    t08 += t14;
    t14 += t28;
    t28 += t10;
    t10 += t20;
    t20 += t02;
    t21 += t16;
    t16 += t32;
    t32 += t04;
    t04 += t06;
    t06 += t08;
    t08 += t18;
    t18 += t14;
    t14 += t30;
    t30 += t28;
    t28 += t26;
    t26 += t10;
    t10 += t12;
    t12 += t20;
    t20 += t24;
    t24 += t02;

    d[dp + 48] = -t33;
    d[dp + 49] = -t21;
    d[dp + 47] = -t21;
    d[dp + 50] = -t17;
    d[dp + 46] = -t17;
    d[dp + 51] = -t16;
    d[dp + 45] = -t16;
    d[dp + 52] = -t01;
    d[dp + 44] = -t01;
    d[dp + 53] = -t32;
    d[dp + 43] = -t32;
    d[dp + 54] = -t29;
    d[dp + 42] = -t29;
    d[dp + 55] = -t04;
    d[dp + 41] = -t04;
    d[dp + 56] = -t03;
    d[dp + 40] = -t03;
    d[dp + 57] = -t06;
    d[dp + 39] = -t06;
    d[dp + 58] = -t25;
    d[dp + 38] = -t25;
    d[dp + 59] = -t08;
    d[dp + 37] = -t08;
    d[dp + 60] = -t11;
    d[dp + 36] = -t11;
    d[dp + 61] = -t18;
    d[dp + 35] = -t18;
    d[dp + 62] = -t09;
    d[dp + 34] = -t09;
    d[dp + 63] = -t14;
    d[dp + 33] = -t14;
    d[dp + 32] = -t05;
    d[dp] = t05;
    d[dp + 31] = -t30;
    d[dp + 1] = t30;
    d[dp + 30] = -t27;
    d[dp + 2] = t27;
    d[dp + 29] = -t28;
    d[dp + 3] = t28;
    d[dp + 28] = -t07;
    d[dp + 4] = t07;
    d[dp + 27] = -t26;
    d[dp + 5] = t26;
    d[dp + 26] = -t23;
    d[dp + 6] = t23;
    d[dp + 25] = -t10;
    d[dp + 7] = t10;
    d[dp + 24] = -t15;
    d[dp + 8] = t15;
    d[dp + 23] = -t12;
    d[dp + 9] = t12;
    d[dp + 22] = -t19;
    d[dp + 10] = t19;
    d[dp + 21] = -t20;
    d[dp + 11] = t20;
    d[dp + 20] = -t13;
    d[dp + 12] = t13;
    d[dp + 19] = -t24;
    d[dp + 13] = t24;
    d[dp + 18] = -t31;
    d[dp + 14] = t31;
    d[dp + 17] = -t02;
    d[dp + 15] = t02;
    d[dp + 16] = 0.0;
}
