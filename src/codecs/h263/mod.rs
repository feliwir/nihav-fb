use std::fmt;
use std::ops::{Add, Sub};
use super::DecoderResult;
use frame::NAVideoBuffer;

pub mod code;
pub mod data;
pub mod decoder;

#[cfg(feature="decoder_intel263")]
pub mod intel263;
#[cfg(feature="decoder_realvideo1")]
pub mod rv10;

pub trait BlockDecoder {
    fn decode_pichdr(&mut self) -> DecoderResult<PicInfo>;
    fn decode_slice_header(&mut self, pinfo: &PicInfo) -> DecoderResult<Slice>;
    fn decode_block_header(&mut self, pinfo: &PicInfo, sinfo: &Slice) -> DecoderResult<BlockInfo>;
    fn decode_block_intra(&mut self, info: &BlockInfo, quant: u8, no: usize, coded: bool, blk: &mut [i16; 64]) -> DecoderResult<()>;
    fn decode_block_inter(&mut self, info: &BlockInfo, quant: u8, no: usize, coded: bool, blk: &mut [i16; 64]) -> DecoderResult<()>;
    fn is_slice_end(&mut self) -> bool;

    fn filter_row(&mut self, buf: &mut NAVideoBuffer<u8>, mb_y: usize, mb_w: usize, cbpi: &CBPInfo);
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub enum Type {
    I, P, Skip, Special
}

#[allow(dead_code)]
#[derive(Debug,Clone,Copy)]
pub struct PBInfo {
    trb:        u8,
    dbquant:    u8,
}

impl PBInfo {
    pub fn new(trb: u8, dbquant: u8) -> Self {
        PBInfo{ trb: trb, dbquant: dbquant }
    }
    pub fn get_trb(&self) -> u8 { self.trb }
    pub fn get_dbquant(&self) -> u8 { self.dbquant }
}

#[allow(dead_code)]
#[derive(Debug,Clone,Copy)]
pub struct PicInfo {
    w:      usize,
    h:      usize,
    mode:   Type,
    quant:  u8,
    apm:    bool,
    mvmode: MVMode,
    pb:     Option<PBInfo>,
    ts:     u8,
    deblock: bool,
}

#[allow(dead_code)]
impl PicInfo {
    pub fn new(w: usize, h: usize, mode: Type, quant: u8, apm: bool, mvmode: MVMode, ts: u8, pb: Option<PBInfo>, deblock: bool) -> Self {
        PicInfo{ w: w, h: h, mode: mode, quant: quant, apm: apm, mvmode: mvmode, ts: ts, pb: pb, deblock: deblock }
    }
    pub fn get_width(&self) -> usize { self.w }
    pub fn get_height(&self) -> usize { self.h }
    pub fn get_mode(&self) -> Type { self.mode }
    pub fn get_quant(&self) -> u8 { self.quant }
    pub fn get_apm(&self) -> bool { self.apm }
    pub fn get_mvmode(&self) -> MVMode { self.mvmode }
    pub fn is_pb(&self) -> bool { self.pb.is_some() }
    pub fn get_ts(&self) -> u8 { self.ts }
    pub fn get_pbinfo(&self) -> PBInfo { self.pb.unwrap() }
}

#[derive(Debug,Clone,Copy)]
pub struct Slice {
    mb_x:   usize,
    mb_y:   usize,
    quant:  u8,
}

impl Slice {
    pub fn new(mb_x: usize, mb_y: usize, quant: u8) -> Self {
        Slice{ mb_x: mb_x, mb_y: mb_y, quant: quant }
    }
    pub fn get_default_slice(pinfo: &PicInfo) -> Self {
        Slice{ mb_x: 0, mb_y: 0, quant: pinfo.get_quant() }
    }
    pub fn get_quant(&self) -> u8 { self.quant }
}

#[derive(Debug,Clone,Copy)]
pub struct BlockInfo {
    intra:   bool,
    skip:    bool,
    mode:    Type,
    cbp:     u8,
    q:       u8,
    mv:      [MV; 4],
    num_mv:  usize,
    bpart:   bool,
    b_cbp:   u8,
    mv2:     [MV; 2],
    num_mv2: usize,
    fwd:     bool,
}

#[allow(dead_code)]
#[derive(Debug,Clone,Copy)]
pub struct BBlockInfo {
    present: bool,
    cbp:     u8,
    num_mv:  usize,
    fwd:     bool,
}

#[allow(dead_code)]
impl BlockInfo {
    pub fn new(mode: Type, cbp: u8, q: u8) -> Self {
        BlockInfo {
            intra:   mode == Type::I,
            skip:    (cbp == 0) && (mode != Type::I),
            mode:    mode,
            cbp:     cbp,
            q:       q,
            mv:      [MV::new(0, 0), MV::new(0, 0), MV::new(0, 0), MV::new(0, 0)],
            num_mv:  0,
            bpart:   false,
            b_cbp:   0,
            mv2:     [ZERO_MV, ZERO_MV],
            num_mv2: 0,
            fwd:     false,
        }
    }
    pub fn is_intra(&self) -> bool { self.intra }
    pub fn is_skipped(&self) -> bool { self.skip }
    pub fn get_mode(&self) -> Type { self.mode }
    pub fn get_cbp(&self) -> u8 { self.cbp }
    pub fn get_q(&self) -> u8 { self.q }
    pub fn get_num_mvs(&self) -> usize { self.num_mv }
    pub fn get_mv(&self, idx: usize) -> MV { self.mv[idx] }
    pub fn has_b_part(&self) -> bool { self.bpart }
    pub fn get_cbp_b(&self) -> u8 { self.b_cbp }
    pub fn get_num_mvs2(&self) -> usize { self.num_mv2 }
    pub fn get_mv2(&self, idx: usize) -> MV { self.mv2[idx] }
    pub fn set_mv(&mut self, mvs: &[MV]) {
        if mvs.len() > 0 { self.skip = false; }
        let mut mv_arr: [MV; 4] = [MV::new(0, 0), MV::new(0, 0), MV::new(0, 0), MV::new(0, 0)];
        for i in 0..mvs.len() { mv_arr[i] = mvs[i]; }
        self.mv     = mv_arr;
        self.num_mv = mvs.len();
    }
    pub fn set_bpart(&mut self, bbinfo: BBlockInfo) {
        self.bpart = bbinfo.present;
        self.b_cbp = bbinfo.cbp;
        self.fwd   = bbinfo.fwd;
        self.num_mv2 = bbinfo.get_num_mv();
    }
    pub fn set_b_mv(&mut self, mvs: &[MV]) {
        if mvs.len() > 0 { self.skip = false; }
        let mut mv_arr: [MV; 2] = [ZERO_MV, ZERO_MV];
        for i in 0..mvs.len() { mv_arr[i] = mvs[i]; }
        self.mv2    = mv_arr;
    }
    pub fn is_b_fwd(&self) -> bool { self.fwd }
}

impl BBlockInfo {
    pub fn new(present: bool, cbp: u8, num_mv: usize, fwd: bool) -> Self {
        BBlockInfo {
            present: present,
            cbp:     cbp,
            num_mv:  num_mv,
            fwd:     fwd,
        }
    }
    pub fn get_num_mv(&self) -> usize { self.num_mv }
}

#[derive(Debug,Clone,Copy)]
pub enum MVMode {
    Old,
    Long,
    UMV,
}

#[derive(Debug,Clone,Copy)]
pub struct MV {
    x: i16,
    y: i16,
}

impl MV {
    pub fn new(x: i16, y: i16) -> Self { MV{ x: x, y: y } }
    pub fn pred(a: MV, b: MV, c: MV) -> Self {
        let x;
        if a.x < b.x {
            if b.x < c.x {
                x = b.x;
            } else {
                if a.x < c.x { x = c.x; } else { x = a.x; }
            }
        } else {
            if b.x < c.x {
                if a.x < c.x { x = a.x; } else { x = c.x; }
            } else {
                x = b.x;
            }
        }
        let y;
        if a.y < b.y {
            if b.y < c.y {
                y = b.y;
            } else {
                if a.y < c.y { y = c.y; } else { y = a.y; }
            }
        } else {
            if b.y < c.y {
                if a.y < c.y { y = a.y; } else { y = c.y; }
            } else {
                y = b.y;
            }
        }
        MV { x: x, y: y }
    }
    fn add_umv(pred_mv: MV, add: MV, mvmode: MVMode) -> Self {
        let mut new_mv = pred_mv + add;
        match mvmode {
            MVMode::Old => {
                    if      new_mv.x >=  64 { new_mv.x -= 64; }
                    else if new_mv.x <= -64 { new_mv.x += 64; }
                    if      new_mv.y >=  64 { new_mv.y -= 64; }
                    else if new_mv.y <= -64 { new_mv.y += 64; }
                },
            MVMode::Long => {
                    if      new_mv.x >  31 { new_mv.x -= 64; }
                    else if new_mv.x < -32 { new_mv.x += 64; }
                    if      new_mv.y >  31 { new_mv.y -= 64; }
                    else if new_mv.y < -32 { new_mv.y += 64; }
                },
            MVMode::UMV => {
                    if pred_mv.x >  32 && new_mv.x >  63 { new_mv.x -= 64; }
                    if pred_mv.x < -31 && new_mv.x < -63 { new_mv.x += 64; }
                    if pred_mv.y >  32 && new_mv.y >  63 { new_mv.y -= 64; }
                    if pred_mv.y < -31 && new_mv.y < -63 { new_mv.y += 64; }
                },
        };
        new_mv
    }
    fn scale(&self, trb: u8, trd: u8) -> Self {
        if (trd == 0) || (trb == 0) {
            ZERO_MV
        } else {
            MV { x: (self.x * (trb as i16)) / (trd as i16), y: (self.y * (trb as i16)) / (trd as i16) }
        }
    }
    fn b_sub(pvec: MV, fwdvec: MV, bvec: MV, trb: u8, trd: u8) -> Self {
        let bscale = (trb as i16) - (trd as i16);
        let x = if bvec.x != 0 { fwdvec.x - pvec.x } else if trd != 0 { bscale * pvec.x / (trd as i16) } else { 0 };
        let y = if bvec.y != 0 { fwdvec.y - pvec.y } else if trd != 0 { bscale * pvec.y / (trd as i16) } else { 0 };
        MV { x: x, y: y }
    }
}

pub const ZERO_MV: MV = MV { x: 0, y: 0 };

impl Add for MV {
    type Output = MV;
    fn add(self, other: MV) -> MV { MV { x: self.x + other.x, y: self.y + other.y } }
}

impl Sub for MV {
    type Output = MV;
    fn sub(self, other: MV) -> MV { MV { x: self.x - other.x, y: self.y - other.y } }
}

impl fmt::Display for MV {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{},{}", self.x, self.y)
    }
}

