use nihav_core::io::bitreader::*;
use nihav_core::io::codebook::*;
use nihav_core::formats;
use nihav_core::codecs::*;

static INDEO2_DELTA_TABLE: [[u8; 256]; 4] = [
    [
      0x80, 0x80, 0x84, 0x84, 0x7C, 0x7C, 0x7F, 0x85,
      0x81, 0x7B, 0x85, 0x7F, 0x7B, 0x81, 0x8C, 0x8C,
      0x74, 0x74, 0x83, 0x8D, 0x7D, 0x73, 0x8D, 0x83,
      0x73, 0x7D, 0x77, 0x89, 0x89, 0x77, 0x89, 0x77,
      0x77, 0x89, 0x8C, 0x95, 0x74, 0x6B, 0x95, 0x8C,
      0x6B, 0x74, 0x7C, 0x90, 0x84, 0x70, 0x90, 0x7C,
      0x70, 0x84, 0x96, 0x96, 0x6A, 0x6A, 0x82, 0x98,
      0x7E, 0x68, 0x98, 0x82, 0x68, 0x7E, 0x97, 0xA2,
      0x69, 0x5E, 0xA2, 0x97, 0x5E, 0x69, 0xA2, 0xA2,
      0x5E, 0x5E, 0x8B, 0xA3, 0x75, 0x5D, 0xA3, 0x8B,
      0x5D, 0x75, 0x71, 0x95, 0x8F, 0x6B, 0x95, 0x71,
      0x6B, 0x8F, 0x78, 0x9D, 0x88, 0x63, 0x9D, 0x78,
      0x63, 0x88, 0x7F, 0xA7, 0x81, 0x59, 0xA7, 0x7F,
      0x59, 0x81, 0xA4, 0xB1, 0x5C, 0x4F, 0xB1, 0xA4,
      0x4F, 0x5C, 0x96, 0xB1, 0x6A, 0x4F, 0xB1, 0x96,
      0x4F, 0x6A, 0xB2, 0xB2, 0x4E, 0x4E, 0x65, 0x9B,
      0x9B, 0x65, 0x9B, 0x65, 0x65, 0x9B, 0x89, 0xB4,
      0x77, 0x4C, 0xB4, 0x89, 0x4C, 0x77, 0x6A, 0xA3,
      0x96, 0x5D, 0xA3, 0x6A, 0x5D, 0x96, 0x73, 0xAC,
      0x8D, 0x54, 0xAC, 0x73, 0x54, 0x8D, 0xB4, 0xC3,
      0x4C, 0x3D, 0xC3, 0xB4, 0x3D, 0x4C, 0xA4, 0xC3,
      0x5C, 0x3D, 0xC3, 0xA4, 0x3D, 0x5C, 0xC4, 0xC4,
      0x3C, 0x3C, 0x96, 0xC6, 0x6A, 0x3A, 0xC6, 0x96,
      0x3A, 0x6A, 0x7C, 0xBA, 0x84, 0x46, 0xBA, 0x7C,
      0x46, 0x84, 0x5B, 0xAB, 0xA5, 0x55, 0xAB, 0x5B,
      0x55, 0xA5, 0x63, 0xB4, 0x9D, 0x4C, 0xB4, 0x63,
      0x4C, 0x9D, 0x86, 0xCA, 0x7A, 0x36, 0xCA, 0x86,
      0x36, 0x7A, 0xB6, 0xD7, 0x4A, 0x29, 0xD7, 0xB6,
      0x29, 0x4A, 0xC8, 0xD7, 0x38, 0x29, 0xD7, 0xC8,
      0x29, 0x38, 0xA4, 0xD8, 0x5C, 0x28, 0xD8, 0xA4,
      0x28, 0x5C, 0x6C, 0xC1, 0x94, 0x3F, 0xC1, 0x6C,
      0x3F, 0x94, 0xD9, 0xD9, 0x27, 0x27, 0x80, 0x80,
   ], [
      0x80, 0x80, 0x85, 0x85, 0x7B, 0x7B, 0x7E, 0x87,
      0x82, 0x79, 0x87, 0x7E, 0x79, 0x82, 0x8F, 0x8F,
      0x71, 0x71, 0x84, 0x8F, 0x7C, 0x71, 0x8F, 0x84,
      0x71, 0x7C, 0x75, 0x8B, 0x8B, 0x75, 0x8B, 0x75,
      0x75, 0x8B, 0x8E, 0x9A, 0x72, 0x66, 0x9A, 0x8E,
      0x66, 0x72, 0x7B, 0x93, 0x85, 0x6D, 0x93, 0x7B,
      0x6D, 0x85, 0x9B, 0x9B, 0x65, 0x65, 0x82, 0x9D,
      0x7E, 0x63, 0x9D, 0x82, 0x63, 0x7E, 0x9B, 0xA8,
      0x65, 0x58, 0xA8, 0x9B, 0x58, 0x65, 0xA9, 0xA9,
      0x57, 0x57, 0x8D, 0xAA, 0x73, 0x56, 0xAA, 0x8D,
      0x56, 0x73, 0x6E, 0x99, 0x92, 0x67, 0x99, 0x6E,
      0x67, 0x92, 0x76, 0xA2, 0x8A, 0x5E, 0xA2, 0x76,
      0x5E, 0x8A, 0x7F, 0xAF, 0x81, 0x51, 0xAF, 0x7F,
      0x51, 0x81, 0xAB, 0xBA, 0x55, 0x46, 0xBA, 0xAB,
      0x46, 0x55, 0x9A, 0xBB, 0x66, 0x45, 0xBB, 0x9A,
      0x45, 0x66, 0xBB, 0xBB, 0x45, 0x45, 0x60, 0xA0,
      0xA0, 0x60, 0xA0, 0x60, 0x60, 0xA0, 0x8B, 0xBE,
      0x75, 0x42, 0xBE, 0x8B, 0x42, 0x75, 0x66, 0xAA,
      0x9A, 0x56, 0xAA, 0x66, 0x56, 0x9A, 0x70, 0xB5,
      0x90, 0x4B, 0xB5, 0x70, 0x4B, 0x90, 0xBE, 0xCF,
      0x42, 0x31, 0xCF, 0xBE, 0x31, 0x42, 0xAB, 0xD0,
      0x55, 0x30, 0xD0, 0xAB, 0x30, 0x55, 0xD1, 0xD1,
      0x2F, 0x2F, 0x9A, 0xD3, 0x66, 0x2D, 0xD3, 0x9A,
      0x2D, 0x66, 0x7B, 0xC5, 0x85, 0x3B, 0xC5, 0x7B,
      0x3B, 0x85, 0x54, 0xB4, 0xAC, 0x4C, 0xB4, 0x54,
      0x4C, 0xAC, 0x5E, 0xBE, 0xA2, 0x42, 0xBE, 0x5E,
      0x42, 0xA2, 0x87, 0xD8, 0x79, 0x28, 0xD8, 0x87,
      0x28, 0x79, 0xC0, 0xE8, 0x40, 0x18, 0xE8, 0xC0,
      0x18, 0x40, 0xD5, 0xE8, 0x2B, 0x18, 0xE8, 0xD5,
      0x18, 0x2B, 0xAB, 0xE9, 0x55, 0x17, 0xE9, 0xAB,
      0x17, 0x55, 0x68, 0xCD, 0x98, 0x33, 0xCD, 0x68,
      0x33, 0x98, 0xEA, 0xEA, 0x16, 0x16, 0x80, 0x80,
    ], [
      0x80, 0x80, 0x86, 0x86, 0x7A, 0x7A, 0x7E, 0x88,
      0x82, 0x78, 0x88, 0x7E, 0x78, 0x82, 0x92, 0x92,
      0x6E, 0x6E, 0x85, 0x92, 0x7B, 0x6E, 0x92, 0x85,
      0x6E, 0x7B, 0x73, 0x8D, 0x8D, 0x73, 0x8D, 0x73,
      0x73, 0x8D, 0x91, 0x9E, 0x6F, 0x62, 0x9E, 0x91,
      0x62, 0x6F, 0x79, 0x97, 0x87, 0x69, 0x97, 0x79,
      0x69, 0x87, 0xA0, 0xA0, 0x60, 0x60, 0x83, 0xA2,
      0x7D, 0x5E, 0xA2, 0x83, 0x5E, 0x7D, 0xA0, 0xB0,
      0x60, 0x50, 0xB0, 0xA0, 0x50, 0x60, 0xB1, 0xB1,
      0x4F, 0x4F, 0x8F, 0xB2, 0x71, 0x4E, 0xB2, 0x8F,
      0x4E, 0x71, 0x6B, 0x9E, 0x95, 0x62, 0x9E, 0x6B,
      0x62, 0x95, 0x74, 0xA9, 0x8C, 0x57, 0xA9, 0x74,
      0x57, 0x8C, 0x7F, 0xB8, 0x81, 0x48, 0xB8, 0x7F,
      0x48, 0x81, 0xB4, 0xC5, 0x4C, 0x3B, 0xC5, 0xB4,
      0x3B, 0x4C, 0x9F, 0xC6, 0x61, 0x3A, 0xC6, 0x9F,
      0x3A, 0x61, 0xC6, 0xC6, 0x3A, 0x3A, 0x59, 0xA7,
      0xA7, 0x59, 0xA7, 0x59, 0x59, 0xA7, 0x8D, 0xCA,
      0x73, 0x36, 0xCA, 0x8D, 0x36, 0x73, 0x61, 0xB2,
      0x9F, 0x4E, 0xB2, 0x61, 0x4E, 0x9F, 0x6D, 0xBF,
      0x93, 0x41, 0xBF, 0x6D, 0x41, 0x93, 0xCA, 0xDF,
      0x36, 0x21, 0xDF, 0xCA, 0x21, 0x36, 0xB3, 0xDF,
      0x4D, 0x21, 0xDF, 0xB3, 0x21, 0x4D, 0xE1, 0xE1,
      0x1F, 0x1F, 0x9F, 0xE3, 0x61, 0x1D, 0xE3, 0x9F,
      0x1D, 0x61, 0x7A, 0xD3, 0x86, 0x2D, 0xD3, 0x7A,
      0x2D, 0x86, 0x4C, 0xBE, 0xB4, 0x42, 0xBE, 0x4C,
      0x42, 0xB4, 0x57, 0xCA, 0xA9, 0x36, 0xCA, 0x57,
      0x36, 0xA9, 0x88, 0xE9, 0x78, 0x17, 0xE9, 0x88,
      0x17, 0x78, 0xCC, 0xFB, 0x34, 0x05, 0xFB, 0xCC,
      0x05, 0x34, 0xE6, 0xFB, 0x1A, 0x05, 0xFB, 0xE6,
      0x05, 0x1A, 0xB4, 0xFD, 0x4C, 0x03, 0xFD, 0xB4,
      0x03, 0x4C, 0x63, 0xDC, 0x9D, 0x24, 0xDC, 0x63,
      0x24, 0x9D, 0xFE, 0xFE, 0x02, 0x02, 0x80, 0x80,
    ], [
      0x80, 0x80, 0x87, 0x87, 0x79, 0x79, 0x7E, 0x89,
      0x82, 0x77, 0x89, 0x7E, 0x77, 0x82, 0x95, 0x95,
      0x6B, 0x6B, 0x86, 0x96, 0x7A, 0x6A, 0x96, 0x86,
      0x6A, 0x7A, 0x70, 0x90, 0x90, 0x70, 0x90, 0x70,
      0x70, 0x90, 0x94, 0xA4, 0x6C, 0x5C, 0xA4, 0x94,
      0x5C, 0x6C, 0x78, 0x9B, 0x88, 0x65, 0x9B, 0x78,
      0x65, 0x88, 0xA6, 0xA6, 0x5A, 0x5A, 0x83, 0xA9,
      0x7D, 0x57, 0xA9, 0x83, 0x57, 0x7D, 0xA6, 0xB9,
      0x5A, 0x47, 0xB9, 0xA6, 0x47, 0x5A, 0xBA, 0xBA,
      0x46, 0x46, 0x92, 0xBC, 0x6E, 0x44, 0xBC, 0x92,
      0x44, 0x6E, 0x67, 0xA3, 0x99, 0x5D, 0xA3, 0x67,
      0x5D, 0x99, 0x72, 0xB0, 0x8E, 0x50, 0xB0, 0x72,
      0x50, 0x8E, 0x7F, 0xC3, 0x81, 0x3D, 0xC3, 0x7F,
      0x3D, 0x81, 0xBE, 0xD2, 0x42, 0x2E, 0xD2, 0xBE,
      0x2E, 0x42, 0xA5, 0xD4, 0x5B, 0x2C, 0xD4, 0xA5,
      0x2C, 0x5B, 0xD4, 0xD4, 0x2C, 0x2C, 0x52, 0xAE,
      0xAE, 0x52, 0xAE, 0x52, 0x52, 0xAE, 0x8F, 0xD8,
      0x71, 0x28, 0xD8, 0x8F, 0x28, 0x71, 0x5B, 0xBB,
      0xA5, 0x45, 0xBB, 0x5B, 0x45, 0xA5, 0x69, 0xCB,
      0x97, 0x35, 0xCB, 0x69, 0x35, 0x97, 0xD8, 0xF0,
      0x28, 0x10, 0xF0, 0xD8, 0x10, 0x28, 0xBD, 0xF1,
      0x43, 0x0F, 0xF1, 0xBD, 0x0F, 0x43, 0xF3, 0xF3,
      0x0D, 0x0D, 0xA5, 0xF6, 0x5B, 0x0A, 0xF6, 0xA5,
      0x0A, 0x5B, 0x78, 0xE2, 0x88, 0x1E, 0xE2, 0x78,
      0x1E, 0x88, 0x42, 0xC9, 0xBE, 0x37, 0xC9, 0x42,
      0x37, 0xBE, 0x4F, 0xD8, 0xB1, 0x28, 0xD8, 0x4F,
      0x28, 0xB1, 0x8A, 0xFD, 0x76, 0x03, 0xFD, 0x8A,
      0x03, 0x76, 0xDB, 0xFF, 0x25, 0x01, 0xFF, 0xDB,
      0x01, 0x25, 0xF9, 0xFF, 0x07, 0x01, 0xFF, 0xF9,
      0x01, 0x07, 0xBE, 0xFF, 0x42, 0x01, 0xFF, 0xBE,
      0x01, 0x42, 0x5E, 0xED, 0xA2, 0x13, 0xED, 0x5E,
      0x13, 0xA2, 0xFF, 0xFF, 0x01, 0x01, 0x80, 0x80,
    ]
];

