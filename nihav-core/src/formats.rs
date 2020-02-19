//! Audio and image sample format definitions.
//!
//! NihAV does not have a fixed list of supported formats but rather accepts format definitions both for audio and video.
//! In result exotic formats like YUV410+alpha plane that is used by Indeo 4 are supported without any additional case handing.
//! Some common format definitions are provided as constants for convenience.
use std::str::FromStr;
use std::string::*;
use std::fmt;

/// Audio format definition.
///
/// The structure describes how audio samples are stored and what characteristics they have.
#[derive(Debug,Copy,Clone,PartialEq)]
pub struct NASoniton {
    /// Bits per sample.
    pub bits:       u8,
    /// Audio format is big-endian.
    pub be:         bool,
    /// Audio samples are packed (e.g. 20-bit audio samples).
    pub packed:     bool,
    /// Audio data is stored in planar format instead of interleaving samples for different channels.
    pub planar:     bool,
    /// Audio data is in floating point format.
    pub float:      bool,
    /// Audio data is signed (usually only 8-bit audio is unsigned).
    pub signed:     bool,
}

/// Flag for specifying that audio format is big-endian in `NASoniton::`[`new`]`()`. Related to [`be`] field of `NASoniton`.
///
/// [`new`]: ./struct.NASoniton.html#method.new
/// [`be`]: ./struct.NASoniton.html#structfield.be
pub const SONITON_FLAG_BE     :u32 = 0x01;
/// Flag for specifying that audio format has packed samples in `NASoniton::`[`new`]`()`. Related to [`packed`] field of `NASoniton`.
///
/// [`new`]: ./struct.NASoniton.html#method.new
/// [`packed`]: ./struct.NASoniton.html#structfield.packed
pub const SONITON_FLAG_PACKED :u32 = 0x02;
/// Flag for specifying that audio data is stored as planar in `NASoniton::`[`new`]`()`. Related to [`planar`] field of `NASoniton`.
///
/// [`new`]: ./struct.NASoniton.html#method.new
/// [`planar`]: ./struct.NASoniton.html#structfield.planar
pub const SONITON_FLAG_PLANAR :u32 = 0x04;
/// Flag for specifying that audio samples are in floating point format in `NASoniton::`[`new`]`()`. Related to [`float`] field of `NASoniton`.
///
/// [`new`]: ./struct.NASoniton.html#method.new
/// [`float`]: ./struct.NASoniton.html#structfield.float
pub const SONITON_FLAG_FLOAT  :u32 = 0x08;
/// Flag for specifying that audio format is signed in `NASoniton::`[`new`]`()`. Related to [`signed`] field of `NASoniton`.
///
/// [`new`]: ./struct.NASoniton.html#method.new
/// [`signed`]: ./struct.NASoniton.html#structfield.signed
pub const SONITON_FLAG_SIGNED :u32 = 0x10;

/// Predefined format for interleaved 8-bit unsigned audio.
pub const SND_U8_FORMAT: NASoniton = NASoniton { bits: 8, be: false, packed: false, planar: false, float: false, signed: false };
/// Predefined format for interleaved 16-bit signed audio.
pub const SND_S16_FORMAT: NASoniton = NASoniton { bits: 16, be: false, packed: false, planar: false, float: false, signed: true };
/// Predefined format for planar 16-bit signed audio.
pub const SND_S16P_FORMAT: NASoniton = NASoniton { bits: 16, be: false, packed: false, planar: true, float: false, signed: true };
/// Predefined format for planar 32-bit floating point audio.
pub const SND_F32P_FORMAT: NASoniton = NASoniton { bits: 32, be: false, packed: false, planar: true, float: true, signed: true };

impl NASoniton {
    /// Constructs a new audio format definition using flags like [`SONITON_FLAG_BE`].
    ///
    /// [`SONITON_FLAG_BE`]: ./constant.SONITON_FLAG_BE.html
    pub fn new(bits: u8, flags: u32) -> Self {
        let is_be = (flags & SONITON_FLAG_BE) != 0;
        let is_pk = (flags & SONITON_FLAG_PACKED) != 0;
        let is_pl = (flags & SONITON_FLAG_PLANAR) != 0;
        let is_fl = (flags & SONITON_FLAG_FLOAT) != 0;
        let is_sg = (flags & SONITON_FLAG_SIGNED) != 0;
        NASoniton { bits, be: is_be, packed: is_pk, planar: is_pl, float: is_fl, signed: is_sg }
    }

