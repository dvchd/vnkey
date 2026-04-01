//! Module chuyển đổi bảng mã tiếng Việt.
//!
//! Kiến trúc hub-and-spoke: Bảng mã bất kỳ → StdVnChar (bảng 213 ký tự nội bộ) → Bảng mã bất kỳ.
//!
//! Hỗ trợ 19 bảng mã tiếng Việt:
//! - Unicode: UCS-2, UTF-8, NCR decimal, NCR hex, decomposed, Windows CP-1258, C-string
//! - Dạng văn bản: VIQR, UTF8-VIQR
//! - Đơn byte: TCVN3, VPS, VISCII, BKHCM1, VietWare-F, VNU (ISC)
//! - Đôi byte: VNI-Win, BKHCM2, VietWare-X, VNI-Mac

mod data;

pub use data::TOTAL_VNCHARS;

/// Mã bảng mã tương ứng hằng số C gốc
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Charset {
    Unicode = 0,
    Utf8 = 1,
    NcrDec = 2,
    NcrHex = 3,
    UniDecomposed = 4,
    WinCP1258 = 5,
    UniCString = 6,
    Viqr = 10,
    Utf8Viqr = 11,
    Tcvn3 = 20,
    Vps = 21,
    Viscii = 22,
    Bkhcm1 = 23,
    VietwareF = 24,
    Isc = 25,
    VniWin = 40,
    Bkhcm2 = 41,
    VietwareX = 42,
    VniMac = 43,
}

impl Default for Charset {
    fn default() -> Self {
        Charset::Utf8
    }
}

impl Charset {
    /// Phân tích bảng mã từ tên (không phân biệt hoa/thường)
    pub fn from_name(name: &str) -> Option<Charset> {
        let upper = name.to_uppercase();
        match upper.as_str() {
            "UNICODE" | "UCS-2" | "UCS2" => Some(Charset::Unicode),
            "UTF-8" | "UTF8" => Some(Charset::Utf8),
            "NCR-DEC" | "NCRDEC" | "NCR" => Some(Charset::NcrDec),
            "NCR-HEX" | "NCRHEX" => Some(Charset::NcrHex),
            "UNI-COMP" | "UNICOMP" | "UNIDECOMPOSED" => Some(Charset::UniDecomposed),
            "WINCP-1258" | "WINCP1258" | "CP1258" | "CP-1258" => Some(Charset::WinCP1258),
            "UNI-CSTRING" | "UNICSTRING" => Some(Charset::UniCString),
            "VIQR" => Some(Charset::Viqr),
            "UTF8VIQR" | "UTF8-VIQR" => Some(Charset::Utf8Viqr),
            "TCVN3" | "TCVN-3" | "ABC" => Some(Charset::Tcvn3),
            "VPS" => Some(Charset::Vps),
            "VISCII" => Some(Charset::Viscii),
            "BKHCM1" | "BKHCM-1" => Some(Charset::Bkhcm1),
            "VIETWARE-F" | "VIETWAREF" => Some(Charset::VietwareF),
            "ISC" | "VNU" => Some(Charset::Isc),
            "VNI-WIN" | "VNIWIN" | "VNI" => Some(Charset::VniWin),
            "BKHCM2" | "BKHCM-2" => Some(Charset::Bkhcm2),
            "VIETWARE-X" | "VIETWAREX" => Some(Charset::VietwareX),
            "VNI-MAC" | "VNIMAC" => Some(Charset::VniMac),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Charset::Unicode => "UNICODE",
            Charset::Utf8 => "UTF-8",
            Charset::NcrDec => "NCR-DEC",
            Charset::NcrHex => "NCR-HEX",
            Charset::UniDecomposed => "UNI-COMP",
            Charset::WinCP1258 => "WINCP-1258",
            Charset::UniCString => "UNI-CSTRING",
            Charset::Viqr => "VIQR",
            Charset::Utf8Viqr => "UTF8VIQR",
            Charset::Tcvn3 => "TCVN3",
            Charset::Vps => "VPS",
            Charset::Viscii => "VISCII",
            Charset::Bkhcm1 => "BKHCM1",
            Charset::VietwareF => "VIETWARE-F",
            Charset::Isc => "VNU",
            Charset::VniWin => "VNI-WIN",
            Charset::Bkhcm2 => "BKHCM2",
            Charset::VietwareX => "VIETWARE-X",
            Charset::VniMac => "VNI-MAC",
        }
    }

