use nihav_core::codecs::*;

#[derive(Clone,Copy,Debug,PartialEq)]
#[allow(dead_code)]
pub enum VPMBType {
    Intra,
    InterNoMV,
    InterMV,
    InterNearest,
    InterNear,
    InterFourMV,
    GoldenNoMV,
    GoldenMV,
    GoldenNearest,
    GoldenNear,
}

#[allow(dead_code)]
impl VPMBType {
    pub fn is_intra(self) -> bool { self == VPMBType::Intra }
    pub fn get_ref_id(self) -> u8 {
        match self {
            VPMBType::Intra         => 0,
            VPMBType::InterNoMV     |
            VPMBType::InterMV       |
            VPMBType::InterNearest  |
            VPMBType::InterNear     |
            VPMBType::InterFourMV   => 1,
            _                       => 2,
        }
    }
}

impl Default for VPMBType {
    fn default() -> Self { VPMBType::Intra }
}

#[derive(Default)]
pub struct VPShuffler {
    lastframe: Option<NAVideoBufferRef<u8>>,
    goldframe: Option<NAVideoBufferRef<u8>>,
}

impl VPShuffler {
    pub fn new() -> Self { VPShuffler { lastframe: None, goldframe: None } }
    pub fn clear(&mut self) { self.lastframe = None; self.goldframe = None; }
    pub fn add_frame(&mut self, buf: NAVideoBufferRef<u8>) {
        self.lastframe = Some(buf);
    }
    pub fn add_golden_frame(&mut self, buf: NAVideoBufferRef<u8>) {
        self.goldframe = Some(buf);
    }
    pub fn get_last(&mut self) -> Option<NAVideoBufferRef<u8>> {
        if let Some(ref frm) = self.lastframe {
            Some(frm.clone())
        } else {
            None
        }
    }
    pub fn get_golden(&mut self) -> Option<NAVideoBufferRef<u8>> {
        if let Some(ref frm) = self.goldframe {
            Some(frm.clone())
        } else {
            None
        }
    }
}

const C1S7: i32 = 64277;
const C2S6: i32 = 60547;
const C3S5: i32 = 54491;
const C4S4: i32 = 46341;
const C5S3: i32 = 36410;
const C6S2: i32 = 25080;
const C7S1: i32 = 12785;

fn mul16(a: i32, b: i32) -> i32 {
    (a * b) >> 16
}

macro_rules! idct_step {
    ($s0:expr, $s1:expr, $s2:expr, $s3:expr, $s4:expr, $s5:expr, $s6:expr, $s7:expr,
     $d0:expr, $d1:expr, $d2:expr, $d3:expr, $d4:expr, $d5:expr, $d6:expr, $d7:expr,
     $bias:expr, $shift:expr, $otype:ty) => {
        let t_a  = mul16(C1S7, i32::from($s1)) + mul16(C7S1, i32::from($s7));
        let t_b  = mul16(C7S1, i32::from($s1)) - mul16(C1S7, i32::from($s7));
        let t_c  = mul16(C3S5, i32::from($s3)) + mul16(C5S3, i32::from($s5));
        let t_d  = mul16(C3S5, i32::from($s5)) - mul16(C5S3, i32::from($s3));
        let t_a1 = mul16(C4S4, t_a - t_c);
        let t_b1 = mul16(C4S4, t_b - t_d);
        let t_c  = t_a + t_c;
        let t_d  = t_b + t_d;
        let t_e  = mul16(C4S4, i32::from($s0 + $s4)) + $bias;
        let t_f  = mul16(C4S4, i32::from($s0 - $s4)) + $bias;
        let t_g  = mul16(C2S6, i32::from($s2)) + mul16(C6S2, i32::from($s6));
        let t_h  = mul16(C6S2, i32::from($s2)) - mul16(C2S6, i32::from($s6));
        let t_e1 = t_e  - t_g;
        let t_g  = t_e  + t_g;
        let t_a  = t_f  + t_a1;
        let t_f  = t_f  - t_a1;
        let t_b  = t_b1 - t_h;
        let t_h  = t_b1 + t_h;

        $d0 = ((t_g  + t_c) >> $shift) as $otype;
        $d7 = ((t_g  - t_c) >> $shift) as $otype;
        $d1 = ((t_a  + t_h) >> $shift) as $otype;
        $d2 = ((t_a  - t_h) >> $shift) as $otype;
        $d3 = ((t_e1 + t_d) >> $shift) as $otype;
        $d4 = ((t_e1 - t_d) >> $shift) as $otype;
        $d5 = ((t_f  + t_b) >> $shift) as $otype;
        $d6 = ((t_f  - t_b) >> $shift) as $otype;
    }
}

