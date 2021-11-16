use std::ops::Index;

pub struct CharsetSymbol<'a> {
    pub(crate) symbol: char,
    pub(crate) chars: &'a [u8],
}

impl<'a> CharsetSymbol<'a> {
    pub const fn new(symbol: char, chars: &'a [u8]) -> CharsetSymbol {
        CharsetSymbol { symbol, chars }
    }
}

// TODO: try use .as_bytes / alternative const / build.rs
pub const SYMBOL2CHARSET: [CharsetSymbol; 6] = [
    // "abcdefghijklmnopqrstuvwxyz"
    CharsetSymbol::new(
        'l',
        &[
            97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114,
            115, 116, 117, 118, 119, 120, 121, 122,
        ],
    ),
    // "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    CharsetSymbol::new(
        'u',
        &[
            65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86,
            87, 88, 89, 90,
        ],
    ),
    // "0123456789"
    CharsetSymbol::new('d', &[48, 49, 50, 51, 52, 53, 54, 55, 56, 57]),
    // " !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
    CharsetSymbol::new(
        's',
        &[
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 58, 59, 60, 61, 62, 63,
            64, 91, 92, 93, 94, 95, 96, 123, 124, 125, 126,
        ],
    ),
    // "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
    CharsetSymbol::new(
        'a',
        &[
            97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114,
            115, 116, 117, 118, 119, 120, 121, 122, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76,
            77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 48, 49, 50, 51, 52, 53, 54, 55,
            56, 57, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 58, 59, 60, 61, 62,
            63, 64, 91, 92, 93, 94, 95, 96, 123, 124, 125, 126,
        ],
    ),
    CharsetSymbol::new(
        'b',
        &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67,
            68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89,
            90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
            109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125,
            126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142,
            143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159,
            160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176,
            177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193,
            194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210,
            211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227,
            228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244,
            245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255,
        ],
    ),
];

#[repr(align(64))]
pub struct Charset {
    pub(crate) jmp_table: [u8; 256],
    pub(crate) min_char: u8,
    pub(crate) len: usize,
}

impl Index<usize> for Charset {
    type Output = u8;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.jmp_table[index]
    }
}

impl Charset {
    pub fn from_chars(chars: &[u8]) -> Charset {
        let mut jmp_table: [u8; 256] = [0; 256];

        // ensure chars are sorted so jmp_table works correctly
        let mut chars = chars.to_owned();
        chars.sort_unstable();
        for i in 0..chars.len() {
            jmp_table[chars[i] as usize] = chars[(i + 1) % chars.len()];
        }
        Charset {
            jmp_table,
            min_char: chars[0],
            len: chars.len(),
        }
    }

    pub fn from_symbol(symbol: char) -> Charset {
        for charset in &SYMBOL2CHARSET {
            if charset.symbol == symbol {
                return Charset::from_chars(charset.chars);
            }
        }
        panic!("unknown mask symbol - {}", symbol);
    }
}