#[allow(dead_code)]
pub struct CBPInfo {
    cbp:        Vec<u8>,
    q:          Vec<u8>,
    mb_w:       usize,
}

impl CBPInfo {
    fn new() -> Self { CBPInfo{ cbp: Vec::new(), q: Vec::new(), mb_w: 0 } }
    fn reset(&mut self, mb_w: usize) {
        self.mb_w = mb_w;
        self.cbp.truncate(0);
        self.cbp.resize(self.mb_w * 2, 0);
        self.q.truncate(0);
        self.q.resize(self.mb_w * 2, 0);
    }
    fn update_row(&mut self) {
        for i in 0..self.mb_w {
            self.cbp[i] = self.cbp[self.mb_w + i];
            self.q[i]   = self.q[self.mb_w + i];
        }
    }
    fn set_cbp(&mut self, mb_x: usize, cbp: u8) {
        self.cbp[self.mb_w + mb_x] = cbp;
    }
    fn set_q(&mut self, mb_x: usize, q: u8) {
        self.q[self.mb_w + mb_x] = q;
    }
    pub fn get_q(&self, mb_x: usize) -> u8 { self.q[mb_x] }
    pub fn is_coded(&self, mb_x: usize, blk_no: usize) -> bool {
        (self.cbp[self.mb_w + mb_x] & (1 << (5 - blk_no))) != 0
    }
    pub fn is_coded_top(&self, mb_x: usize, blk_no: usize) -> bool {
        let cbp     = self.cbp[self.mb_w + mb_x];
        let cbp_top = self.cbp[mb_x];
        match blk_no {
            0 => { (cbp_top & 0b001000) != 0 },
            1 => { (cbp_top & 0b000100) != 0 },
            2 => { (cbp     & 0b100000) != 0 },
            3 => { (cbp     & 0b010000) != 0 },
            4 => { (cbp_top & 0b000010) != 0 },
            _ => { (cbp_top & 0b000001) != 0 },
        }
    }
}

