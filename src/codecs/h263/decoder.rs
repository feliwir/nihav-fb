//use std::mem;
use frame::*;
use super::super::*;
use super::super::blockdsp;
use super::*;
//use super::code::*;
use formats;

#[allow(dead_code)]
struct MVInfo {
    mv:         Vec<MV>,
    mb_w:       usize,
    mb_stride:  usize,
    mb_start:   usize,
    top:        bool,
    mvmode:     MVMode,
}

impl MVInfo {
    fn new() -> Self { MVInfo{ mv: Vec::new(), mb_w: 0, mb_stride: 0, mb_start: 0, top: true, mvmode: MVMode::Old } }
    fn reset(&mut self, mb_w: usize, mb_start: usize, mvmode: MVMode) {
        self.mb_start  = mb_start;
        self.mb_w      = mb_w;
        self.mb_stride = mb_w * 2;
        self.mv.resize(self.mb_stride * 3, ZERO_MV);
        self.mvmode    = mvmode;
    }
    fn update_row(&mut self) {
        self.mb_start = self.mb_w + 1;
        self.top      = false;
        for i in 0..self.mb_stride {
            self.mv[i] = self.mv[self.mb_stride * 2 + i];
        }
    }
    #[allow(non_snake_case)]
    fn predict(&mut self, mb_x: usize, blk_no: usize, use4: bool, diff: MV, first_line: bool, first_mb: bool) -> MV {
        let A;
        let B;
        let C;
        let last = mb_x == self.mb_w - 1;
        match blk_no {
            0 => {
                    if mb_x != self.mb_start {
                        A = if !first_mb   { self.mv[self.mb_stride + mb_x * 2 - 1] } else { ZERO_MV };
                        B = if !first_line { self.mv[                 mb_x * 2] } else { A };
                        C = if !first_line && !last { self.mv[mb_x * 2 + 2] } else { ZERO_MV };
                    } else {
                        A = ZERO_MV; B = ZERO_MV; C = ZERO_MV;
                    }
                },
            1 => {
                    A = self.mv[self.mb_stride + mb_x * 2];
                    B = if !first_line { self.mv[mb_x * 2 + 1] } else { A };
                    C = if !first_line && !last { self.mv[mb_x * 2 + 2] } else { A };
                },
            2 => {
                    A = if mb_x != self.mb_start { self.mv[self.mb_stride * 2 + mb_x * 2 - 1] } else { ZERO_MV };
                    B = self.mv[self.mb_stride + mb_x * 2];
                    C = self.mv[self.mb_stride + mb_x * 2 + 1];
                },
            3 => {
                    A = self.mv[self.mb_stride * 2 + mb_x * 2];
                    B = self.mv[self.mb_stride * 1 + mb_x * 2 + 1];
                    C = self.mv[self.mb_stride * 1 + mb_x * 2];
                },
            _ => { return ZERO_MV; }
        }
        let pred_mv = MV::pred(A, B, C);
        let new_mv = MV::add_umv(pred_mv, diff, self.mvmode);
        if !use4 {
            self.mv[self.mb_stride * 1 + mb_x * 2 + 0] = new_mv;
            self.mv[self.mb_stride * 1 + mb_x * 2 + 1] = new_mv;
            self.mv[self.mb_stride * 2 + mb_x * 2 + 0] = new_mv;
            self.mv[self.mb_stride * 2 + mb_x * 2 + 1] = new_mv;
        } else {
            match blk_no {
                0 => { self.mv[self.mb_stride * 1 + mb_x * 2 + 0] = new_mv; },
                1 => { self.mv[self.mb_stride * 1 + mb_x * 2 + 1] = new_mv; },
                2 => { self.mv[self.mb_stride * 2 + mb_x * 2 + 0] = new_mv; },
                3 => { self.mv[self.mb_stride * 2 + mb_x * 2 + 1] = new_mv; },
                _ => {},
            };
        }
        
        new_mv
    }
    fn set_zero_mv(&mut self, mb_x: usize) {
        self.mv[self.mb_stride * 1 + mb_x * 2 + 0] = ZERO_MV;
        self.mv[self.mb_stride * 1 + mb_x * 2 + 1] = ZERO_MV;
        self.mv[self.mb_stride * 2 + mb_x * 2 + 0] = ZERO_MV;
        self.mv[self.mb_stride * 2 + mb_x * 2 + 1] = ZERO_MV;
    }
}

