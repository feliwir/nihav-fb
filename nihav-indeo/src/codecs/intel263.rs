use std::rc::Rc;
use std::cell::RefCell;
use nihav_core::io::bitreader::*;
use nihav_core::io::codebook::*;
use nihav_core::formats;
use nihav_core::frame::*;
use nihav_core::codecs::*;
use nihav_core::codecs::h263::*;
use nihav_core::codecs::h263::decoder::*;
use nihav_core::codecs::h263::data::*;
use nihav_core::codecs::h263::code::H263BlockDSP;

#[allow(dead_code)]
struct Tables {
    intra_mcbpc_cb: Codebook<u8>,
    inter_mcbpc_cb: Codebook<u8>,
    cbpy_cb:        Codebook<u8>,
    rl_cb:          Codebook<H263RLSym>,
    aic_rl_cb:      Codebook<H263RLSym>,
    mv_cb:          Codebook<u8>,
}

struct Intel263Decoder {
    info:    Rc<NACodecInfo>,
    dec:     H263BaseDecoder,
    tables:  Tables,
    bdsp:    H263BlockDSP,
}

struct Intel263BR<'a> {
    br:     BitReader<'a>,
    tables: &'a Tables,
    gob_no: usize,
    mb_w:   usize,
    is_pb:  bool,
    is_ipb: bool,
}

fn check_marker<'a>(br: &mut BitReader<'a>) -> DecoderResult<()> {
    let mark = br.read(1)?;
    validate!(mark == 1);
    Ok(())
}

impl<'a> Intel263BR<'a> {
    fn new(src: &'a [u8], tables: &'a Tables) -> Self {
        Intel263BR {
            br:     BitReader::new(src, src.len(), BitReaderMode::BE),
            tables: tables,
            gob_no: 0,
            mb_w:   0,
            is_pb:  false,
            is_ipb: false,
        }
    }

    fn decode_block(&mut self, quant: u8, intra: bool, coded: bool, blk: &mut [i16; 64]) -> DecoderResult<()> {
        let br = &mut self.br;
        let mut idx = 0;
        if intra {
            let mut dc = br.read(8)?;
            if dc == 255 { dc = 128; }
            blk[0] = (dc as i16) << 3;
            idx = 1;
        }
        if !coded { return Ok(()); }

        let rl_cb = &self.tables.rl_cb; // could be aic too
        let q_add = if quant == 0 { 0i16 } else { ((quant - 1) | 1) as i16 };
        let q = (quant * 2) as i16;
        while idx < 64 {
            let code = br.read_cb(rl_cb)?;
            let run;
            let mut level;
            let last;
            if !code.is_escape() {
                run   = code.get_run();
                level = code.get_level();
                last  = code.is_last();
                if br.read_bool()? { level = -level; }
                level = (level * q) + q_add;
            } else {
                last  = br.read_bool()?;
                run   = br.read(6)? as u8;
                level = br.read_s(8)? as i16;
                if level == -128 {
                    let low = br.read(5)? as i16;
                    let top = br.read_s(6)? as i16;
                    level = (top << 5) | low;
                }
                level = (level * q) + q_add;
                if level < -2048 { level = -2048; }
                if level >  2047 { level =  2047; }
            }
            idx += run;
            validate!(idx < 64);
            let oidx = H263_ZIGZAG[idx as usize];
            blk[oidx] = level;
            idx += 1;
            if last { break; }
        }
        Ok(())
    }
}

fn decode_mv_component(br: &mut BitReader, mv_cb: &Codebook<u8>) -> DecoderResult<i16> {
    let code = br.read_cb(mv_cb)? as i16;
    if code == 0 { return Ok(0) }
    if !br.read_bool()? {
        Ok(code)
    } else {
        Ok(-code)
    }
}

fn decode_mv(br: &mut BitReader, mv_cb: &Codebook<u8>) -> DecoderResult<MV> {
    let xval = decode_mv_component(br, mv_cb)?;
    let yval = decode_mv_component(br, mv_cb)?;
    Ok(MV::new(xval, yval))
}