    /// Returns the number of bits per sample.
    pub fn get_bits(self)   -> u8   { self.bits }
    /// Reports whether the format is big-endian.
    pub fn is_be(self)      -> bool { self.be }
    /// Reports whether the format has packed samples.
    pub fn is_packed(self)  -> bool { self.packed }
    /// Reports whether audio data is planar instead of interleaved.
    pub fn is_planar(self)  -> bool { self.planar }
    /// Reports whether audio samples are in floating point format.
    pub fn is_float(self)   -> bool { self.float }
    /// Reports whether audio samples are signed.
    pub fn is_signed(self)  -> bool { self.signed }

    /// Returns the amount of bytes needed to store the audio of requested length (in samples).
    pub fn get_audio_size(self, length: u64) -> usize {
        if self.packed {
            ((length * u64::from(self.bits) + 7) >> 3) as usize
        } else {
            (length * u64::from((self.bits + 7) >> 3)) as usize
        }
    }
}

impl fmt::Display for NASoniton {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fmt = if self.float { "float" } else if self.signed { "int" } else { "uint" };
        let end = if self.be { "BE" } else { "LE" };
        write!(f, "({} bps, {} planar: {} packed: {} {})", self.bits, end, self.packed, self.planar, fmt)
    }
}

/// Known channel types.
#[derive(Debug,Clone,Copy,PartialEq)]
pub enum NAChannelType {
    C, L, R, Cs, Ls, Rs, Lss, Rss, LFE, Lc, Rc, Lh, Rh, Ch, LFE2, Lw, Rw, Ov, Lhs, Rhs, Chs, Ll, Rl, Cl, Lt, Rt, Lo, Ro
}

impl NAChannelType {
    /// Reports whether this is some center channel.
    pub fn is_center(self) -> bool {
        match self {
            NAChannelType::C => true,   NAChannelType::Ch => true,
            NAChannelType::Cl => true,  NAChannelType::Ov => true,
            NAChannelType::LFE => true, NAChannelType::LFE2 => true,
            NAChannelType::Cs => true,  NAChannelType::Chs => true,
            _ => false,
        }
    }
    /// Reports whether this is some left channel.
    pub fn is_left(self) -> bool {
        match self {
            NAChannelType::L   => true, NAChannelType::Ls => true,
            NAChannelType::Lss => true, NAChannelType::Lc => true,
            NAChannelType::Lh  => true, NAChannelType::Lw => true,
            NAChannelType::Lhs => true, NAChannelType::Ll => true,
            NAChannelType::Lt  => true, NAChannelType::Lo => true,
            _ => false,
        }
    }
    /// Reports whether this is some right channel.
    pub fn is_right(self) -> bool {
        match self {
            NAChannelType::R   => true, NAChannelType::Rs => true,
            NAChannelType::Rss => true, NAChannelType::Rc => true,
            NAChannelType::Rh  => true, NAChannelType::Rw => true,
            NAChannelType::Rhs => true, NAChannelType::Rl => true,
            NAChannelType::Rt  => true, NAChannelType::Ro => true,
            _ => false,
        }
    }
}

/// Generic channel configuration parsing error.
#[derive(Clone,Copy,Debug,PartialEq)]
pub struct ChannelParseError {}

impl FromStr for NAChannelType {
    type Err = ChannelParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "C"     => Ok(NAChannelType::C),
            "L"     => Ok(NAChannelType::L),
            "R"     => Ok(NAChannelType::R),
            "Cs"    => Ok(NAChannelType::Cs),
            "Ls"    => Ok(NAChannelType::Ls),
            "Rs"    => Ok(NAChannelType::Rs),
            "Lss"   => Ok(NAChannelType::Lss),
            "Rss"   => Ok(NAChannelType::Rss),
            "LFE"   => Ok(NAChannelType::LFE),
            "Lc"    => Ok(NAChannelType::Lc),
            "Rc"    => Ok(NAChannelType::Rc),
            "Lh"    => Ok(NAChannelType::Lh),
            "Rh"    => Ok(NAChannelType::Rh),
            "Ch"    => Ok(NAChannelType::Ch),
            "LFE2"  => Ok(NAChannelType::LFE2),
            "Lw"    => Ok(NAChannelType::Lw),
            "Rw"    => Ok(NAChannelType::Rw),
            "Ov"    => Ok(NAChannelType::Ov),
            "Lhs"   => Ok(NAChannelType::Lhs),
            "Rhs"   => Ok(NAChannelType::Rhs),
            "Chs"   => Ok(NAChannelType::Chs),
            "Ll"    => Ok(NAChannelType::Ll),
            "Rl"    => Ok(NAChannelType::Rl),
            "Cl"    => Ok(NAChannelType::Cl),
            "Lt"    => Ok(NAChannelType::Lt),
            "Rt"    => Ok(NAChannelType::Rt),
            "Lo"    => Ok(NAChannelType::Lo),
            "Ro"    => Ok(NAChannelType::Ro),
            _   => Err(ChannelParseError{}),
        }
    }
}