#[allow(dead_code)]
#[derive(Clone,Copy)]
struct BMB {
    num_mv: usize,
    mv_f:   [MV; 4],
    mv_b:   [MV; 4],
    fwd:    bool,
    blk:    [[i16; 64]; 6],
    cbp:    u8,
}

impl BMB {
    fn new() -> Self { BMB {blk: [[0; 64]; 6], cbp: 0, fwd: false, mv_f: [ZERO_MV; 4], mv_b: [ZERO_MV; 4], num_mv: 0} }
}

#[derive(Clone,Copy)]
struct PredCoeffs {
    hor: [[i16; 8]; 6],
    ver: [[i16; 8]; 6],
}

const ZERO_PRED_COEFFS: PredCoeffs = PredCoeffs { hor: [[0; 8]; 6], ver: [[0; 8]; 6] };

pub struct H263BaseDecoder {
    w:          usize,
    h:          usize,
    mb_w:       usize,
    mb_h:       usize,
    num_mb:     usize,
    ftype:      Type,
    ipbs:       IPBShuffler,
    next_ts:    u16,
    last_ts:    u16,
    tsdiff:     u16,
    has_b:      bool,
    b_data:     Vec<BMB>,
    pred_coeffs: Vec<PredCoeffs>,
    is_gob:     bool,
    may_have_b_frames: bool,
    mv_data:    Vec<BlockMVInfo>,
}

#[inline]
fn clip_dc(dc: i16) -> i16 {
    if dc < 0 { 0 }
    else if dc > 2046 { 2046 }
    else { (dc + 1) & !1 }
}

#[inline]
fn clip_ac(ac: i16) -> i16 {
    if ac < -2048 { -2048 }
    else if ac > 2047 { 2047 }
    else { ac }
}

#[allow(dead_code)]
impl H263BaseDecoder {
    pub fn new_with_opts(is_gob: bool, may_have_b_frames: bool) -> Self {
        H263BaseDecoder{
            w: 0, h: 0, mb_w: 0, mb_h: 0, num_mb: 0,
            ftype: Type::Special,
            ipbs: IPBShuffler::new(),
            last_ts: 0, next_ts: 0, tsdiff: 0,
            has_b: false, b_data: Vec::new(),
            pred_coeffs: Vec::new(),
            is_gob: is_gob,
            may_have_b_frames: may_have_b_frames,
            mv_data: Vec::new(),
        }
    }
    pub fn new(is_gob: bool) -> Self {
        Self::new_with_opts(is_gob, false)
    }
    pub fn new_b_frames(is_gob: bool) -> Self {
        Self::new_with_opts(is_gob, true)
    }

    pub fn is_intra(&self) -> bool { self.ftype == Type::I }
    pub fn get_frame_type(&self) -> FrameType {
        match self.ftype {
            Type::I       => FrameType::I,
            Type::P       => FrameType::P,
            Type::B       => FrameType::B,
            Type::PB      => FrameType::P,
            Type::Skip    => FrameType::Skip,
            Type::Special => FrameType::Skip,
        }
    }
    pub fn get_dimensions(&self) -> (usize, usize) { (self.w, self.h) }