fn decode_b_info(br: &mut BitReader, is_pb: bool, is_ipb: bool, is_intra: bool) -> DecoderResult<BBlockInfo> {
    if is_pb { // as improved pb
        if is_ipb {
            let pb_mv_add = if is_intra { 1 } else { 0 };
            if br.read_bool()?{
                if br.read_bool()? {
                    let pb_mv_count = 1 - (br.read(1)? as usize);
                    let cbpb = br.read(6)? as u8;
                    Ok(BBlockInfo::new(true, cbpb, pb_mv_count + pb_mv_add, pb_mv_count == 1))
                } else {
                    Ok(BBlockInfo::new(true, 0, 1 + pb_mv_add, true))
                }
            } else {
                Ok(BBlockInfo::new(true, 0, pb_mv_add, false))
            }
        } else {
            let mvdb = br.read_bool()?;
            let cbpb = if mvdb && br.read_bool()? { br.read(6)? as u8 } else { 0 };
            Ok(BBlockInfo::new(true, cbpb, if mvdb { 1 } else { 0 }, false))
        }
    } else {
        Ok(BBlockInfo::new(false, 0, 0, false))
    }
}

impl<'a> BlockDecoder for Intel263BR<'a> {

#[allow(unused_variables)]
    fn decode_pichdr(&mut self) -> DecoderResult<PicInfo> {
        let br = &mut self.br;
        let syncw = br.read(22)?;
        validate!(syncw == 0x000020);
        let tr = (br.read(8)? << 8) as u16;
        check_marker(br)?;
        let id = br.read(1)?;
        validate!(id == 0);
        br.read(1)?; // split screen indicator
        br.read(1)?; // document camera indicator
        br.read(1)?; // freeze picture release
        let mut sfmt = br.read(3)?;
        validate!((sfmt != 0b000) && (sfmt != 0b110));
        let is_intra = !br.read_bool()?;
        let umv = br.read_bool()?;
        br.read(1)?; // syntax arithmetic coding
        let apm = br.read_bool()?;
        self.is_pb = br.read_bool()?;
        let deblock;
        let pbplus;
        if sfmt == 0b111 {
            sfmt = br.read(3)?;
            validate!((sfmt != 0b000) && (sfmt != 0b111));
            br.read(2)?; // unknown flags
            deblock = br.read_bool()?;
            br.read(1)?; // unknown flag
            pbplus = br.read_bool()?;
            br.read(5)?; // unknown flags
            let marker = br.read(5)?;
            validate!(marker == 1);
        } else {
            deblock = false;
            pbplus = false;
        }
        self.is_ipb = pbplus;
        let w; let h;
        if sfmt == 0b110 {
            let par = br.read(4)?;
            w = ((br.read(9)? + 1) * 4) as usize;
            check_marker(br)?;
            h = ((br.read(9)? + 1) * 4) as usize;
            if par == 0b1111 {
                let pixw = br.read(8)?;
                let pixh = br.read(8)?;
                validate!((pixw != 0) && (pixh != 0));
            }
        } else {
            let (w_, h_) = H263_SIZES[sfmt as usize];
            w = w_;
            h = h_;
        }
        let quant = br.read(5)?;
        let cpm = br.read_bool()?;
        validate!(!cpm);

        let pbinfo;
        if self.is_pb {
            let trb = br.read(3)?;
            let dbquant = br.read(2)?;
            pbinfo = Some(PBInfo::new(trb as u8, dbquant as u8, pbplus));
        } else {
            pbinfo = None;
        }
        while br.read_bool()? { // skip PEI
            br.read(8)?;
        }
//println!("frame {}x{} intra: {} q {} pb {} apm {} umv {} @{}", w, h, is_intra, quant, self.is_pb, apm, umv, br.tell());
        self.gob_no = 0;
        self.mb_w = (w + 15) >> 4;

        let ftype = if is_intra { Type::I } else { Type::P };
        let plusinfo = if deblock { Some(PlusInfo::new(false, deblock, false, false)) } else { None };
        let mvmode = if umv { MVMode::UMV } else { MVMode::Old };
        let picinfo = PicInfo::new(w, h, ftype, mvmode, umv, apm, quant as u8, tr, pbinfo, plusinfo);
        Ok(picinfo)
    }

    #[allow(unused_variables)]
    fn decode_slice_header(&mut self, info: &PicInfo) -> DecoderResult<SliceInfo> {
        let br = &mut self.br;
        let gbsc = br.read(17)?;
        validate!(gbsc == 1);
        let gn = br.read(5)?;
        let gfid = br.read(2)?;
        let gquant = br.read(5)?;
//println!("GOB gn {:X} id {} q {}", gn, gfid, gquant);
        let ret = SliceInfo::new_gob(0, self.gob_no, gquant as u8);
        self.gob_no += 1;
        Ok(ret)
    }