    /// Liệt kê tất cả bảng mã hỗ trợ
    pub fn all() -> &'static [Charset] {
        &[
            Charset::Unicode, Charset::Utf8, Charset::NcrDec, Charset::NcrHex,
            Charset::UniDecomposed, Charset::WinCP1258, Charset::UniCString,
            Charset::Viqr, Charset::Utf8Viqr,
            Charset::Tcvn3, Charset::Vps, Charset::Viscii,
            Charset::Bkhcm1, Charset::VietwareF, Charset::Isc,
            Charset::VniWin, Charset::Bkhcm2, Charset::VietwareX, Charset::VniMac,
        ]
    }
}

/// Biểu diễn ký tự tiếng Việt chuẩn nội bộ.
/// Giá trị >= VN_STD_CHAR_OFFSET là ký tự Việt (chỉ mục = giá trị - offset).
/// Giá trị < VN_STD_CHAR_OFFSET được truyền thẳng.
pub type StdVnChar = u32;

pub const VN_STD_CHAR_OFFSET: u32 = 0x10000;
pub const INVALID_STD_CHAR: u32 = 0xFFFFFFFF;
const TOTAL_ALPHA_VNCHARS: usize = 186;
const PAD_CHAR: u8 = b'#';

/// Chuyển byte từ bảng mã này sang bảng mã khác.
/// Trả về byte đã chuyển, hoặc thông báo lỗi.
pub fn convert(input: &[u8], from: Charset, to: Charset) -> Result<Vec<u8>, &'static str> {
    let std_chars = decode(input, from)?;
    encode(&std_chars, to)
}

/// Chuyển chuỗi UTF-8 sang bảng mã đích
pub fn from_utf8(s: &str, to: Charset) -> Result<Vec<u8>, &'static str> {
    if to == Charset::Utf8 {
        return Ok(s.as_bytes().to_vec());
    }
    let std_chars = decode_utf8(s);
    encode(&std_chars, to)
}

/// Chuyển byte từ bảng mã nguồn sang chuỗi UTF-8
pub fn to_utf8(input: &[u8], from: Charset) -> Result<String, &'static str> {
    if from == Charset::Utf8 {
        return String::from_utf8(input.to_vec()).map_err(|_| "invalid UTF-8");
    }
    let std_chars = decode(input, from)?;
    let bytes = encode(&std_chars, Charset::Utf8)?;
    String::from_utf8(bytes).map_err(|_| "invalid UTF-8 output")
}

// ---------- Giải mã: byte bảng mã → chuỗi StdVnChar ----------

fn decode(input: &[u8], charset: Charset) -> Result<Vec<StdVnChar>, &'static str> {
    match charset {
        Charset::Utf8 => {
            let s = std::str::from_utf8(input).map_err(|_| "invalid UTF-8 input")?;
            Ok(decode_utf8(s))
        }
        Charset::Unicode => Ok(decode_unicode(input)),
        Charset::NcrDec => Ok(decode_ncr(input, false)),
        Charset::NcrHex => Ok(decode_ncr(input, true)),
        Charset::UniDecomposed => Ok(decode_uni_decomposed(input)),
        Charset::WinCP1258 => Ok(decode_double_byte(input, &data::WIN_CP1258, Some(&data::WIN_CP1258_PRE))),
        Charset::UniCString => Ok(decode_uni_cstring(input)),
        Charset::Viqr => Ok(decode_viqr(input)),
        Charset::Utf8Viqr => Ok(decode_utf8_viqr(input)),
        Charset::Tcvn3 => Ok(decode_single_byte(input, &data::SINGLE_BYTE_TCVN3)),
        Charset::Vps => Ok(decode_single_byte(input, &data::SINGLE_BYTE_VPS)),
        Charset::Viscii => Ok(decode_single_byte(input, &data::SINGLE_BYTE_VISCII)),
        Charset::Bkhcm1 => Ok(decode_single_byte(input, &data::SINGLE_BYTE_BKHCM1)),
        Charset::VietwareF => Ok(decode_single_byte(input, &data::SINGLE_BYTE_VIETWAREF)),
        Charset::Isc => Ok(decode_single_byte(input, &data::SINGLE_BYTE_ISC)),
        Charset::VniWin => Ok(decode_double_byte(input, &data::DOUBLE_BYTE_VNIWIN, None)),
        Charset::Bkhcm2 => Ok(decode_double_byte(input, &data::DOUBLE_BYTE_BKHCM2, None)),
        Charset::VietwareX => Ok(decode_double_byte(input, &data::DOUBLE_BYTE_VIETWAREX, None)),
        Charset::VniMac => Ok(decode_double_byte(input, &data::DOUBLE_BYTE_VNIMAC, None)),
    }
}