    pub fn parse_frame(&mut self, bd: &mut BlockDecoder, bdsp: &BlockDSP) -> DecoderResult<NABufferType> {
        let pinfo = bd.decode_pichdr()?;
        let mut mvi = MVInfo::new();
        let mut mvi2 = MVInfo::new();
        let mut cbpi = CBPInfo::new();

//todo handle res change
        self.w = pinfo.w;
        self.h = pinfo.h;
        self.mb_w = (pinfo.w + 15) >> 4;
        self.mb_h = (pinfo.h + 15) >> 4;
        self.num_mb = self.mb_w * self.mb_h;
        self.ftype = pinfo.mode;
        self.has_b = pinfo.is_pb();

        if self.has_b {
            self.mv_data.truncate(0);
        }

        let save_b_data = pinfo.mode.is_ref() && self.may_have_b_frames;
        if save_b_data {
            self.mv_data.truncate(0);
        }
        let is_b = pinfo.mode == Type::B;

        let tsdiff = pinfo.ts.wrapping_sub(self.last_ts) >> 1;
        let bsdiff = if pinfo.is_pb() { (pinfo.get_pbinfo().get_trb() as u16) << 7 }
                     else { pinfo.ts.wrapping_sub(self.next_ts) >> 1 };

        let fmt = formats::YUV420_FORMAT;
        let vinfo = NAVideoInfo::new(self.w, self.h, false, fmt);
        let bufret = alloc_video_buffer(vinfo, 4);
        if let Err(_) = bufret { return Err(DecoderError::InvalidData); }
        let mut bufinfo = bufret.unwrap();
        let mut buf = bufinfo.get_vbuf().unwrap();

        let mut slice = if self.is_gob {
                SliceInfo::get_default_slice(&pinfo)
            } else {
                bd.decode_slice_header(&pinfo)?
            };
        mvi.reset(self.mb_w, 0, pinfo.get_mvmode());
        if is_b {
            mvi2.reset(self.mb_w, 0, pinfo.get_mvmode());
        }
        cbpi.reset(self.mb_w);

        let mut blk: [[i16; 64]; 6] = [[0; 64]; 6];
        let mut sstate = SliceState::new(pinfo.mode == Type::I);
        let mut mb_pos = 0;
        let apply_acpred = (pinfo.mode == Type::I) && pinfo.plusinfo.is_some() && pinfo.plusinfo.unwrap().aic;
        if apply_acpred {
            self.pred_coeffs.truncate(0);
            self.pred_coeffs.resize(self.mb_w * self.mb_h, ZERO_PRED_COEFFS);
        }
        for mb_y in 0..self.mb_h {
            for mb_x in 0..self.mb_w {
                for i in 0..6 { for j in 0..64 { blk[i][j] = 0; } }

                if slice.is_at_end(mb_pos) || (slice.needs_check() && mb_pos > 0 && bd.is_slice_end()) {
                    slice = bd.decode_slice_header(&pinfo)?;
                    if !self.is_gob {
                        mvi.reset(self.mb_w, mb_x, pinfo.get_mvmode());
                        if is_b {
                            mvi2.reset(self.mb_w, mb_x, pinfo.get_mvmode());
                        }
                        cbpi.reset(self.mb_w);
                        sstate.reset_slice(mb_x, mb_y);
                    }
                }

                let binfo = bd.decode_block_header(&pinfo, &slice, &sstate)?;
                let cbp = binfo.get_cbp();
                cbpi.set_cbp(mb_x, cbp);
                cbpi.set_q(mb_x, binfo.get_q());
                if binfo.is_intra() {
                    if save_b_data {
                        self.mv_data.push(BlockMVInfo::Intra);
                    }
                    for i in 0..6 {
                        bd.decode_block_intra(&binfo, &sstate, binfo.get_q(), i, (cbp & (1 << (5 - i))) != 0, &mut blk[i])?;
                        if apply_acpred && (binfo.acpred != ACPredMode::None) {
                            let has_b = (i == 1) || (i == 3) || !sstate.first_mb;
                            let has_a = (i == 2) || (i == 3) || !sstate.first_line;
                            let (b_mb, b_blk) = if has_b {
                                    if (i == 1) || (i == 3) {
                                        (mb_pos, i - 1)
                                    } else if i < 4 {
                                        (mb_pos - 1, i + 1)
                                    } else {
                                        (mb_pos - 1, i)
                                    }
                                } else { (0, 0) };
                            let (a_mb, a_blk) = if has_a {
                                    if (i == 2) || (i == 3) {
                                        (mb_pos, i - 2)
                                    } else if i < 4 {
                                        (mb_pos - self.mb_w, i + 2)
                                    } else {
                                        (mb_pos - self.mb_w, i)
                                    }
                                } else { (0, 0) };
                            match binfo.acpred {
                                ACPredMode::DC   => {
                                            let dc;
                                            if has_a && has_b {
                                                dc = (self.pred_coeffs[b_mb].hor[b_blk][0] + self.pred_coeffs[a_mb].ver[a_blk][0]) / 2;
                                            } else if has_a {
                                                dc = self.pred_coeffs[a_mb].ver[a_blk][0];
                                            } else if has_b {
                                                dc = self.pred_coeffs[b_mb].hor[b_blk][0];
                                            } else {
                                                dc = 1024;
                                            }
                                            blk[i][0] = clip_dc(blk[i][0] + dc);
                                        },
                                ACPredMode::Hor  => {
                                        if has_b {
                                            for k in 0..8 {
                                                blk[i][k * 8] += self.pred_coeffs[b_mb].hor[b_blk][k];
                                            }
                                            for k in 1..8 {
                                                blk[i][k * 8] = clip_ac(blk[i][k * 8]);
                                            }
                                        } else {
                                            blk[i][0] += 1024;
                                        }
                                        blk[i][0] = clip_dc(blk[i][0]);
                                    },
                                ACPredMode::Ver  => {
                                        if has_a {
                                            for k in 0..8 {
                                                blk[i][k] += self.pred_coeffs[a_mb].ver[a_blk][k];
                                            }
                                            for k in 1..8 {
                                                blk[i][k] = clip_ac(blk[i][k]);
                                            }
                                        } else {
                                            blk[i][0] += 1024;
                                        }
                                        blk[i][0] = clip_dc(blk[i][0]);
                                    },
                                ACPredMode::None => {},
                            };
                            for t in 0..8 { self.pred_coeffs[mb_pos].hor[i][t] = blk[i][t * 8]; }
                            for t in 0..8 { self.pred_coeffs[mb_pos].ver[i][t] = blk[i][t]; }
                        }
                        bdsp.idct(&mut blk[i]);
                    }
                    blockdsp::put_blocks(&mut buf, mb_x, mb_y, &blk);
                    mvi.set_zero_mv(mb_x);
                    if is_b {
                        mvi2.set_zero_mv(mb_x);
                    }
                } else if (binfo.mode != Type::B) && !binfo.is_skipped() {
                    if binfo.get_num_mvs() == 1 {
                        let mv = mvi.predict(mb_x, 0, false, binfo.get_mv(0), sstate.first_line, sstate.first_mb);
                        if save_b_data {
                            self.mv_data.push(BlockMVInfo::Inter_1MV(mv));
                        }
                        if let Some(ref srcbuf) = self.ipbs.get_lastref() {
                            bdsp.copy_blocks(&mut buf, srcbuf, mb_x * 16, mb_y * 16, 16, 16, mv);
                        }
                    } else {
                        let mut mv: [MV; 4] = [ZERO_MV, ZERO_MV, ZERO_MV, ZERO_MV];
                        for blk_no in 0..4 {
                            mv[blk_no] = mvi.predict(mb_x, blk_no, true, binfo.get_mv(blk_no), sstate.first_line, sstate.first_mb);
                            if let Some(ref srcbuf) = self.ipbs.get_lastref() {
                                bdsp.copy_blocks(&mut buf, srcbuf,
                                                 mb_x * 16 + (blk_no & 1) * 8,
                                                 mb_y * 16 + (blk_no & 2) * 4, 8, 8, mv[blk_no]);
                            }
                        }
                        if save_b_data {
                            self.mv_data.push(BlockMVInfo::Inter_4MV(mv));
                        }
                    }
                    for i in 0..6 {
                        bd.decode_block_inter(&binfo, &sstate, binfo.get_q(), i, ((cbp >> (5 - i)) & 1) != 0, &mut blk[i])?;
                        bdsp.idct(&mut blk[i]);
                    }
                    blockdsp::add_blocks(&mut buf, mb_x, mb_y, &blk);
                    if is_b {
                        mvi2.set_zero_mv(mb_x);
                    }
                } else if binfo.mode != Type::B {
                    self.mv_data.push(BlockMVInfo::Inter_1MV(ZERO_MV));
                    mvi.set_zero_mv(mb_x);
                    if is_b {
                        mvi2.set_zero_mv(mb_x);
                    }
                    if let Some(ref srcbuf) = self.ipbs.get_lastref() {
                        bdsp.copy_blocks(&mut buf, srcbuf, mb_x * 16, mb_y * 16, 16, 16, ZERO_MV);
                    }
                } else {
                    let ref_mv_info = self.mv_data[mb_pos];
                    let has_fwd = binfo.get_num_mvs() > 0;
                    let has_bwd = binfo.get_num_mvs2() > 0;
//todo refactor
                    if has_fwd || has_bwd {
                        let fwd_mv;
                        if has_fwd {
                            fwd_mv = mvi.predict(mb_x, 0, false, binfo.get_mv(0), sstate.first_line, sstate.first_mb);
                        } else {
                            fwd_mv = ZERO_MV;
                            mvi.set_zero_mv(mb_x);
                        }
                        let bwd_mv;
                        if has_bwd {
                            bwd_mv = mvi2.predict(mb_x, 0, false, binfo.get_mv2(0), sstate.first_line, sstate.first_mb);
                        } else {
                            bwd_mv = ZERO_MV;
                            mvi2.set_zero_mv(mb_x);
                        }
                        if let (Some(ref fwd_buf), Some(ref bck_buf)) = (self.ipbs.get_nextref(), self.ipbs.get_lastref()) {
                            if has_fwd && has_bwd {
                                bdsp.copy_blocks(&mut buf, fwd_buf, mb_x * 16, mb_y * 16, 16, 16, fwd_mv);
                                bdsp.avg_blocks (&mut buf, bck_buf, mb_x * 16, mb_y * 16, 16, 16, bwd_mv);
                            } else if has_fwd {
                                bdsp.copy_blocks(&mut buf, fwd_buf, mb_x * 16, mb_y * 16, 16, 16, fwd_mv);
                            } else {
                                bdsp.copy_blocks(&mut buf, bck_buf, mb_x * 16, mb_y * 16, 16, 16, bwd_mv);
                            }
                        }
                    } else {
                        if let BlockMVInfo::Inter_4MV(mvs) = ref_mv_info {
                            for blk_no in 0..4 {
                                let ref_mv = mvs[blk_no];
                                let ref_mv_fwd = ref_mv.scale(bsdiff, tsdiff);
                                let ref_mv_bwd = ref_mv - ref_mv_fwd;
                                let xoff = mb_x * 16 + (blk_no & 1) * 8;
                                let yoff = mb_y * 16 + (blk_no & 2) * 4;
                                if let (Some(ref fwd_buf), Some(ref bck_buf)) = (self.ipbs.get_nextref(), self.ipbs.get_lastref()) {
                                    bdsp.copy_blocks(&mut buf, fwd_buf, xoff, yoff, 8, 8, ref_mv_fwd);
                                    bdsp.avg_blocks (&mut buf, bck_buf, xoff, yoff, 8, 8, ref_mv_bwd);
                                }
                            }
                        } else {
                            let ref_mv = if let BlockMVInfo::Inter_1MV(mv_) = ref_mv_info { mv_ } else { ZERO_MV };
                            let ref_mv_fwd = ref_mv.scale(bsdiff, tsdiff);
                            let ref_mv_bwd = MV::b_sub(ref_mv, ref_mv_fwd, ZERO_MV, bsdiff, tsdiff);

                            if let (Some(ref fwd_buf), Some(ref bck_buf)) = (self.ipbs.get_nextref(), self.ipbs.get_lastref()) {
                                bdsp.copy_blocks(&mut buf, fwd_buf, mb_x * 16, mb_y * 16, 16, 16, ref_mv_fwd);
                                bdsp.avg_blocks (&mut buf, bck_buf, mb_x * 16, mb_y * 16, 16, 16, ref_mv_bwd);
                            }
                        }
                        mvi.set_zero_mv(mb_x);
                        mvi2.set_zero_mv(mb_x);
                    }
                    if cbp != 0 {
                        for i in 0..6 {
                            bd.decode_block_inter(&binfo, &sstate, binfo.get_q(), i, ((cbp >> (5 - i)) & 1) != 0, &mut blk[i])?;
                            bdsp.idct(&mut blk[i]);
                        }
                        blockdsp::add_blocks(&mut buf, mb_x, mb_y, &blk);
                    }
                }
                if pinfo.is_pb() {
                    let mut b_mb = BMB::new();
                    let cbp = binfo.get_cbp_b();
                    let bq = (((pinfo.get_pbinfo().get_dbquant() + 5) as u16) * (binfo.get_q() as u16)) >> 2;
                    let bquant;
                    if bq < 1 { bquant = 1; }
                    else if bq > 31 { bquant = 31; }
                    else { bquant = bq as u8; }

                    b_mb.cbp = cbp;
                    for i in 0..6 {
                        bd.decode_block_inter(&binfo, &sstate, bquant, i, (cbp & (1 << (5 - i))) != 0, &mut b_mb.blk[i])?;
                        bdsp.idct(&mut b_mb.blk[i]);
                    }

                    let is_fwd = !binfo.is_b_fwd();
                    b_mb.fwd = is_fwd;
                    b_mb.num_mv = binfo.get_num_mvs();
                    if binfo.get_num_mvs() == 0 {
                        b_mb.num_mv = 1;
                        b_mb.mv_f[0] = binfo.get_mv2(1);
                        b_mb.mv_b[0] = binfo.get_mv2(0);
                    } else if binfo.get_num_mvs() == 1 {
                        let src_mv = binfo.get_mv(0).scale(bsdiff, tsdiff);
                        let mv_f = MV::add_umv(src_mv, binfo.get_mv2(0), pinfo.get_mvmode());
                        let mv_b = MV::b_sub(binfo.get_mv(0), mv_f, binfo.get_mv2(0), bsdiff, tsdiff);
                        b_mb.mv_f[0] = mv_f;
                        b_mb.mv_b[0] = mv_b;
                    } else {
                        for blk_no in 0..4 {
                            let src_mv = binfo.get_mv(blk_no).scale(bsdiff, tsdiff);
                            let mv_f = MV::add_umv(src_mv, binfo.get_mv2(0), pinfo.get_mvmode());
                            let mv_b = MV::b_sub(binfo.get_mv(blk_no), mv_f, binfo.get_mv2(0), bsdiff, tsdiff);
                            b_mb.mv_f[blk_no] = mv_f;
                            b_mb.mv_b[blk_no] = mv_b;
                        }
                    }
                    self.b_data.push(b_mb);
                }
                sstate.next_mb();
                mb_pos += 1;
            }
            if let Some(plusinfo) = pinfo.plusinfo {
                if plusinfo.deblock {
                    bdsp.filter_row(&mut buf, mb_y, self.mb_w, &cbpi);
                }
            }
            mvi.update_row();
            if is_b {
                mvi2.update_row();
            }
            cbpi.update_row();
            sstate.new_row();
        }

        if pinfo.mode.is_ref() {
            self.ipbs.add_frame(buf);
            self.next_ts = self.last_ts;
            self.last_ts = pinfo.ts;
            self.tsdiff  = tsdiff;
        }

        Ok(bufinfo)
    }

