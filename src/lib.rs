#[macro_use(value_t)]
extern crate clap;
#[macro_use(lazy_static)]
extern crate lazy_static;
extern crate regex;

use std::io;
use std::io::Write;

use crate::charsets::Charset;
use regex::Regex;

pub mod charsets;
pub mod runner;

const KB: usize = 1024;
const BUFFER_SIZE: usize = 4 * KB;
pub const MAX_WORD_SIZE: usize = 128;

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