static INDEO2_CODE_CODES: &[u16] = &[
    0x0000, 0x0004, 0x0006, 0x0001, 0x0009, 0x0019, 0x000D, 0x001D,
    0x0023, 0x0013, 0x0033, 0x000B, 0x002B, 0x001B, 0x0007, 0x0087,
    0x0027, 0x00A7, 0x0067, 0x00E7, 0x0097, 0x0057, 0x0037, 0x00B7,
    0x00F7, 0x000F, 0x008F, 0x018F, 0x014F, 0x00CF, 0x002F, 0x012F,
    0x01AF, 0x006F, 0x00EF, 0x01EF, 0x001F, 0x021F, 0x011F, 0x031F,
    0x009F, 0x029F, 0x019F, 0x039F, 0x005F, 0x025F, 0x015F, 0x035F,
    0x00DF, 0x02DF, 0x01DF, 0x03DF, 0x003F, 0x103F, 0x083F, 0x183F,
    0x043F, 0x143F, 0x0C3F, 0x1C3F, 0x023F, 0x123F, 0x0A3F, 0x1A3F,
    0x063F, 0x163F, 0x0E3F, 0x1E3F, 0x013F, 0x113F, 0x093F, 0x193F,
    0x053F, 0x153F, 0x0D3F, 0x1D3F, 0x033F, 0x133F, 0x0B3F, 0x1B3F,
    0x073F, 0x173F, 0x0F3F, 0x1F3F, 0x00BF, 0x10BF, 0x08BF, 0x18BF,
    0x04BF, 0x14BF, 0x0CBF, 0x1CBF, 0x02BF, 0x12BF, 0x0ABF, 0x1ABF,
    0x06BF, 0x16BF, 0x0EBF, 0x1EBF, 0x01BF, 0x11BF, 0x09BF, 0x19BF,
    0x05BF, 0x15BF, 0x0DBF, 0x1DBF, 0x03BF, 0x13BF, 0x0BBF, 0x1BBF,
    0x07BF, 0x17BF, 0x0FBF, 0x1FBF, 0x007F, 0x207F, 0x107F, 0x307F,
    0x087F, 0x287F, 0x187F, 0x387F, 0x047F, 0x247F, 0x147F, 0x0002,
    0x0011, 0x0005, 0x0015, 0x0003, 0x003B, 0x0047, 0x00C7, 0x0017,
    0x00D7, 0x0077, 0x010F, 0x004F, 0x01CF, 0x00AF, 0x016F
];