fn decode_utf8(s: &str) -> Vec<StdVnChar> {
    let mut result = Vec::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        let code = ch as u32;
        // Kiểm tra dạng decomposed: ký tự cơ sở + dấu kết hợp
        if let Some(&next_ch) = chars.peek() {
            let next_code = next_ch as u32;
            if (0x0300..=0x0323).contains(&next_code) {
                // Thử tra bảng composite
                let composite = (next_code << 16) | code;
                if let Some(idx) = data::unicode_composite_to_index(composite) {
                    result.push(VN_STD_CHAR_OFFSET + idx as u32);
                    chars.next(); // tiêu thụ dấu kết hợp
                    continue;
                }
            }
        }
        // Thử tra bảng precomposed
        if let Some(idx) = data::unicode_to_index(code as u16) {
            result.push(VN_STD_CHAR_OFFSET + idx as u32);
        } else {
            result.push(code);
        }
    }
    result
}

fn decode_unicode(input: &[u8]) -> Vec<StdVnChar> {
    let mut result = Vec::with_capacity(input.len() / 2);
    let mut i = 0;
    while i + 1 < input.len() {
        let w = u16::from_le_bytes([input[i], input[i + 1]]);
        if let Some(idx) = data::unicode_to_index(w) {
            result.push(VN_STD_CHAR_OFFSET + idx as u32);
        } else {
            result.push(w as u32);
        }
        i += 2;
    }
    result
}

fn decode_ncr(input: &[u8], allow_hex: bool) -> Vec<StdVnChar> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'&' && i + 1 < input.len() && input[i + 1] == b'#' {
            i += 2;
            let is_hex = allow_hex && i < input.len() && (input[i] == b'x' || input[i] == b'X');
            if is_hex {
                i += 1;
            }
            let mut code: u32 = 0;
            let mut digits = 0;
            while i < input.len() && digits < 5 {
                let b = input[i];
                if is_hex {
                    if b.is_ascii_hexdigit() {
                        code = (code << 4) + hex_digit_value(b) as u32;
                        digits += 1;
                        i += 1;
                    } else {
                        break;
                    }
                } else if b.is_ascii_digit() {
                    code = code * 10 + (b - b'0') as u32;
                    digits += 1;
                    i += 1;
                } else {
                    break;
                }
            }
            if i < input.len() && input[i] == b';' {
                i += 1;
            }
            if let Some(idx) = data::unicode_to_index(code as u16) {
                result.push(VN_STD_CHAR_OFFSET + idx as u32);
            } else {
                result.push(code);
            }
        } else {
            result.push(input[i] as u32);
            i += 1;
        }
    }
    result
}

fn decode_uni_decomposed(input: &[u8]) -> Vec<StdVnChar> {
    let mut result = Vec::new();
    let mut i = 0;
    while i + 1 < input.len() {
        let w1 = u16::from_le_bytes([input[i], input[i + 1]]);
        i += 2;
        // Thử với dấu kết hợp
        if i + 1 < input.len() {
            let w2 = u16::from_le_bytes([input[i], input[i + 1]]);
            if w2 > 0 {
                let composite = ((w2 as u32) << 16) | (w1 as u32);
                if let Some(idx) = data::unicode_composite_to_index(composite) {
                    result.push(VN_STD_CHAR_OFFSET + idx as u32);
                    i += 2;
                    continue;
                }
            }
        }
        // Thử precomposed
        if let Some(idx) = data::unicode_to_index(w1) {
            result.push(VN_STD_CHAR_OFFSET + idx as u32);
        } else {
            // Cũng thử bảng composite cho mục đơn mã
            if let Some(idx) = data::unicode_composite_to_index(w1 as u32) {
                result.push(VN_STD_CHAR_OFFSET + idx as u32);
            } else {
                result.push(w1 as u32);
            }
        }
    }
    result
}

