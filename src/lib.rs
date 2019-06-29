#[macro_use(value_t)]
extern crate clap;
#[macro_use(lazy_static)]
extern crate lazy_static;
extern crate regex;

use std::io;
use std::io::Write;
use std::ops::Index;

use regex::Regex;

pub mod runner;

const KB: usize = 1024;
const BUFFER_SIZE: usize = 4 * KB;
pub const MAX_WORD_SIZE: usize = 128;

pub struct CharsetSymbol<'a> {
    symbol: char,
    chars: &'a [u8],
}

impl<'a> CharsetSymbol<'a> {
    pub const fn new(symbol: char, chars: &'a [u8]) -> CharsetSymbol {
        CharsetSymbol { symbol, chars }
    }
}

// TODO: try use .as_bytes / alternative const / build.rs
const SYMBOL2CHARSET: [CharsetSymbol; 6] = [
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

pub struct Charset {
    jmp_table: [u8; 256],
    min_char: u8,
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
        chars.sort();
        for i in 0..chars.len() {
            jmp_table[chars[i] as usize] = chars[(i + 1) % chars.len()];
        }
        Charset {
            jmp_table,
            min_char: chars[0],
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

fn is_valid_mask(mask: &str) -> bool {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(format!(r"^(\?[ludsab]){{1,{}}}$", MAX_WORD_SIZE).as_str()).unwrap();
    }
    RE.is_match(mask)
}

pub struct WordGenerator<'a> {
    pub mask: &'a str,
    pub minlen: usize,
    pub maxlen: usize,
    charsets: Vec<Charset>,
    min_word: Vec<u8>,
}

impl<'a> WordGenerator<'a> {
    pub fn new(
        mask: &'a str,
        minlen: Option<usize>,
        maxlen: Option<usize>,
    ) -> Result<WordGenerator, &'static str> {
        if !is_valid_mask(mask) {
            return Err("invalid mask");
        }

        // build charsets
        let charsets: Vec<_> = mask
            .split('?')
            .skip(1)
            .map(|chr| Charset::from_symbol(chr.chars().next().unwrap()))
            .collect();

        // min/max pwd length is by default the longest word
        let minlen = minlen.unwrap_or_else(|| charsets.len());
        let maxlen = maxlen.unwrap_or_else(|| charsets.len());

        // validate minlen
        if !(0 < minlen && minlen <= maxlen && minlen <= charsets.len()) {
            return Err("minlen is invalid");
        }
        if maxlen > charsets.len() {
            return Err("maxlen is invalid");
        }

        // prepare min word - the longest first word
        let min_word: Vec<u8> = charsets.iter().map(|c| c.min_char).collect();

        Ok(WordGenerator {
            mask,
            charsets,
            minlen,
            maxlen,
            min_word,
        })
    }

    /// generates all words into the output buffer `out`
    pub fn gen<'b>(&self, out: Option<Box<dyn Write + 'b>>) -> Result<(), std::io::Error> {
        let mut out = out.unwrap_or_else(|| Box::new(io::stdout()));

        for pwdlen in self.minlen..=self.maxlen {
            self.gen_by_length(pwdlen, &mut out)?;
        }
        Ok(())
    }

    #[allow(clippy::borrowed_box)]
    fn gen_by_length<'b>(
        &self,
        pwdlen: usize,
        out: &mut Box<dyn Write + 'b>,
    ) -> Result<(), std::io::Error> {
        let mut buf = StackBuf::new();
        let batch_size = buf.len() / (pwdlen + 1);

        let word = &mut [b'\n'; MAX_WORD_SIZE][..=pwdlen];
        word[..pwdlen].copy_from_slice(&self.min_word[..pwdlen]);

        'outer_loop: loop {
            'batch_for: for _ in 0..batch_size {
                buf.write(word);
                for pos in (0..pwdlen).rev() {
                    let chr = word[pos];
                    let next_chr = self.charsets[pos][chr as usize];
                    word[pos] = next_chr;

                    if chr < next_chr {
                        continue 'batch_for;
                    }
                }
                break 'outer_loop;
            }

            out.write_all(&buf.getdata())?;
            buf.clear();
        }
        out.write_all(buf.getdata())?;
        Ok(())
    }
}

pub struct StackBuf {
    buf: [u8; BUFFER_SIZE],
    pos: usize,
}

impl StackBuf {
    pub fn new() -> StackBuf {
        StackBuf {
            buf: [0; BUFFER_SIZE],
            pos: 0,
        }
    }

    #[inline]
    pub fn write(&mut self, word: &[u8]) {
        self.buf[self.pos..self.pos + word.len()].copy_from_slice(word);
        self.pos += word.len();
    }

    #[inline]
    pub fn clear(&mut self) {
        self.pos = 0;
    }

    #[inline]
    pub fn getdata(&self) -> &[u8] {
        &self.buf[..self.pos]
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for StackBuf {
    fn default() -> Self {
        Self::new()
    }
}

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Cursor;
    use std::path;

    use crate::{StackBuf, WordGenerator};

    #[test]
    fn test_gen_words_single_digit() {
        let mask = "?d";
        let word_gen = WordGenerator::new(mask, None, None).unwrap();

        assert_eq!(word_gen.mask, mask);
        assert_eq!(word_gen.minlen, 1);
        assert_eq!(word_gen.maxlen, 1);
        assert_eq!(word_gen.charsets.len(), 1);
        assert_eq!(word_gen.min_word, "0".as_bytes());

        let res = assert_gen(word_gen, "single-digits.txt");

        // paranoid test of assert_gen
        assert_eq!(res, "0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
    }

    #[test]
    fn test_gen_upper_lower_1_4() {
        let mask = "?u?l?u?l";
        let word_gen = WordGenerator::new(mask, Some(1), None).unwrap();

        assert_eq!(word_gen.mask, mask);
        assert_eq!(word_gen.minlen, 1);
        assert_eq!(word_gen.maxlen, 4);
        assert_eq!(word_gen.charsets.len(), 4);
        assert_eq!(word_gen.min_word, "AaAa".as_bytes());

        assert_gen(word_gen, "upper-lower-1-4.txt");
    }

    fn assert_gen(w: WordGenerator, fname: &str) -> String {
        let mut buf: Vec<u8> = Vec::new();
        let mut cur = Cursor::new(&mut buf);
        w.gen(Some(Box::new(&mut cur))).unwrap();

        let result = String::from_utf8(buf).unwrap();
        let expected = fs::read_to_string(wordlist_fname(fname)).unwrap();

        assert_eq!(result, expected);
        result
    }

    #[test]
    fn test_stack_buf() {
        let buf = StackBuf::new();
        assert!(!buf.is_empty());

        let default_buf = StackBuf::default();
        assert_eq!(default_buf.pos, 0);
    }

    #[test]
    fn test_is_valid_mask() {
        assert!(crate::is_valid_mask("?d?d?d?d"));
        assert!(crate::is_valid_mask("?l?u?a?b?s"));

        assert!(!crate::is_valid_mask(""));
        assert!(!crate::is_valid_mask("?"));
        assert!(!crate::is_valid_mask("?x"));
    }

    fn wordlist_fname(fname: &str) -> path::PathBuf {
        let mut d = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.extend(vec!["test-resources", fname]);
        d
    }
}