static INDEO2_CODE_LENGTHS: &[u8] = &[
     3,  3,  3,  5,  5,  5,  5,  5,  6,  6,  6,  6,  6,  6,  8,  8,
     8,  8,  8,  8,  8,  8,  8,  8,  8,  9,  9,  9,  9,  9,  9,  9,
     9,  9,  9,  9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
    10, 10, 10, 10, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13,
    13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13,
    13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13,
    13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13,
    13, 13, 13, 13, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14,  3,
     5,  5,  5,  6,  6,  8,  8,  8,  8,  8,  9,  9,  9,  9,  9
];

struct IR2CodeReader { }

impl CodebookDescReader<u8> for IR2CodeReader {
    fn bits(&mut self, idx: usize) -> u8  { INDEO2_CODE_LENGTHS[idx] }
    fn code(&mut self, idx: usize) -> u32 { u32::from(INDEO2_CODE_CODES[idx]) }
    fn sym (&mut self, idx: usize) -> u8 {
        if idx < 0x7F { (idx + 1) as u8 } else { (idx + 2) as u8 }
    }
    fn len(&mut self) -> usize { INDEO2_CODE_LENGTHS.len() }
}

struct Indeo2Decoder {
    info:    NACodecInfoRef,
    cb:      Codebook<u8>,
    frmmgr:  HAMShuffler,
}