impl fmt::Display for NAChannelType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match *self {
            NAChannelType::C    => "C".to_string(),
            NAChannelType::L    => "L".to_string(),
            NAChannelType::R    => "R".to_string(),
            NAChannelType::Cs   => "Cs".to_string(),
            NAChannelType::Ls   => "Ls".to_string(),
            NAChannelType::Rs   => "Rs".to_string(),
            NAChannelType::Lss  => "Lss".to_string(),
            NAChannelType::Rss  => "Rss".to_string(),
            NAChannelType::LFE  => "LFE".to_string(),
            NAChannelType::Lc   => "Lc".to_string(),
            NAChannelType::Rc   => "Rc".to_string(),
            NAChannelType::Lh   => "Lh".to_string(),
            NAChannelType::Rh   => "Rh".to_string(),
            NAChannelType::Ch   => "Ch".to_string(),
            NAChannelType::LFE2 => "LFE2".to_string(),
            NAChannelType::Lw   => "Lw".to_string(),
            NAChannelType::Rw   => "Rw".to_string(),
            NAChannelType::Ov   => "Ov".to_string(),
            NAChannelType::Lhs  => "Lhs".to_string(),
            NAChannelType::Rhs  => "Rhs".to_string(),
            NAChannelType::Chs  => "Chs".to_string(),
            NAChannelType::Ll   => "Ll".to_string(),
            NAChannelType::Rl   => "Rl".to_string(),
            NAChannelType::Cl   => "Cl".to_string(),
            NAChannelType::Lt   => "Lt".to_string(),
            NAChannelType::Rt   => "Rt".to_string(),
            NAChannelType::Lo   => "Lo".to_string(),
            NAChannelType::Ro   => "Ro".to_string(),
        };
        write!(f, "{}", name)
    }
}

/// Channel map.
///
/// This is essentially an ordered sequence of channels.
#[derive(Clone,Default)]
pub struct NAChannelMap {
    ids: Vec<NAChannelType>,
}

const MS_CHANNEL_MAP: [NAChannelType; 11] = [
    NAChannelType::L,
    NAChannelType::R,
    NAChannelType::C,
    NAChannelType::LFE,
    NAChannelType::Ls,
    NAChannelType::Rs,
    NAChannelType::Lss,
    NAChannelType::Rss,
    NAChannelType::Cs,
    NAChannelType::Lc,
    NAChannelType::Rc,
];

impl NAChannelMap {
    /// Constructs a new `NAChannelMap` instance.
    pub fn new() -> Self { NAChannelMap { ids: Vec::new() } }
    /// Adds a new channel to the map.
    pub fn add_channel(&mut self, ch: NAChannelType) {
        self.ids.push(ch);
    }
    /// Adds several channels to the map at once.
    pub fn add_channels(&mut self, chs: &[NAChannelType]) {
        for e in chs.iter() {
            self.ids.push(*e);
        }
    }
    /// Returns the total number of channels.
    pub fn num_channels(&self) -> usize {
        self.ids.len()
    }
    /// Reports channel type for a requested index.
    pub fn get_channel(&self, idx: usize) -> NAChannelType {
        self.ids[idx]
    }
    /// Tries to find position of the channel with requested type.
    pub fn find_channel_id(&self, t: NAChannelType) -> Option<u8> {
        for i in 0..self.ids.len() {
            if self.ids[i] as i32 == t as i32 { return Some(i as u8); }
        }
        None
    }
    /// Creates a new `NAChannelMap` using the channel mapping flags from WAVE format.
    pub fn from_ms_mapping(chmap: u32) -> Self {
        let mut cm = NAChannelMap::new();
        for (i, ch) in MS_CHANNEL_MAP.iter().enumerate() {
            if ((chmap >> i) & 1) != 0 {
                cm.add_channel(*ch);
            }
        }
        cm
    }
}