fn decode_uni_cstring(input: &[u8]) -> Vec<StdVnChar> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'\\' && i + 1 < input.len() && (input[i + 1] == b'x' || input[i + 1] == b'X') {
            i += 2;
            let mut code: u16 = 0;
            let mut digits = 0;
            while i < input.len() && input[i].is_ascii_hexdigit() && digits < 4 {
                code = (code << 4) + hex_digit_value(input[i]) as u16;
                digits += 1;
                i += 1;
            }
            if let Some(idx) = data::unicode_to_index(code) {
                result.push(VN_STD_CHAR_OFFSET + idx as u32);
            } else {
                result.push(code as u32);
            }
        } else {
            result.push(input[i] as u32);
            i += 1;
        }
    }
    result
}

fn decode_single_byte(input: &[u8], table: &[u8; TOTAL_VNCHARS]) -> Vec<StdVnChar> {
    // Tạo bảng ngược: byte → chỉ mục StdVnChar+1 (0 = không ánh xạ)
    let mut std_map = [0u16; 256];
    for i in 0..TOTAL_VNCHARS {
        let b = table[i];
        if b != 0 && (i == TOTAL_VNCHARS - 1 || table[i] != table[i + 1]) {
            std_map[b as usize] = (i + 1) as u16;
        }
    }
    let mut result = Vec::with_capacity(input.len());
    for &b in input {
        if std_map[b as usize] != 0 {
            result.push(VN_STD_CHAR_OFFSET + std_map[b as usize] as u32 - 1);
        } else {
            result.push(b as u32);
        }
    }
    result
}

fn decode_double_byte(input: &[u8], table: &[u16; TOTAL_VNCHARS], extra_table: Option<&[u16; TOTAL_VNCHARS]>) -> Vec<StdVnChar> {
    // Tạo bảng tra sắp xếp: word thấp = mã ký tự, word cao = chỉ mục
    let mut lookup: Vec<u32> = Vec::with_capacity(TOTAL_VNCHARS * 2);
    let mut std_map = [0u16; 256];

    for i in 0..TOTAL_VNCHARS {
        let w = table[i];
        if w >> 8 != 0 {
            std_map[(w >> 8) as usize] = 0xFFFF;
        } else if std_map[w as usize] == 0 {
            std_map[w as usize] = (i + 1) as u16;
        }
        lookup.push(((i as u32) << 16) | (w as u32));
    }
    let mut _total = TOTAL_VNCHARS;

    if let Some(extra) = extra_table {
        for k in 0..TOTAL_VNCHARS {
            if extra[k] != table[k] {
                if extra[k] >> 8 != 0 {
                    std_map[(extra[k] >> 8) as usize] = 0xFFFF;
                } else if std_map[extra[k] as usize] == 0 {
                    std_map[extra[k] as usize] = (k + 1) as u16;
                }
                lookup.push(((k as u32) << 16) | (extra[k] as u32));
                _total += 1;
            }
        }
    }

    lookup.sort_by(|a, b| (*a as u16).cmp(&(*b as u16)));

    let mut result = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        let b = input[i];
        let mapped = std_map[b as usize];
        if mapped == 0 {
            result.push(b as u32);
            i += 1;
        } else if mapped == 0xFFFF {
            // Byte đầu tiên của ký tự đôi byte tiềm năng
            if i + 1 < input.len() {
                let hi = input[i + 1];
                let key = ((hi as u16) << 8) | (b as u16);
                if let Ok(pos) = lookup.binary_search_by(|entry| (*entry as u16).cmp(&key)) {
                    result.push(VN_STD_CHAR_OFFSET + (lookup[pos] >> 16));
                    i += 2;
                } else {
                    result.push(INVALID_STD_CHAR);
                    i += 1;
                }
            } else {
                result.push(INVALID_STD_CHAR);
                i += 1;
            }
        } else {
            let std_char = VN_STD_CHAR_OFFSET + mapped as u32 - 1;
            // Kiểm tra đôi byte
            if i + 1 < input.len() {
                let hi = input[i + 1];
                if hi > 0 {
                    let key = ((hi as u16) << 8) | (b as u16);
                    if let Ok(pos) = lookup.binary_search_by(|entry| (*entry as u16).cmp(&key)) {
                        result.push(VN_STD_CHAR_OFFSET + (lookup[pos] >> 16));
                        i += 2;
                        continue;
                    }
                }
            }
            result.push(std_char);
            i += 1;
        }
    }
    result
}