impl Indeo2Decoder {
    fn new() -> Self {
        let dummy_info = NACodecInfo::new_dummy();
        let mut coderead = IR2CodeReader{};
        let cb = Codebook::new(&mut coderead, CodebookMode::LSB).unwrap();
        Indeo2Decoder { info: dummy_info, cb, frmmgr: HAMShuffler::new() }
    }

    fn decode_plane_intra(&self, br: &mut BitReader,
                          buf: &mut NAVideoBuffer<u8>, planeno: usize,
                          tableno: usize) -> DecoderResult<()> {
        let offs = buf.get_offset(planeno);
        let (w, h) = buf.get_dimensions(planeno);
        let stride = buf.get_stride(planeno);
        let cb = &self.cb;

        let data = buf.get_data_mut().unwrap();
        let framebuf: &mut [u8] = data.as_mut_slice();

        let table = &INDEO2_DELTA_TABLE[tableno];

        let mut base = offs;
        let mut x: usize = 0;
        while x < w {
            let idx = br.read_cb(cb)? as usize;
            if idx >= 0x80 {
                let run = (idx - 0x80) * 2;
                if x + run > w { return Err(DecoderError::InvalidData); }
                for i in 0..run {
                    framebuf[base + x + i] = 0x80;
                }
                x += run;
            } else {
                framebuf[base + x + 0] = table[(idx * 2 + 0) as usize];
                framebuf[base + x + 1] = table[(idx * 2 + 1) as usize];
                x += 2;
            }
        }
        base += stride;
        for _ in 1..h {
            let mut x: usize = 0;
            while x < w {
                let idx = br.read_cb(cb)? as usize;
                if idx >= 0x80 {
                    let run = (idx - 0x80) * 2;
                    if x + run > w { return Err(DecoderError::InvalidData); }
                    for i in 0..run {
                        framebuf[base + x + i] = framebuf[base + x + i - stride];
                    }
                    x += run;
                } else {
                    let delta0 = i16::from(table[idx * 2 + 0]) - 0x80;
                    let delta1 = i16::from(table[idx * 2 + 1]) - 0x80;
                    let mut pix0 = i16::from(framebuf[base + x + 0 - stride]);
                    let mut pix1 = i16::from(framebuf[base + x + 1 - stride]);
                    pix0 += delta0;
                    pix1 += delta1;
                    if pix0 < 0 { pix0 = 0; }
                    if pix1 < 0 { pix1 = 0; }
                    if pix0 > 255 { pix0 = 255; }
                    if pix1 > 255 { pix1 = 255; }
                    framebuf[base + x + 0] = pix0 as u8;
                    framebuf[base + x + 1] = pix1 as u8;
                    x += 2;
                }
            }
            base += stride;
        }
        Ok(())
    }