impl fmt::Display for NAChannelMap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut map = String::new();
        for el in self.ids.iter() {
            if !map.is_empty() { map.push(','); }
            map.push_str(&*el.to_string());
        }
        write!(f, "{}", map)
    }
}

impl FromStr for NAChannelMap {
    type Err = ChannelParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chm = NAChannelMap::new();
        for tok in s.split(',') {
            chm.add_channel(NAChannelType::from_str(tok)?);
        }
        Ok(chm)
    }
}

/// A list of RGB colour model variants.
#[derive(Debug,Clone,Copy,PartialEq)]
pub enum RGBSubmodel {
    RGB,
    SRGB,
}

impl fmt::Display for RGBSubmodel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match *self {
            RGBSubmodel::RGB  => "RGB".to_string(),
            RGBSubmodel::SRGB => "sRGB".to_string(),
        };
        write!(f, "{}", name)
    }
}

/// A list of YUV colour model variants.
#[derive(Debug,Clone,Copy,PartialEq)]
pub enum YUVSubmodel {
    YCbCr,
    /// NTSC variant.
    YIQ,
    /// The YUV variant used by JPEG.
    YUVJ,
}

impl fmt::Display for YUVSubmodel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match *self {
            YUVSubmodel::YCbCr => "YCbCr".to_string(),
            YUVSubmodel::YIQ   => "YIQ".to_string(),
            YUVSubmodel::YUVJ  => "YUVJ".to_string(),
        };
        write!(f, "{}", name)
    }
}

/// A list of known colour models.
#[derive(Debug, Clone,Copy,PartialEq)]
pub enum ColorModel {
    RGB(RGBSubmodel),
    YUV(YUVSubmodel),
    CMYK,
    HSV,
    LAB,
    XYZ,
}

impl ColorModel {
    /// Returns the number of colour model components.
    ///
    /// The actual image may have more components e.g. alpha component.
    pub fn get_default_components(self) -> usize {
        match self {
            ColorModel::CMYK => 4,
            _                => 3,
        }
    }
    /// Reports whether the current colour model is RGB.
    pub fn is_rgb(self) -> bool {
        match self {
            ColorModel::RGB(_) => true,
            _ => false,
        }
    }
    /// Reports whether the current colour model is YUV.
    pub fn is_yuv(self) -> bool {
        match self {
            ColorModel::YUV(_) => true,
            _ => false,
        }
    }
    /// Returns short name for the current colour mode.
    pub fn get_short_name(self) -> &'static str {
        match self {
            ColorModel::RGB(_)   => "rgb",
            ColorModel::YUV(_)   => "yuv",
            ColorModel::CMYK     => "cmyk",
            ColorModel::HSV      => "hsv",
            ColorModel::LAB      => "lab",
            ColorModel::XYZ      => "xyz",
        }
    }
}

impl fmt::Display for ColorModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match *self {
            ColorModel::RGB(fmt) => format!("RGB({})", fmt).to_string(),
            ColorModel::YUV(fmt) => format!("YUV({})", fmt).to_string(),
            ColorModel::CMYK     => "CMYK".to_string(),
            ColorModel::HSV      => "HSV".to_string(),
            ColorModel::LAB      => "LAB".to_string(),
            ColorModel::XYZ      => "XYZ".to_string(),
        };
        write!(f, "{}", name)
    }
}

/// Single colourspace component definition.
///
/// This structure defines how components of a colourspace are subsampled and where and how they are stored.
#[derive(Clone,Copy,PartialEq)]
pub struct NAPixelChromaton {
    /// Horizontal subsampling in power of two (e.g. `0` = no subsampling, `1` = only every second value is stored).
    pub h_ss:           u8,
    /// Vertial subsampling in power of two (e.g. `0` = no subsampling, `1` = only every second value is stored).
    pub v_ss:           u8,
    /// A flag to signal that component is packed.
    pub packed:         bool,
    /// Bit depth of current component.
    pub depth:          u8,
    /// Shift for packed components.
    pub shift:          u8,
    /// Component offset for byte-packed components.
    pub comp_offs:      u8,
    /// The distance to the next packed element in bytes.
    pub next_elem:      u8,
}