    #[allow(unused_variables)]
    fn decode_block_header(&mut self, info: &PicInfo, slice: &SliceInfo, sstate: &SliceState) -> DecoderResult<BlockInfo> {
        let br = &mut self.br;
        let mut q = slice.get_quant();
        match info.get_mode() {
            Type::I => {
                    let mut cbpc = br.read_cb(&self.tables.intra_mcbpc_cb)?;
                    while cbpc == 8 { cbpc = br.read_cb(&self.tables.intra_mcbpc_cb)?; }
                    let cbpy = br.read_cb(&self.tables.cbpy_cb)?;
                    let cbp = (cbpy << 2) | (cbpc & 3);
                    let dquant = (cbpc & 4) != 0;
                    if dquant {
                        let idx = br.read(2)? as usize;
                        q = ((q as i16) + (H263_DQUANT_TAB[idx] as i16)) as u8;
                    }
                    Ok(BlockInfo::new(Type::I, cbp, q))
                },
            Type::P => {
                    if br.read_bool()? { return Ok(BlockInfo::new(Type::Skip, 0, info.get_quant())); }
                    let mut cbpc = br.read_cb(&self.tables.inter_mcbpc_cb)?;
                    while cbpc == 20 { cbpc = br.read_cb(&self.tables.inter_mcbpc_cb)?; }
                    let is_intra = (cbpc & 0x04) != 0;
                    let dquant   = (cbpc & 0x08) != 0;
                    let is_4x4   = (cbpc & 0x10) != 0;
                    if is_intra {
                        let mut mvec: Vec<MV> = Vec::new();
                        let bbinfo = decode_b_info(br, self.is_pb, self.is_ipb, true)?;
                        let cbpy = br.read_cb(&self.tables.cbpy_cb)?;
                        let cbp = (cbpy << 2) | (cbpc & 3);
                        if dquant {
                            let idx = br.read(2)? as usize;
                            q = ((q as i16) + (H263_DQUANT_TAB[idx] as i16)) as u8;
                        }
                        let mut binfo = BlockInfo::new(Type::I, cbp, q);
                        binfo.set_bpart(bbinfo);
                        if self.is_pb {
                            for _ in 0..bbinfo.get_num_mv() {
                                mvec.push(decode_mv(br, &self.tables.mv_cb)?);
                            }
                            binfo.set_b_mv(mvec.as_slice());
                        }
                        return Ok(binfo);
                    }

                    let bbinfo = decode_b_info(br, self.is_pb, self.is_ipb, false)?;
                    let mut cbpy = br.read_cb(&self.tables.cbpy_cb)?;
//                    if /* !aiv && */(cbpc & 3) != 3 {
                        cbpy ^= 0xF;
//                    }
                    let cbp = (cbpy << 2) | (cbpc & 3);
                    if dquant {
                        let idx = br.read(2)? as usize;
                        q = ((q as i16) + (H263_DQUANT_TAB[idx] as i16)) as u8;
                    }
                    let mut binfo = BlockInfo::new(Type::P, cbp, q);
                    binfo.set_bpart(bbinfo);
                    if !is_4x4 {
                        let mvec: [MV; 1] = [decode_mv(br, &self.tables.mv_cb)?];
                        binfo.set_mv(&mvec);
                    } else {
                        let mvec: [MV; 4] = [
                                decode_mv(br, &self.tables.mv_cb)?,
                                decode_mv(br, &self.tables.mv_cb)?,
                                decode_mv(br, &self.tables.mv_cb)?,
                                decode_mv(br, &self.tables.mv_cb)?
                            ];
                        binfo.set_mv(&mvec);
                    }
                    if self.is_pb {
                        let mut mvec: Vec<MV> = Vec::with_capacity(bbinfo.get_num_mv());
                        for _ in 0..bbinfo.get_num_mv() {
                            let mv = decode_mv(br, &self.tables.mv_cb)?;
                            mvec.push(mv);
                        }
                        binfo.set_b_mv(mvec.as_slice());
                    }
                    Ok(binfo)
                },
            _ => { Err(DecoderError::InvalidData) },
        }
    }

    #[allow(unused_variables)]
    fn decode_block_intra(&mut self, info: &BlockInfo, sstate: &SliceState, quant: u8, no: usize, coded: bool, blk: &mut [i16; 64]) -> DecoderResult<()> {
        self.decode_block(quant, true, coded, blk)
    }

    #[allow(unused_variables)]
    fn decode_block_inter(&mut self, info: &BlockInfo, sstate: &SliceState, quant: u8, no: usize, coded: bool, blk: &mut [i16; 64]) -> DecoderResult<()> {
        self.decode_block(quant, false, coded, blk)
    }

    fn is_slice_end(&mut self) -> bool { self.br.peek(16) == 0 }
}