    fn decode_plane_inter(&self, br: &mut BitReader,
                          buf: &mut NAVideoBuffer<u8>, planeno: usize,
                          tableno: usize) -> DecoderResult<()> {
        let offs = buf.get_offset(planeno);
        let (w, h) = buf.get_dimensions(planeno);
        let stride = buf.get_stride(planeno);
        let cb = &self.cb;

        let data = buf.get_data_mut().unwrap();
        let framebuf: &mut [u8] = data.as_mut_slice();

        let table = &INDEO2_DELTA_TABLE[tableno];

        let mut base = offs;
        for _ in 0..h {
            let mut x: usize = 0;
            while x < w {
                let idx = br.read_cb(cb)? as usize;
                if idx >= 0x80 {
                    let run = (idx - 0x80) * 2;
                    if x + run > w { return Err(DecoderError::InvalidData); }
                    x += run;
                } else {
                    let delta0 = i16::from(table[idx * 2 + 0]) - 0x80;
                    let delta1 = i16::from(table[idx * 2 + 1]) - 0x80;
                    let mut pix0 = i16::from(framebuf[base + x + 0]);
                    let mut pix1 = i16::from(framebuf[base + x + 1]);
                    pix0 += (delta0 * 3) >> 2;
                    pix1 += (delta1 * 3) >> 2;
                    if pix0 < 0 { pix0 = 0; }
                    if pix1 < 0 { pix1 = 0; }
                    if pix0 > 255 { pix0 = 255; }
                    if pix1 > 255 { pix1 = 255; }
                    framebuf[base + x + 0] = pix0 as u8;
                    framebuf[base + x + 1] = pix1 as u8;
                    x += 2;
                }
            }
            base += stride;
        }
        Ok(())
    }
}