/// Flag for specifying that image data is stored big-endian in `NAPixelFormaton::`[`new`]`()`. Related to its [`be`] field.
///
/// [`new`]: ./struct.NAPixelFormaton.html#method.new
/// [`be`]: ./struct.NAPixelFormaton.html#structfield.new
pub const FORMATON_FLAG_BE      :u32 = 0x01;
/// Flag for specifying that image data has alpha plane in `NAPixelFormaton::`[`new`]`()`. Related to its [`alpha`] field.
///
/// [`new`]: ./struct.NAPixelFormaton.html#method.new
/// [`alpha`]: ./struct.NAPixelFormaton.html#structfield.alpha
pub const FORMATON_FLAG_ALPHA   :u32 = 0x02;
/// Flag for specifying that image data is stored in paletted form for `NAPixelFormaton::`[`new`]`()`. Related to its [`palette`] field.
///
/// [`new`]: ./struct.NAPixelFormaton.html#method.new
/// [`palette`]: ./struct.NAPixelFormaton.html#structfield.palette
pub const FORMATON_FLAG_PALETTE :u32 = 0x04;

/// The current limit on number of components in image colourspace model (including alpha component).
pub const MAX_CHROMATONS: usize = 5;

/// Image colourspace representation.
///
/// This structure includes both definitions for each component and some common definitions.
/// For example the format can be paletted and then components describe the palette storage format while actual data is 8-bit palette indices.
#[derive(Clone,Copy,PartialEq)]
pub struct NAPixelFormaton {
    /// Image colour model.
    pub model:      ColorModel,
    /// Actual number of components present.
    pub components: u8,
    /// Format definition for each component.
    pub comp_info:  [Option<NAPixelChromaton>; MAX_CHROMATONS],
    /// Single pixel size for packed formats.
    pub elem_size:  u8,
    /// A flag signalling that data is stored as big-endian.
    pub be:         bool,
    /// A flag signalling that image has alpha component.
    pub alpha:      bool,
    /// A flag signalling that data is paletted.
    ///
    /// This means that image data is stored as 8-bit indices (in the first image component) for the palette stored as second component of the image and actual palette format is described in this structure.
    pub palette:    bool,
}

macro_rules! chromaton {
    ($hs: expr, $vs: expr, $pck: expr, $d: expr, $sh: expr, $co: expr, $ne: expr) => ({
        Some(NAPixelChromaton{ h_ss: $hs, v_ss: $vs, packed: $pck, depth: $d, shift: $sh, comp_offs: $co, next_elem: $ne })
    });
    (yuv8; $hs: expr, $vs: expr, $co: expr) => ({
        Some(NAPixelChromaton{ h_ss: $hs, v_ss: $vs, packed: false, depth: 8, shift: 0, comp_offs: $co, next_elem: 1 })
    });
    (packrgb; $d: expr, $s: expr, $co: expr, $ne: expr) => ({
        Some(NAPixelChromaton{ h_ss: 0, v_ss: 0, packed: true, depth: $d, shift: $s, comp_offs: $co, next_elem: $ne })
    });
    (pal8; $co: expr) => ({
        Some(NAPixelChromaton{ h_ss: 0, v_ss: 0, packed: true, depth: 8, shift: 0, comp_offs: $co, next_elem: 3 })
    });
}

/// Predefined format for planar 8-bit YUV with 4:2:0 subsampling.
pub const YUV420_FORMAT: NAPixelFormaton = NAPixelFormaton { model: ColorModel::YUV(YUVSubmodel::YUVJ), components: 3,
                                        comp_info: [
                                            chromaton!(0, 0, false, 8, 0, 0, 1),
                                            chromaton!(yuv8; 1, 1, 1),
                                            chromaton!(yuv8; 1, 1, 2),
                                            None, None],
                                        elem_size: 0, be: false, alpha: false, palette: false };

/// Predefined format for planar 8-bit YUV with 4:1:0 subsampling.
pub const YUV410_FORMAT: NAPixelFormaton = NAPixelFormaton { model: ColorModel::YUV(YUVSubmodel::YUVJ), components: 3,
                                        comp_info: [
                                            chromaton!(0, 0, false, 8, 0, 0, 1),
                                            chromaton!(yuv8; 2, 2, 1),
                                            chromaton!(yuv8; 2, 2, 2),
                                            None, None],
                                        elem_size: 0, be: false, alpha: false, palette: false };