impl Intel263Decoder {
    fn new() -> Self {
        let mut coderead = H263ShortCodeReader::new(H263_INTRA_MCBPC);
        let intra_mcbpc_cb = Codebook::new(&mut coderead, CodebookMode::MSB).unwrap();
        let mut coderead = H263ShortCodeReader::new(H263_INTER_MCBPC);
        let inter_mcbpc_cb = Codebook::new(&mut coderead, CodebookMode::MSB).unwrap();
        let mut coderead = H263ShortCodeReader::new(H263_CBPY);
        let cbpy_cb = Codebook::new(&mut coderead, CodebookMode::MSB).unwrap();
        let mut coderead = H263RLCodeReader::new(H263_RL_CODES);
        let rl_cb = Codebook::new(&mut coderead, CodebookMode::MSB).unwrap();
        let mut coderead = H263RLCodeReader::new(H263_RL_CODES_AIC);
        let aic_rl_cb = Codebook::new(&mut coderead, CodebookMode::MSB).unwrap();
        let mut coderead = H263ShortCodeReader::new(H263_MV);
        let mv_cb = Codebook::new(&mut coderead, CodebookMode::MSB).unwrap();
        let tables = Tables {
            intra_mcbpc_cb: intra_mcbpc_cb,
            inter_mcbpc_cb: inter_mcbpc_cb,
            cbpy_cb:        cbpy_cb,
            rl_cb:          rl_cb,
            aic_rl_cb:      aic_rl_cb,
            mv_cb:          mv_cb,
        };

        Intel263Decoder{
            info:           Rc::new(DUMMY_CODEC_INFO),
            dec:            H263BaseDecoder::new(true),
            tables:         tables,
            bdsp:           H263BlockDSP::new(),
        }
    }
}

impl NADecoder for Intel263Decoder {
    fn init(&mut self, info: Rc<NACodecInfo>) -> DecoderResult<()> {
        if let NACodecTypeInfo::Video(vinfo) = info.get_properties() {
            let w = vinfo.get_width();
            let h = vinfo.get_height();
            let fmt = formats::YUV420_FORMAT;
            let myinfo = NACodecTypeInfo::Video(NAVideoInfo::new(w, h, false, fmt));
            self.info = Rc::new(NACodecInfo::new_ref(info.get_name(), myinfo, info.get_extradata()));
            Ok(())
        } else {
            Err(DecoderError::InvalidData)
        }
    }
    fn decode(&mut self, pkt: &NAPacket) -> DecoderResult<NAFrameRef> {
        let src = pkt.get_buffer();

        if src.len() == 8 {
            let bret = self.dec.get_bframe(&self.bdsp);
            let buftype;
            let is_skip;
            if let Ok(btype) = bret {
                buftype = btype;
                is_skip = false;
            } else {
                buftype = NABufferType::None;
                is_skip = true;
            }
            let mut frm = NAFrame::new_from_pkt(pkt, self.info.clone(), buftype);
            frm.set_keyframe(false);
            frm.set_frame_type(if is_skip { FrameType::Skip } else { FrameType::B });
            return Ok(Rc::new(RefCell::new(frm)));
        }
        let mut ibr = Intel263BR::new(&src, &self.tables);

        let bufinfo = self.dec.parse_frame(&mut ibr, &self.bdsp)?;

        let mut frm = NAFrame::new_from_pkt(pkt, self.info.clone(), bufinfo);
        frm.set_keyframe(self.dec.is_intra());
        frm.set_frame_type(if self.dec.is_intra() { FrameType::I } else { FrameType::P });
        Ok(Rc::new(RefCell::new(frm)))
    }
}


pub fn get_decoder() -> Box<NADecoder> {
    Box::new(Intel263Decoder::new())
}

#[cfg(test)]
mod test {
    use nihav_core::codecs::RegisteredDecoders;
    use nihav_core::demuxers::RegisteredDemuxers;
    use nihav_core::test::dec_video::*;
    use crate::codecs::indeo_register_all_codecs;
    use nihav_commonfmt::demuxers::generic_register_all_demuxers;
    #[test]
    fn test_intel263() {
        let mut dmx_reg = RegisteredDemuxers::new();
        generic_register_all_demuxers(&mut dmx_reg);
        let mut dec_reg = RegisteredDecoders::new();
        indeo_register_all_codecs(&mut dec_reg);

        test_file_decoding("avi", "assets/neal73_saber.avi", Some(16), true, false, None/*Some("i263")*/, &dmx_reg, &dec_reg);
    }
}
