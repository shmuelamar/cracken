#[macro_use(value_t)]
extern crate clap;
#[macro_use(lazy_static)]
extern crate lazy_static;
extern crate regex;

use std::io;
use std::io::Write;

use crate::charsets::Charset;
use crate::wordlists::Wordlist;
use regex::Regex;
use std::rc::Rc;

pub mod charsets;
pub mod runner;
pub mod wordlists;

const KB: usize = 1024;
const BUFFER_SIZE: usize = 4 * KB;
pub const MAX_WORD_SIZE: usize = 128;

#[derive(Debug, PartialEq)]
enum MaskOp {
    Char(char),
    BuiltinCharset(char),
    CustomCharset(usize),
    Wordlist(usize),
}

/// returns true iff the mask is valid
fn is_valid_mask(mask: &str) -> bool {
    lazy_static! {
        static ref RE: Regex = Regex::new(
            format!(
                r"^(\?[ludsab1-9]|\?w[1-9]|\\.|[^?\\]){{1,{}}}$",
                MAX_WORD_SIZE
            )
            .as_str()
        )
        .unwrap();
    }
    RE.is_match(mask)
}

/// parses `mask` string into the operations it means
fn parse_mask(mask: &str) -> Result<Vec<MaskOp>, &'static str> {
    if !is_valid_mask(mask) {
        return Err("Invalid mask");
    }

    let mut mask_ops = vec![];
    let mut chars = mask.chars();
    let mut next = chars.next();

    while next.is_some() {
        let ch = next.unwrap();
        match ch {
            // 1. escaped char (like \?)
            '\\' => mask_ops.push(MaskOp::Char(chars.next().unwrap())),
            // 2. charsets (like ?d)
            '?' => {
                let next_chr = chars.next().unwrap();

                // 2.1 custom charset
                if next_chr.is_digit(10) {
                    mask_ops.push(MaskOp::CustomCharset(((next_chr as u8) - b'1') as usize))

                // 2.2 wordlist
                } else if next_chr == 'w' {
                    let idx = chars.next().unwrap();
                    mask_ops.push(MaskOp::Wordlist(((idx as u8) - b'1') as usize));

                // 2.3 builtin charset
                } else {
                    mask_ops.push(MaskOp::BuiltinCharset(next_chr))
                }
            }
            // 3. single char
            _ => mask_ops.push(MaskOp::Char(ch)),
        }
        next = chars.next();
    }
    Ok(mask_ops)
}

pub struct WordGenerator<'a> {
    pub mask: &'a str,
    pub minlen: usize,
    pub maxlen: usize,
    charsets: Vec<Charset>,
    min_word: Vec<u8>,
}

enum AItem {
    Charset(Charset),
    Wordlist(Rc<Wordlist>),
}

struct AGenerator<'a> {
    pub mask: &'a str,
    items: Vec<AItem>,
}

impl<'a> AGenerator<'a> {
    pub fn new(
        mask: &'a str,
        wordlists_fnames: &[&'a str],
        custom_charsets: &[&'a str],
    ) -> Result<AGenerator<'a>, &'static str> {
        let mask_ops = parse_mask(mask)?;

        // TODO: split to functions
        let mut wordlists_data = vec![];
        for fname in wordlists_fnames.iter() {
            wordlists_data.push(Rc::new(Wordlist::from_file(fname).expect("invalid fname")));
        }

        // TODO: return error from custom_charset not in index & invalid symbol
        let items: Vec<AItem> = mask_ops
            .into_iter()
            .map(|op| match op {
                MaskOp::Char(ch) => AItem::Charset(Charset::from_chars(vec![ch as u8].as_ref())),
                MaskOp::BuiltinCharset(ch) => AItem::Charset(Charset::from_symbol(ch)),
                MaskOp::CustomCharset(idx) => {
                    AItem::Charset(Charset::from_chars(custom_charsets[idx].as_bytes()))
                }
                MaskOp::Wordlist(idx) => AItem::Wordlist(Rc::clone(&wordlists_data[idx])),
            })
            .collect();

        Ok(AGenerator { mask, items })
    }

    /// generates all words into the output buffer `out`
    pub fn gen<'b>(&self, out: Option<Box<dyn Write + 'b>>) -> Result<(), std::io::Error> {
        let mut out = out.unwrap_or_else(|| Box::new(io::stdout()));

        self.gen_words(&mut out)?;
        Ok(())
    }

    fn gen_words<'b>(&self, out: &mut Box<dyn Write + 'b>) -> Result<(), std::io::Error> {
        let mut buf = StackBuf::new();

        let mut word_buf = [b'\n'; MAX_WORD_SIZE];
        let word = &mut word_buf[..];

        enum Position<'a> {
            CharsetPos {
                charset: &'a Charset,
                chr: u8,
            },
            WordlistPos {
                wordlist: &'a Rc<Wordlist>,
                idx: usize,
            },
        }