fn is_vowel(ch: u8) -> bool {
    matches!(ch.to_ascii_lowercase(), b'a' | b'e' | b'i' | b'o' | b'u' | b'y')
}

fn decode_viqr(input: &[u8]) -> Vec<StdVnChar> {
    // Tạo bảng ngược VIQR
    let mut std_map = [0u16; 256];
    for i in 0..TOTAL_VNCHARS {
        let dw = data::VIQR_TABLE[i];
        if dw & 0xFFFFFF00 == 0 {
            std_map[dw as usize] = (i + 256) as u16;
        }
    }
    // Offset dấu/dấu phụ
    std_map[b'\'' as usize] = 2;
    std_map[b'`' as usize] = 4;
    std_map[b'?' as usize] = 6;
    std_map[b'~' as usize] = 8;
    std_map[b'.' as usize] = 10;
    std_map[b'^' as usize] = 12;
    std_map[b'(' as usize] = 24;
    std_map[b'+' as usize] = 26;
    std_map[b'*' as usize] = 26;

    let mut result = Vec::with_capacity(input.len());
    let mut i = 0;
    let mut at_word_beginning = true;
    let mut got_tone = false;

    while i < input.len() {
        let ch1 = input[i];
        i += 1;

        // Thoát ký tự
        if ch1 == b'\\' && i < input.len() {
            result.push(input[i] as u32);
            i += 1;
            at_word_beginning = true;
            got_tone = false;
            continue;
        }

        let mut std_char = std_map[ch1 as usize] as u32;

        if std_char < 256 {
            std_char = ch1 as u32;
        } else if i < input.len() {
            let upper = ch1.to_ascii_uppercase();
            let ch2 = input[i];

            // Xử lý DD
            if at_word_beginning && upper == b'D' && (ch2 == b'd' || ch2 == b'D') {
                std_char += 2;
                i += 1;
            } else {
                let index = std_map[ch2 as usize];
                let cond = is_vowel(ch1) && (
                    (index <= 10 && index > 0 && (!got_tone || (index != 6 && index != 10))) ||
                    (index == 12 && (upper == b'A' || upper == b'E' || upper == b'O')) ||
                    (std_map[ch2 as usize] == 24 && upper == b'A') ||
                    (std_map[ch2 as usize] == 26 && (upper == b'O' || upper == b'U'))
                );

                if cond {
                    if index > 0 {
                        got_tone = true;
                    }
                    i += 1;
                    let mut offset = std_map[ch2 as usize] as u32;
                    if offset == 26 { offset = 24; }
                    if offset == 24 && (ch1 == b'u' || ch1 == b'U') {
                        offset = 12;
                    }
                    std_char += offset;
                    // Kiểm tra dấu sau dấu phụ
                    if i < input.len() && index > 10 {
                        let ch3_idx = std_map[input[i] as usize];
                        if ch3_idx > 0 && ch3_idx <= 10 {
                            std_char += ch3_idx as u32;
                            i += 1;
                        }
                    }
                }
            }
        }

        at_word_beginning = std_char < 256;
        if std_char < 256 {
            got_tone = false;
        }

        if std_char >= 256 {
            result.push(VN_STD_CHAR_OFFSET + std_char - 256);
        } else {
            result.push(std_char);
        }
    }
    result
}

fn decode_utf8_viqr(input: &[u8]) -> Vec<StdVnChar> {
    // Hỗn hợp UTF-8/VIQR: nếu byte > 0xBF, giải mã UTF-8; ngược lại VIQR
    let mut result = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let b = input[i];
        if b > 0xBF && b < 0xFE {
            // Chuỗi UTF-8
            if (b & 0xE0) == 0xC0 && i + 1 < input.len() {
                let b2 = input[i + 1];
                if (b2 & 0xC0) == 0x80 {
                    let code = (((b as u16) & 0x1F) << 6) | ((b2 as u16) & 0x3F);
                    if let Some(idx) = data::unicode_to_index(code) {
                        result.push(VN_STD_CHAR_OFFSET + idx as u32);
                    } else {
                        result.push(code as u32);
                    }
                    i += 2;
                    continue;
                }
            } else if (b & 0xF0) == 0xE0 && i + 2 < input.len() {
                let b2 = input[i + 1];
                let b3 = input[i + 2];
                if (b2 & 0xC0) == 0x80 && (b3 & 0xC0) == 0x80 {
                    let code = (((b as u16) & 0x0F) << 12) | (((b2 as u16) & 0x3F) << 6) | ((b3 as u16) & 0x3F);
                    if let Some(idx) = data::unicode_to_index(code) {
                        result.push(VN_STD_CHAR_OFFSET + idx as u32);
                    } else {
                        result.push(code as u32);
                    }
                    i += 3;
                    continue;
                }
            }
            result.push(b as u32);
            i += 1;
        } else {
            // Giải mã VIQR từ điểm này cho một "từ"
            let chunk_start = i;
            while i < input.len() && !(input[i] > 0xBF && input[i] < 0xFE) {
                i += 1;
            }
            let viqr_chars = decode_viqr(&input[chunk_start..i]);
            result.extend(viqr_chars);
        }
    }
    result
}