/// Predefined format for planar 8-bit YUV with 4:1:0 subsampling and alpha component.
pub const YUVA410_FORMAT: NAPixelFormaton = NAPixelFormaton { model: ColorModel::YUV(YUVSubmodel::YUVJ), components: 4,
                                        comp_info: [
                                            chromaton!(0, 0, false, 8, 0, 0, 1),
                                            chromaton!(yuv8; 2, 2, 1),
                                            chromaton!(yuv8; 2, 2, 2),
                                            chromaton!(0, 0, false, 8, 0, 3, 1),
                                            None],
                                        elem_size: 0, be: false, alpha: true, palette: false };

/// Predefined format with RGB24 palette.
pub const PAL8_FORMAT: NAPixelFormaton = NAPixelFormaton { model: ColorModel::RGB(RGBSubmodel::RGB), components: 3,
                                        comp_info: [
                                            chromaton!(pal8; 0),
                                            chromaton!(pal8; 1),
                                            chromaton!(pal8; 2),
                                            None, None],
                                        elem_size: 3, be: false, alpha: false, palette: true };

/// Predefined format for RGB565 packed video.
pub const RGB565_FORMAT: NAPixelFormaton = NAPixelFormaton { model: ColorModel::RGB(RGBSubmodel::RGB), components: 3,
                                        comp_info: [
                                            chromaton!(packrgb; 5, 11, 0, 2),
                                            chromaton!(packrgb; 6,  5, 0, 2),
                                            chromaton!(packrgb; 5,  0, 0, 2),
                                            None, None],
                                        elem_size: 2, be: false, alpha: false, palette: false };

/// Predefined format for RGB24.
pub const RGB24_FORMAT: NAPixelFormaton = NAPixelFormaton { model: ColorModel::RGB(RGBSubmodel::RGB), components: 3,
                                        comp_info: [
                                            chromaton!(packrgb; 8, 0, 0, 3),
                                            chromaton!(packrgb; 8, 0, 1, 3),
                                            chromaton!(packrgb; 8, 0, 2, 3),
                                            None, None],
                                        elem_size: 3, be: false, alpha: false, palette: false };

impl NAPixelChromaton {
    /// Constructs a new `NAPixelChromaton` instance.
    pub fn new(h_ss: u8, v_ss: u8, packed: bool, depth: u8, shift: u8, comp_offs: u8, next_elem: u8) -> Self {
        Self { h_ss, v_ss, packed, depth, shift, comp_offs, next_elem }
    }
    /// Returns subsampling for the current component.
    pub fn get_subsampling(self) -> (u8, u8) { (self.h_ss, self.v_ss) }
    /// Reports whether current component is packed.
    pub fn is_packed(self) -> bool { self.packed }
    /// Returns bit depth of current component.
    pub fn get_depth(self) -> u8   { self.depth }
    /// Returns bit shift for packed component.
    pub fn get_shift(self) -> u8   { self.shift }
    /// Returns byte offset for packed component.
    pub fn get_offset(self) -> u8  { self.comp_offs }
    /// Returns byte offset to the next element of current packed component.
    pub fn get_step(self)  -> u8   { self.next_elem }

    /// Calculates the width for current component from general image width.
    pub fn get_width(self, width: usize) -> usize {
        (width  + ((1 << self.h_ss) - 1)) >> self.h_ss
    }
    /// Calculates the height for current component from general image height.
    pub fn get_height(self, height: usize) -> usize {
        (height + ((1 << self.v_ss) - 1)) >> self.v_ss
    }
    /// Calculates the minimal stride for current component from general image width.
    pub fn get_linesize(self, width: usize) -> usize {
        let d = self.depth as usize;
        if self.packed {
            (self.get_width(width) * d + d - 1) >> 3
        } else {
            self.get_width(width)
        }
    }
    /// Calculates the required image size in pixels for current component from general image width.
    pub fn get_data_size(self, width: usize, height: usize) -> usize {
        let nh = (height + ((1 << self.v_ss) - 1)) >> self.v_ss;
        self.get_linesize(width) * nh
    }
}

impl fmt::Display for NAPixelChromaton {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let pfmt = if self.packed {
            let mask = ((1 << self.depth) - 1) << self.shift;
            format!("packed(+{},{:X}, step {})", self.comp_offs, mask, self.next_elem)
        } else {
            format!("planar({},{})", self.comp_offs, self.next_elem)
        };
        write!(f, "({}x{}, {})", self.h_ss, self.v_ss, pfmt)
    }
}