    pub fn get_bframe(&mut self, bdsp: &BlockDSP) -> DecoderResult<NABufferType> {
        if !self.has_b || !self.ipbs.get_lastref().is_some() || !self.ipbs.get_nextref().is_some() {
            return Err(DecoderError::MissingReference);
        }
        self.has_b = false;

        let fmt = formats::YUV420_FORMAT;
        let vinfo = NAVideoInfo::new(self.w, self.h, false, fmt);
        let bufret = alloc_video_buffer(vinfo, 4);
        if let Err(_) = bufret { return Err(DecoderError::InvalidData); }
        let mut bufinfo = bufret.unwrap();
        let mut b_buf = bufinfo.get_vbuf().unwrap();

        if let (Some(ref bck_buf), Some(ref fwd_buf)) = (self.ipbs.get_nextref(), self.ipbs.get_lastref()) {
            recon_b_frame(&mut b_buf, bck_buf, fwd_buf, self.mb_w, self.mb_h, &self.b_data, bdsp);
        }

        self.b_data.truncate(0);
        Ok(bufinfo)
    }
}

fn recon_b_frame(b_buf: &mut NAVideoBuffer<u8>, bck_buf: &NAVideoBuffer<u8>, fwd_buf: &NAVideoBuffer<u8>,
                 mb_w: usize, mb_h: usize, b_data: &Vec<BMB>, bdsp: &BlockDSP) {
    let mut cbpi = CBPInfo::new();
    let mut cur_mb = 0;
    cbpi.reset(mb_w);
    for mb_y in 0..mb_h {
        for mb_x in 0..mb_w {
            let num_mv = b_data[cur_mb].num_mv;
            let is_fwd = b_data[cur_mb].fwd;
            let cbp    = b_data[cur_mb].cbp;
            cbpi.set_cbp(mb_x, cbp);
            if num_mv == 1 {
                bdsp.copy_blocks(b_buf, fwd_buf, mb_x * 16, mb_y * 16, 16, 16, b_data[cur_mb].mv_b[0]);
                if !is_fwd {
                    bdsp.avg_blocks(b_buf, bck_buf, mb_x * 16, mb_y * 16, 16, 16, b_data[cur_mb].mv_f[0]);
                }
            } else {
                for blk_no in 0..4 {
                    let xpos = mb_x * 16 + (blk_no & 1) * 8;
                    let ypos = mb_y * 16 + (blk_no & 2) * 4;
                    bdsp.copy_blocks(b_buf, fwd_buf, xpos, ypos, 8, 8, b_data[cur_mb].mv_b[blk_no]);
                    if !is_fwd {
                        bdsp.avg_blocks(b_buf, bck_buf, xpos, ypos, 8, 8, b_data[cur_mb].mv_f[blk_no]);
                    }
                }
            }
            if cbp != 0 {
                blockdsp::add_blocks(b_buf, mb_x, mb_y, &b_data[cur_mb].blk);
            }
            cur_mb += 1;
        }
        cbpi.update_row();
    }
}