// ---------- Mã hóa: chuỗi StdVnChar → byte bảng mã ----------

fn encode(chars: &[StdVnChar], charset: Charset) -> Result<Vec<u8>, &'static str> {
    match charset {
        Charset::Utf8 => Ok(encode_utf8(chars)),
        Charset::Unicode => Ok(encode_unicode(chars)),
        Charset::NcrDec => Ok(encode_ncr_dec(chars)),
        Charset::NcrHex => Ok(encode_ncr_hex(chars)),
        Charset::UniDecomposed => Ok(encode_uni_decomposed(chars)),
        Charset::WinCP1258 => Ok(encode_double_byte(chars, &data::WIN_CP1258)),
        Charset::UniCString => Ok(encode_uni_cstring(chars)),
        Charset::Viqr | Charset::Utf8Viqr => Ok(encode_viqr(chars)),
        Charset::Tcvn3 => Ok(encode_single_byte(chars, &data::SINGLE_BYTE_TCVN3)),
        Charset::Vps => Ok(encode_single_byte(chars, &data::SINGLE_BYTE_VPS)),
        Charset::Viscii => Ok(encode_single_byte(chars, &data::SINGLE_BYTE_VISCII)),
        Charset::Bkhcm1 => Ok(encode_single_byte(chars, &data::SINGLE_BYTE_BKHCM1)),
        Charset::VietwareF => Ok(encode_single_byte(chars, &data::SINGLE_BYTE_VIETWAREF)),
        Charset::Isc => Ok(encode_single_byte(chars, &data::SINGLE_BYTE_ISC)),
        Charset::VniWin => Ok(encode_double_byte(chars, &data::DOUBLE_BYTE_VNIWIN)),
        Charset::Bkhcm2 => Ok(encode_double_byte(chars, &data::DOUBLE_BYTE_BKHCM2)),
        Charset::VietwareX => Ok(encode_double_byte(chars, &data::DOUBLE_BYTE_VIETWAREX)),
        Charset::VniMac => Ok(encode_double_byte(chars, &data::DOUBLE_BYTE_VNIMAC)),
    }
}

fn encode_utf8(chars: &[StdVnChar]) -> Vec<u8> {
    let mut result = Vec::with_capacity(chars.len() * 3);
    for &sc in chars {
        let code = if sc >= VN_STD_CHAR_OFFSET {
            data::UNICODE_TABLE[(sc - VN_STD_CHAR_OFFSET) as usize] as u32
        } else {
            sc
        };
        if code < 0x80 {
            result.push(code as u8);
        } else if code < 0x800 {
            result.push(0xC0 | (code >> 6) as u8);
            result.push(0x80 | (code & 0x3F) as u8);
        } else {
            result.push(0xE0 | (code >> 12) as u8);
            result.push(0x80 | ((code >> 6) & 0x3F) as u8);
            result.push(0x80 | (code & 0x3F) as u8);
        }
    }
    result
}

fn encode_unicode(chars: &[StdVnChar]) -> Vec<u8> {
    let mut result = Vec::with_capacity(chars.len() * 2);
    for &sc in chars {
        let code = if sc >= VN_STD_CHAR_OFFSET {
            data::UNICODE_TABLE[(sc - VN_STD_CHAR_OFFSET) as usize]
        } else {
            sc as u16
        };
        result.extend_from_slice(&code.to_le_bytes());
    }
    result
}