        let mut positions: Vec<_> = self
            .items
            .iter()
            .map(|item| match item {
                AItem::Charset(charset) => Position::CharsetPos {
                    charset,
                    chr: charset.min_char,
                },
                AItem::Wordlist(wordlist) => Position::WordlistPos { wordlist, idx: 0 },
            })
            .collect();

        let mut min_word = vec![];
        for pos in positions.iter() {
            match pos {
                Position::CharsetPos { charset: _, chr } => min_word.push(*chr),
                Position::WordlistPos { wordlist, idx: _ } => {
                    min_word.extend_from_slice(&wordlist[0])
                }
            }
        }
        min_word.push(b'\n');
        let min_word = min_word;
        let mut word_len = min_word.len();

        word[..word_len].copy_from_slice(&min_word);

        'outer_loop: loop {
            if buf.pos + word_len >= buf.len() {
                out.write_all(&buf.getdata())?;
                buf.clear();
            }
            buf.write(&word[..word_len]);

            let mut pos = word_len - 2;

            for itempos in positions.iter_mut().rev() {
                match itempos {
                    Position::CharsetPos { charset, chr } => {
                        let prev_chr = *chr;
                        *chr = charset[prev_chr as usize];
                        word[pos] = *chr;

                        if prev_chr < *chr {
                            continue 'outer_loop;
                        }

                        // TODO: this is because test has overflow check
                        if pos == 0 {
                            break 'outer_loop;
                        }
                        pos -= 1;
                    }
                    Position::WordlistPos { wordlist, idx } => {
                        let prev_len = wordlist[*idx].len();
                        *idx += 1;
                        if *idx == wordlist.len() {
                            *idx = 0;
                        }

                        let wlen = wordlist[*idx].len();

                        if prev_len == wlen {
                            word[pos + 1 - wlen..pos + 1].copy_from_slice(&wordlist[*idx]);
                            if pos >= wlen {
                                pos -= wlen;
                            } else {
                                pos = 0;
                            }
                        } else {
                            let offset = wlen as isize - prev_len as isize;

                            // move the suffix by offset (can be negative)
                            let after_word = pos + 1;
                            let tmp = word[after_word..word_len].to_vec();
                            word[(after_word as isize + offset) as usize
                                ..(word_len as isize + offset) as usize]
                                .copy_from_slice(&tmp);

                            // update current position & wordlien by offset
                            pos = (pos as isize + offset) as usize;
                            word_len = (word_len as isize + offset) as usize;

                            // copy the next word (similar to prev_len == wlen block)
                            word[pos + 1 - wlen..pos + 1].copy_from_slice(&wordlist[*idx]);
                            if pos >= wlen {
                                pos -= wlen;
                            } else {
                                pos = 0;
                            }
                        }

                        // if idx == 0 we finished the wordlist
                        if *idx > 0 {
                            continue 'outer_loop;
                        }
                    }
                }
            }

            // done
            break;
        }
        out.write_all(buf.getdata())?;
        Ok(())
    }
}