pub fn vp_idct(coeffs: &mut [i16; 64]) {
    let mut tmp = [0i32; 64];
    for (src, dst) in coeffs.chunks(8).zip(tmp.chunks_mut(8)) {
        idct_step!(src[0], src[1], src[2], src[3], src[4], src[5], src[6], src[7],
                   dst[0], dst[1], dst[2], dst[3], dst[4], dst[5], dst[6], dst[7], 0, 0, i32);
    }
    let src = &tmp;
    let dst = coeffs;
    for i in 0..8 {
        idct_step!(src[0 * 8 + i], src[1 * 8 + i], src[2 * 8 + i], src[3 * 8 + i],
                   src[4 * 8 + i], src[5 * 8 + i], src[6 * 8 + i], src[7 * 8 + i],
                   dst[0 * 8 + i], dst[1 * 8 + i], dst[2 * 8 + i], dst[3 * 8 + i],
                   dst[4 * 8 + i], dst[5 * 8 + i], dst[6 * 8 + i], dst[7 * 8 + i], 8, 4, i16);
    }
}

pub fn vp_idct_dc(coeffs: &mut [i16; 64]) {
    let dc = ((mul16(C4S4, mul16(C4S4, i32::from(coeffs[0]))) + 8) >> 4) as i16;
    for i in 0..64 {
        coeffs[i] = dc;
    }
}

pub fn unquant(coeffs: &mut [i16; 64], qmat: &[i16; 64]) {
    for i in 1..64 {
        coeffs[i] = coeffs[i].wrapping_mul(qmat[i]);
    }
}

pub fn vp_put_block(coeffs: &mut [i16; 64], bx: usize, by: usize, plane: usize, frm: &mut NASimpleVideoFrame<u8>) {
    vp_idct(coeffs);
    let mut off = frm.offset[plane] + bx * 8 + by * 8 * frm.stride[plane];
    for y in 0..8 {
        for x in 0..8 {
            frm.data[off + x] = (coeffs[x + y * 8] + 128).min(255).max(0) as u8;
        }
        off += frm.stride[plane];
    }
}

pub fn vp_put_block_dc(coeffs: &mut [i16; 64], bx: usize, by: usize, plane: usize, frm: &mut NASimpleVideoFrame<u8>) {
    vp_idct_dc(coeffs);
    let dc = (coeffs[0] + 128).min(255).max(0) as u8;
    let mut off = frm.offset[plane] + bx * 8 + by * 8 * frm.stride[plane];
    for _ in 0..8 {
        for x in 0..8 {
            frm.data[off + x] = dc;
        }
        off += frm.stride[plane];
    }
}

pub fn vp_add_block(coeffs: &mut [i16; 64], bx: usize, by: usize, plane: usize, frm: &mut NASimpleVideoFrame<u8>) {
    vp_idct(coeffs);
    let mut off = frm.offset[plane] + bx * 8 + by * 8 * frm.stride[plane];
    for y in 0..8 {
        for x in 0..8 {
            frm.data[off + x] = (coeffs[x + y * 8] + (frm.data[off + x] as i16)).min(255).max(0) as u8;
        }
        off += frm.stride[plane];
    }
}

pub fn vp_add_block_dc(coeffs: &mut [i16; 64], bx: usize, by: usize, plane: usize, frm: &mut NASimpleVideoFrame<u8>) {
    vp_idct_dc(coeffs);
    let dc = coeffs[0];
    let mut off = frm.offset[plane] + bx * 8 + by * 8 * frm.stride[plane];
    for _ in 0..8 {
        for x in 0..8 {
            frm.data[off + x] = (dc + (frm.data[off + x] as i16)).min(255).max(0) as u8;
        }
        off += frm.stride[plane];
    }
}