fn encode_ncr_dec(chars: &[StdVnChar]) -> Vec<u8> {
    let mut result = Vec::new();
    for &sc in chars {
        let code = if sc >= VN_STD_CHAR_OFFSET {
            data::UNICODE_TABLE[(sc - VN_STD_CHAR_OFFSET) as usize] as u32
        } else {
            sc
        };
        if code < 128 {
            result.push(code as u8);
        } else {
            result.extend_from_slice(b"&#");
            let s = code.to_string();
            result.extend_from_slice(s.as_bytes());
            result.push(b';');
        }
    }
    result
}

fn encode_ncr_hex(chars: &[StdVnChar]) -> Vec<u8> {
    let mut result = Vec::new();
    for &sc in chars {
        let code = if sc >= VN_STD_CHAR_OFFSET {
            data::UNICODE_TABLE[(sc - VN_STD_CHAR_OFFSET) as usize] as u32
        } else {
            sc
        };
        if code < 256 {
            result.push(code as u8);
        } else {
            result.extend_from_slice(b"&#x");
            let s = format!("{:X}", code);
            result.extend_from_slice(s.as_bytes());
            result.push(b';');
        }
    }
    result
}

fn encode_uni_decomposed(chars: &[StdVnChar]) -> Vec<u8> {
    let mut result = Vec::new();
    for &sc in chars {
        if sc >= VN_STD_CHAR_OFFSET {
            let comp = data::UNICODE_COMPOSITE[(sc - VN_STD_CHAR_OFFSET) as usize];
            let lo = (comp & 0xFFFF) as u16;
            let hi = ((comp >> 16) & 0xFFFF) as u16;
            result.extend_from_slice(&lo.to_le_bytes());
            if hi > 0 {
                result.extend_from_slice(&hi.to_le_bytes());
            }
        } else {
            result.extend_from_slice(&(sc as u16).to_le_bytes());
        }
    }
    result
}

fn encode_uni_cstring(chars: &[StdVnChar]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut prev_is_hex = false;
    for &sc in chars {
        let code = if sc >= VN_STD_CHAR_OFFSET {
            data::UNICODE_TABLE[(sc - VN_STD_CHAR_OFFSET) as usize] as u32
        } else {
            sc
        };
        if code < 256 && (!(code as u8).is_ascii_hexdigit() || !prev_is_hex) {
            result.push(code as u8);
            prev_is_hex = false;
        } else {
            let s = format!("\\x{:X}", code);
            result.extend_from_slice(s.as_bytes());
            prev_is_hex = true;
        }
    }
    result
}

fn encode_single_byte(chars: &[StdVnChar], table: &[u8; TOTAL_VNCHARS]) -> Vec<u8> {
    // Tạo bảng ngược để phát hiện xung đột
    let mut std_map = [0u16; 256];
    for i in 0..TOTAL_VNCHARS {
        let b = table[i];
        if b != 0 && (i == TOTAL_VNCHARS - 1 || table[i] != table[i + 1]) {
            std_map[b as usize] = (i + 1) as u16;
        }
    }

    let mut result = Vec::with_capacity(chars.len());
    for &sc in chars {
        if sc >= VN_STD_CHAR_OFFSET {
            let idx = (sc - VN_STD_CHAR_OFFSET) as usize;
            if idx < TOTAL_VNCHARS {
                let b = table[idx];
                if b == 0 {
                    result.push(PAD_CHAR);
                } else {
                    result.push(b);
                }
            } else {
                result.push(PAD_CHAR);
            }
        } else if sc > 255 || std_map[sc as usize] != 0 {
            result.push(PAD_CHAR);
        } else {
            result.push(sc as u8);
        }
    }
    result
}

fn encode_double_byte(chars: &[StdVnChar], table: &[u16; TOTAL_VNCHARS]) -> Vec<u8> {
    let mut std_map = [0u16; 256];
    for i in 0..TOTAL_VNCHARS {
        let w = table[i];
        if w >> 8 != 0 {
            std_map[(w >> 8) as usize] = 0xFFFF;
        } else if std_map[w as usize] == 0 {
            std_map[w as usize] = (i + 1) as u16;
        }
    }

    let mut result = Vec::with_capacity(chars.len() * 2);
    for &sc in chars {
        if sc >= VN_STD_CHAR_OFFSET {
            let idx = (sc - VN_STD_CHAR_OFFSET) as usize;
            if idx < TOTAL_VNCHARS {
                let w = table[idx];
                if w & 0xFF00 != 0 {
                    result.push((w & 0xFF) as u8);
                    result.push((w >> 8) as u8);
                } else {
                    let b = w as u8;
                    if std_map[b as usize] == 0xFFFF {
                        result.push(PAD_CHAR);
                    } else {
                        result.push(b);
                    }
                }
            } else {
                result.push(PAD_CHAR);
            }
        } else if sc > 255 || std_map[sc as usize] != 0 {
            result.push(PAD_CHAR);
        } else {
            result.push(sc as u8);
        }
    }
    result
}