impl<'a> WordGenerator<'a> {
    pub fn new(
        mask: &'a str,
        minlen: Option<usize>,
        maxlen: Option<usize>,
        custom_charsets: &[&'a str],
    ) -> Result<WordGenerator<'a>, &'static str> {
        let mask_ops = parse_mask(mask)?;

        // TODO: return error from custom_charset not in index & invalid symbol
        let charsets: Vec<_> = mask_ops
            .into_iter()
            .map(|op| match op {
                MaskOp::Char(ch) => Charset::from_chars(vec![ch as u8].as_ref()),
                MaskOp::BuiltinCharset(ch) => Charset::from_symbol(ch),
                MaskOp::CustomCharset(idx) => Charset::from_chars(custom_charsets[idx].as_bytes()),
                MaskOp::Wordlist(_) => unreachable!("cant handle wordlists"),
            })
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

    /// calculates number of words to be generated by this WordGenerator
    pub fn combinations(&self) -> u64 {
        let mut combs = 0;
        for i in self.minlen..=self.maxlen {
            combs += self
                .charsets
                .iter()
                .take(i)
                .fold(1, |acc, x| acc * x.chars.len() as u64);
        }
        combs
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
    use crate::{MaskOp, StackBuf, WordGenerator};
    use std::fs;
    use std::io::Cursor;
    use std::path;

    #[test]
    fn test_gen_words_single_digit() {
        let mask = "?d";
        let word_gen = WordGenerator::new(mask, None, None, &vec![]).unwrap();

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
        let word_gen = WordGenerator::new(mask, Some(1), None, &vec![]).unwrap();

        assert_eq!(word_gen.mask, mask);
        assert_eq!(word_gen.minlen, 1);
        assert_eq!(word_gen.maxlen, 4);
        assert_eq!(word_gen.charsets.len(), 4);
        assert_eq!(word_gen.min_word, "AaAa".as_bytes());

        assert_gen(word_gen, "upper-lower-1-4.txt");
    }

    #[test]
    fn test_gen_pwd_upper_lower_year_1_4() {
        let mask = "pwd?u?l201?1";
        let word_gen = WordGenerator::new(mask, Some(1), None, &vec!["56789"]).unwrap();

        assert_eq!(word_gen.mask, mask);
        assert_eq!(word_gen.minlen, 1);
        assert_eq!(word_gen.maxlen, 9);
        assert_eq!(word_gen.charsets.len(), 9);
        assert_eq!(word_gen.min_word, "pwdAa2015".as_bytes());

        assert_gen(word_gen, "upper-lower-year-1-4.txt");
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
    fn test_gen_stats() {
        let custom_charsets = vec!["abcd", "01"];
        let combinations = vec![
            ("?d?s?u?l?a?b", 5368197120, None, None),
            ("?d?d?d?d?d?d?d?d", 111111110, Some(1), Some(8)),
            ("?d?d?d?d?d?d?d?d", 10000, Some(4), Some(4)),
            ("?d?d?d?d?d?d?d?d", 100000000, None, Some(8)),
            ("?1?2", 8, None, None),
            ("?d?1?2", 80, None, None),
            ("?d?s?u?l?a?b?1?2", 42945576960, None, None),
            ("?d?1?2?d", 930, Some(1), None),
        ];

        for (mask, result, minlen, maxlen) in combinations {
            let word_gen = WordGenerator::new(mask, minlen, maxlen, &custom_charsets).unwrap();
            assert_eq!(word_gen.combinations(), result);
        }
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
        let valid_masks = vec![
            "?d?d?d?d",
            "?l?u?a?b?s",
            "abc?l?u?a?b?sdef?1?2?3",
            "?a?b\\?",
        ];
        for mask in valid_masks {
            assert!(crate::is_valid_mask(mask));
        }

        let invalid_masks = vec!["", "?", "?x", "??", "?"];
        for mask in invalid_masks {
            assert!(!crate::is_valid_mask(mask));
        }
    }

    #[test]
    fn test_parse_mask() {
        let valid_masks = vec![
            (
                "?d?d",
                vec![MaskOp::BuiltinCharset('d'), MaskOp::BuiltinCharset('d')],
            ),
            (
                "?l?u?a?b?s",
                vec![
                    MaskOp::BuiltinCharset('l'),
                    MaskOp::BuiltinCharset('u'),
                    MaskOp::BuiltinCharset('a'),
                    MaskOp::BuiltinCharset('b'),
                    MaskOp::BuiltinCharset('s'),
                ],
            ),
            (
                "a ?ld?1?2\\?a",
                vec![
                    MaskOp::Char('a'),
                    MaskOp::Char(' '),
                    MaskOp::BuiltinCharset('l'),
                    MaskOp::Char('d'),
                    MaskOp::CustomCharset(0),
                    MaskOp::CustomCharset(1),
                    MaskOp::Char('?'),
                    MaskOp::Char('a'),
                ],
            ),
        ];

        for (mask, expected) in valid_masks {
            let mask_ops = crate::parse_mask(mask).unwrap();
            assert_eq!(mask_ops, expected);
        }
    }

    fn wordlist_fname(fname: &str) -> path::PathBuf {
        let mut d = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.extend(vec!["test-resources", fname]);
        d
    }
}