impl NAPixelFormaton {
    /// Constructs a new instance of `NAPixelFormaton`.
    pub fn new(model: ColorModel,
               comp1: Option<NAPixelChromaton>,
               comp2: Option<NAPixelChromaton>,
               comp3: Option<NAPixelChromaton>,
               comp4: Option<NAPixelChromaton>,
               comp5: Option<NAPixelChromaton>,
               flags: u32, elem_size: u8) -> Self {
        let mut chromatons: [Option<NAPixelChromaton>; MAX_CHROMATONS] = [None; MAX_CHROMATONS];
        let mut ncomp = 0;
        let be      = (flags & FORMATON_FLAG_BE)      != 0;
        let alpha   = (flags & FORMATON_FLAG_ALPHA)   != 0;
        let palette = (flags & FORMATON_FLAG_PALETTE) != 0;
        if let Some(c) = comp1 { chromatons[0] = Some(c); ncomp += 1; }
        if let Some(c) = comp2 { chromatons[1] = Some(c); ncomp += 1; }
        if let Some(c) = comp3 { chromatons[2] = Some(c); ncomp += 1; }
        if let Some(c) = comp4 { chromatons[3] = Some(c); ncomp += 1; }
        if let Some(c) = comp5 { chromatons[4] = Some(c); ncomp += 1; }
        NAPixelFormaton { model,
                          components: ncomp,
                          comp_info: chromatons,
                          elem_size,
                          be, alpha, palette }
    }

    /// Returns current colour model.
    pub fn get_model(&self) -> ColorModel { self.model }
    /// Returns the number of components.
    pub fn get_num_comp(&self) -> usize { self.components as usize }
    /// Returns selected component information.
    pub fn get_chromaton(&self, idx: usize) -> Option<NAPixelChromaton> {
        if idx < self.comp_info.len() { return self.comp_info[idx]; }
        None
    }
    /// Reports whether the packing format is big-endian.
    pub fn is_be(self) -> bool { self.be }
    /// Reports whether colourspace has alpha component.
    pub fn has_alpha(self) -> bool { self.alpha }
    /// Reports whether this is paletted format.
    pub fn is_paletted(self) -> bool { self.palette }
    /// Returns single packed pixel size.
    pub fn get_elem_size(self) -> u8 { self.elem_size }
    /// Reports whether the format is not packed.
    pub fn is_unpacked(&self) -> bool {
        if self.palette { return false; }
        for chr in self.comp_info.iter() {
            if let Some(ref chromaton) = chr {
                if chromaton.is_packed() { return false; }
            }
        }
        true
    }
    /// Returns the maximum component bit depth.
    pub fn get_max_depth(&self) -> u8 {
        let mut mdepth = 0;
        for chr in self.comp_info.iter() {
            if let Some(ref chromaton) = chr {
                mdepth = mdepth.max(chromaton.depth);
            }
        }
        mdepth
    }
    /// Returns the maximum component subsampling.
    pub fn get_max_subsampling(&self) -> u8 {
        let mut ssamp = 0;
        for chr in self.comp_info.iter() {
            if let Some(ref chromaton) = chr {
                let (ss_v, ss_h) = chromaton.get_subsampling();
                ssamp = ssamp.max(ss_v).max(ss_h);
            }
        }
        ssamp
    }
}

impl fmt::Display for NAPixelFormaton {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let end = if self.be { "BE" } else { "LE" };
        let palstr = if self.palette { "palette " } else { "" };
        let astr = if self.alpha { "alpha " } else { "" };
        let mut str = format!("Formaton for {} ({}{}elem {} size {}): ", self.model, palstr, astr,end, self.elem_size);
        for i in 0..self.comp_info.len() {
            if let Some(chr) = self.comp_info[i] {
                str = format!("{} {}", str, chr);
            }
        }
        write!(f, "[{}]", str)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fmt() {
        println!("{}", SND_S16_FORMAT);
        println!("{}", SND_U8_FORMAT);
        println!("{}", SND_F32P_FORMAT);
        println!("formaton yuv- {}", YUV420_FORMAT);
        println!("formaton pal- {}", PAL8_FORMAT);
        println!("formaton rgb565- {}", RGB565_FORMAT);
    }
}