const IR2_START: usize = 48;

impl NADecoder for Indeo2Decoder {
    fn init(&mut self, _supp: &mut NADecoderSupport, info: NACodecInfoRef) -> DecoderResult<()> {
        if let NACodecTypeInfo::Video(vinfo) = info.get_properties() {
            let w = vinfo.get_width();
            let h = vinfo.get_height();
            let f = vinfo.is_flipped();
            let fmt = formats::YUV410_FORMAT;
            let myinfo = NACodecTypeInfo::Video(NAVideoInfo::new(w, h, f, fmt));
            self.info = NACodecInfo::new_ref(info.get_name(), myinfo, info.get_extradata()).into_ref();
            self.frmmgr.clear();
            Ok(())
        } else {
            Err(DecoderError::InvalidData)
        }
    }
    fn decode(&mut self, _supp: &mut NADecoderSupport, pkt: &NAPacket) -> DecoderResult<NAFrameRef> {
        let src = pkt.get_buffer();
        if src.len() <= IR2_START { return Err(DecoderError::ShortData); }
        let interframe = src[18];
        let tabs = src[34];
        let mut br = BitReader::new(&src[IR2_START..], src.len() - IR2_START, BitReaderMode::LE);
        let luma_tab = tabs & 3;
        let chroma_tab = (tabs >> 2) & 3;
        if interframe != 0 {
            let vinfo = self.info.get_properties().get_video_info().unwrap();
            let bufinfo = alloc_video_buffer(vinfo, 2)?;
            let mut buf = bufinfo.get_vbuf().unwrap();
            for plane in 0..3 {
                let tabidx = (if plane == 0 { luma_tab } else { chroma_tab }) as usize;
                let planeno = if plane == 0 { 0 } else { plane ^ 3 };
                self.decode_plane_intra(&mut br, &mut buf, planeno, tabidx)?;
            }
            self.frmmgr.add_frame(buf);
            let mut frm = NAFrame::new_from_pkt(pkt, self.info.clone(), bufinfo);
            frm.set_keyframe(true);
            frm.set_frame_type(FrameType::I);
            Ok(frm.into_ref())
        } else {
            let bufret = self.frmmgr.clone_ref();
            if bufret.is_none() { return Err(DecoderError::MissingReference); }
            let mut buf = bufret.unwrap();

            for plane in 0..3 {
                let tabidx = (if plane == 0 { luma_tab } else { chroma_tab }) as usize;
                let planeno = if plane == 0 { 0 } else { plane ^ 3 };
                self.decode_plane_inter(&mut br, &mut buf, planeno, tabidx)?;
            }
            let mut frm = NAFrame::new_from_pkt(pkt, self.info.clone(), NABufferType::Video(buf));
            frm.set_keyframe(false);
            frm.set_frame_type(FrameType::P);
            Ok(frm.into_ref())
        }
    }
    fn flush(&mut self) {
        self.frmmgr.clear();
    }
}