fn encode_viqr(chars: &[StdVnChar]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut escape_tone = false;
    let mut escape_bowl = false;
    let mut escape_roof = false;
    let mut escape_hook = false;

    // Bảng dấu VIQR cho thoát ký tự đầu ra
    let mut tone_map = [0u16; 256];
    tone_map[b'\'' as usize] = 2;
    tone_map[b'`' as usize] = 4;
    tone_map[b'?' as usize] = 6;
    tone_map[b'~' as usize] = 8;
    tone_map[b'.' as usize] = 10;
    tone_map[b'^' as usize] = 12;
    tone_map[b'(' as usize] = 24;
    tone_map[b'+' as usize] = 26;
    tone_map[b'*' as usize] = 26;

    for &sc in chars {
        if sc >= VN_STD_CHAR_OFFSET {
            let idx = (sc - VN_STD_CHAR_OFFSET) as usize;
            if idx < TOTAL_VNCHARS {
                let dw = data::VIQR_TABLE[idx];
                let first = (dw & 0xFF) as u8;
                result.push(first);
                if dw & 0xFF00 != 0 {
                    result.push(((dw >> 8) & 0xFF) as u8);
                    if dw & 0xFF0000 != 0 {
                        result.push(((dw >> 16) & 0xFF) as u8);
                        escape_tone = false;
                    } else {
                        let second = ((dw >> 8) & 0xFF) as u8;
                        let idx2 = tone_map[second as usize];
                        escape_tone = idx2 == 12 || idx2 == 24 || idx2 == 26;
                    }
                    escape_bowl = false;
                    escape_hook = false;
                    escape_roof = false;
                } else {
                    let first_upper = first.to_ascii_uppercase();
                    escape_tone = is_vowel(first);
                    escape_bowl = first_upper == b'A';
                    escape_hook = first_upper == b'U' || first_upper == b'O';
                    escape_roof = first_upper == b'A' || first_upper == b'E' || first_upper == b'O';
                }
            }
        } else if sc > 255 {
            result.push(PAD_CHAR);
        } else {
            let b = sc as u8;
            let index = tone_map[b as usize];
            if b == b'\\' ||
               (index > 0 && index <= 10 && escape_tone) ||
               (index == 12 && escape_roof) ||
               (index == 24 && escape_bowl) ||
               (index == 26 && escape_hook) {
                result.push(b'\\');
            }
            result.push(b);
            escape_bowl = false;
            escape_roof = false;
            escape_hook = false;
            escape_tone = false;
        }
    }
    result
}

// ---------- Hàm tiện ích ----------

/// Chuyển StdVnChar sang chữ hoa
pub fn std_vn_to_upper(ch: StdVnChar) -> StdVnChar {
    if ch >= VN_STD_CHAR_OFFSET && ch < VN_STD_CHAR_OFFSET + TOTAL_ALPHA_VNCHARS as u32 && (ch & 1) != 0 {
        ch - 1
    } else {
        ch
    }
}

/// Chuyển StdVnChar sang chữ thường
pub fn std_vn_to_lower(ch: StdVnChar) -> StdVnChar {
    if ch >= VN_STD_CHAR_OFFSET && ch < VN_STD_CHAR_OFFSET + TOTAL_ALPHA_VNCHARS as u32 && (ch & 1) == 0 {
        ch + 1
    } else {
        ch
    }
}

/// Bỏ dấu thanh của StdVnChar
pub fn std_vn_remove_tone(ch: StdVnChar) -> StdVnChar {
    if ch >= VN_STD_CHAR_OFFSET && ch < VN_STD_CHAR_OFFSET + TOTAL_VNCHARS as u32 {
        VN_STD_CHAR_OFFSET + data::STD_VN_NO_TONE[(ch - VN_STD_CHAR_OFFSET) as usize] as u32
    } else {
        ch
    }
}

fn hex_digit_value(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}