pub fn get_decoder() -> Box<dyn NADecoder + Send> {
    Box::new(Indeo2Decoder::new())
}

#[cfg(test)]
mod test {
    use nihav_core::codecs::RegisteredDecoders;
    use nihav_core::demuxers::RegisteredDemuxers;
    use nihav_core::test::dec_video::*;
    use crate::codecs::indeo_register_all_codecs;
    use nihav_commonfmt::demuxers::generic_register_all_demuxers;
    #[test]
    fn test_indeo2() {
        let mut dmx_reg = RegisteredDemuxers::new();
        generic_register_all_demuxers(&mut dmx_reg);
        let mut dec_reg = RegisteredDecoders::new();
        indeo_register_all_codecs(&mut dec_reg);

        test_decoding("avi", "indeo2", "assets/Indeo/laser05.avi", Some(10),
                      &dmx_reg, &dec_reg, ExpectedTestResult::MD5Frames(vec![
                            [0x55f509ad, 0x62fb52d5, 0x6e9a86b2, 0x3910ce74],
                            [0x76a2b95d, 0x97bd2eca, 0xc9815f99, 0xe196b47a],
                            [0x4ce19793, 0x46ff7429, 0x89d5c3aa, 0x822b8825],
                            [0xb9cd338f, 0x3d4884a7, 0x5a9e978d, 0xc5abcfe8],
                            [0xc4c6997a, 0x7dbb3a97, 0x1e4e65f6, 0xb5b6fba5],
                            [0xe315980e, 0x817f51e5, 0xf9a45363, 0x943c94b9],
                            [0x09b8c723, 0xb39aa17e, 0x6de2a61b, 0xaceca224],
                            [0xdc1b1966, 0xba5a13b3, 0x3a7fbdab, 0xdebb504c],
                            [0xd33eed2a, 0x7b3834a6, 0x2d57cd23, 0x73644cd9],
                            [0xd7bd2ade, 0x114f973e, 0xe9a9cf45, 0x3c04297e],
                            [0x4d851f61, 0x519c41df, 0x325dc9f9, 0xdf88b57a]]));
    }
}
