use num_bigint::{BigUint, ToBigUint};
use std::io;
use std::io::Write;
use std::rc::Rc;

use crate::charsets::Charset;
use crate::mask::{parse_mask, MaskOp};
use crate::stackbuf::StackBuf;
use crate::wordlists::{Wordlist, WordlistIterator};
use crate::{BoxResult, MAX_WORD_SIZE};

pub trait WordGenerator {
    fn gen<'b>(&self, out: Option<Box<dyn Write + 'b>>) -> Result<(), std::io::Error>;
    fn combinations(&self) -> BigUint;
}

/// Generator optimized for charsets only
pub struct CharsetGenerator<'a> {
    pub mask: &'a str,
    pub minlen: usize,
    pub maxlen: usize,
    charsets: Vec<Charset>,
    min_word: Vec<u8>,
}

/// Wordlist Generator for both charsets and wordlists
pub struct WordlistGenerator<'a> {
    pub mask: &'a str,
    items: Vec<WordlistItem>,
}

#[allow(clippy::large_enum_variant)]
enum WordlistItem {
    Charset(Charset),
    Wordlist(Rc<Wordlist>),
}

enum Position<'a> {
    CharsetPos {
        charset: &'a Charset,
        chr: u8,
    },
    WordlistPos {
        wordlist: &'a Rc<Wordlist>,
        idx: WordlistIterator<'a>,
    },
}

/// returns the correct word generator based on the args provided
pub fn get_word_generator<'a>(
    mask: &'a str,
    minlen: Option<usize>,
    maxlen: Option<usize>,
    custom_charsets: &[&'a str],
    wordlists_fnames: &[&'a str],
) -> BoxResult<Box<dyn WordGenerator + 'a>> {
    if wordlists_fnames.is_empty() {
        Ok(Box::new(CharsetGenerator::new(
            mask,
            minlen,
            maxlen,
            custom_charsets,
        )?))
    } else if minlen.is_some() || maxlen.is_some() {
        bail!("cannot set minlen or maxlen with wordlists")
    } else {
        Ok(Box::new(WordlistGenerator::new(
            mask,
            wordlists_fnames,
            custom_charsets,
        )?))
    }
}

impl<'a> CharsetGenerator<'a> {
    pub fn new(
        mask: &'a str,
        minlen: Option<usize>,
        maxlen: Option<usize>,
        custom_charsets: &[&'a str],
    ) -> BoxResult<CharsetGenerator<'a>> {
        let mask_ops = parse_mask(mask)?;

        let mut max_custom_charset = -1;
        for op in &mask_ops {
            if let MaskOp::CustomCharset(idx) = op {
                max_custom_charset = max_custom_charset.max(idx.to_owned() as isize)
            }
        }

        // validate custom charset
        if max_custom_charset >= custom_charsets.len() as isize {
            bail!(format!(
                "mask contains ?{} charset but only {} custom charsets defined",
                max_custom_charset + 1,
                custom_charsets.len()
            ));
        }

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
            bail!("minlen is invalid");
        }
        if maxlen > charsets.len() {
            bail!("maxlen is invalid");
        }

        // prepare min word - the longest first word
        let min_word: Vec<u8> = charsets.iter().map(|c| c.min_char).collect();

        Ok(CharsetGenerator {
            mask,
            minlen,
            maxlen,
            charsets,
            min_word,
        })
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

            out.write_all(buf.getdata())?;
            buf.clear();
        }
        out.write_all(buf.getdata())?;
        Ok(())
    }
}

impl<'a> WordGenerator for CharsetGenerator<'a> {
    /// generates all words into the output buffer `out`
    fn gen<'b>(&self, out: Option<Box<dyn Write + 'b>>) -> Result<(), std::io::Error> {
        let mut out = out.unwrap_or_else(|| Box::new(io::stdout()));

        for pwdlen in self.minlen..=self.maxlen {
            self.gen_by_length(pwdlen, &mut out)?;
        }
        Ok(())
    }

    /// calculates number of words to be generated by this WordGenerator
    fn combinations(&self) -> BigUint {
        let mut combs: BigUint = 0.to_biguint().unwrap();
        for i in self.minlen..=self.maxlen {
            combs += self
                .charsets
                .iter()
                .take(i)
                .fold(1.to_biguint().unwrap(), |acc, x| {
                    (acc * x.chars.len()).to_biguint().unwrap()
                });
        }
        combs
    }
}

impl<'a> WordlistGenerator<'a> {
    pub fn new(
        mask: &'a str,
        wordlists_fnames: &[&'a str],
        custom_charsets: &[&'a str],
    ) -> BoxResult<WordlistGenerator<'a>> {
        let mask_ops = parse_mask(mask)?;

        // TODO: split to functions
        let mut wordlists_data = vec![];
        for fname in wordlists_fnames.iter() {
            wordlists_data.push(Rc::new(Wordlist::from_file(fname)?));
        }

        // TODO: return error from custom_charset not in index & invalid symbol
        let items: Vec<WordlistItem> = mask_ops
            .into_iter()
            .map(|op| match op {
                MaskOp::Char(ch) => {
                    WordlistItem::Charset(Charset::from_chars(vec![ch as u8].as_ref()))
                }
                MaskOp::BuiltinCharset(ch) => WordlistItem::Charset(Charset::from_symbol(ch)),
                MaskOp::CustomCharset(idx) => {
                    WordlistItem::Charset(Charset::from_chars(custom_charsets[idx].as_bytes()))
                }
                MaskOp::Wordlist(idx) => WordlistItem::Wordlist(Rc::clone(&wordlists_data[idx])),
            })
            .collect();

        Ok(WordlistGenerator { mask, items })
    }

    #[allow(clippy::borrowed_box)]
    fn gen_words<'b>(&self, out: &mut Box<dyn Write + 'b>) -> Result<(), std::io::Error> {
        let mut buf = StackBuf::new();

        let mut word_buf = [b'\n'; MAX_WORD_SIZE];
        let word = &mut word_buf[..];
        let mut positions: Vec<_> = self
            .items
            .iter()
            .map(|item| match item {
                WordlistItem::Charset(charset) => Position::CharsetPos {
                    charset,
                    chr: charset.min_char,
                },
                WordlistItem::Wordlist(wordlist) => Position::WordlistPos {
                    wordlist,
                    idx: wordlist.iter(),
                },
            })
            .collect();

        let mut min_word = vec![];
        for pos in positions.iter_mut() {
            match pos {
                Position::CharsetPos { chr, .. } => min_word.push(*chr),
                Position::WordlistPos { idx, .. } => {
                    min_word.extend_from_slice(idx.next().unwrap())
                }
            }
        }
        min_word.push(b'\n');
        let min_word = min_word;
        let mut word_len = min_word.len();

        word[..word_len].copy_from_slice(&min_word);

        'outer_loop: loop {
            if buf.pos() + word_len >= buf.len() {
                out.write_all(buf.getdata())?;
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

                        // on debug build we have overflow checks
                        if cfg!(debug_assertions) && pos == 0 {
                            break 'outer_loop;
                        }
                        pos -= 1;
                    }
                    Position::WordlistPos { wordlist, idx } => {
                        let finished;
                        let prev_len = idx.current_len();
                        let wordlist_word = match idx.next() {
                            Some(w) => {
                                finished = false;
                                w
                            }
                            None => {
                                *idx = wordlist.iter();
                                finished = true;
                                idx.next().unwrap()
                            }
                        };

                        let wlen = wordlist_word.len();

                        // move the suffix by offset (can be negative)
                        if prev_len != wlen {
                            let offset = wlen as isize - prev_len as isize;

                            // copy by offset
                            for i in (pos + 1..word_len).rev() {
                                word[(i as isize + offset) as usize] = word[i];
                            }

                            // update current position & wordlien by offset
                            pos = (pos as isize + offset) as usize;
                            word_len = (word_len as isize + offset) as usize;
                        }

                        word[pos + 1 - wlen..=pos].copy_from_slice(wordlist_word);
                        // on debug build we have overflow checks
                        if cfg!(debug_assertions) && pos < wlen {
                            pos = 0;
                        } else {
                            pos -= wlen;
                        }

                        if !finished {
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

impl<'a> WordGenerator for WordlistGenerator<'a> {
    /// generates all words into the output buffer `out`
    fn gen<'b>(&self, out: Option<Box<dyn Write + 'b>>) -> Result<(), std::io::Error> {
        let mut out = out.unwrap_or_else(|| Box::new(io::stdout()));

        self.gen_words(&mut out)?;
        Ok(())
    }

    fn combinations(&self) -> BigUint {
        self.items
            .iter()
            .map(|item| match item {
                WordlistItem::Wordlist(wl) => wl.len().to_biguint().unwrap(),
                WordlistItem::Charset(c) => c.chars.len().to_biguint().unwrap(),
            })
            .product()
    }
}

#[cfg(test)]
mod tests {
    use super::{CharsetGenerator, WordGenerator};
    use crate::generators::get_word_generator;
    use num_bigint::{BigUint, ToBigUint};
    use std::fs;
    use std::io::Cursor;
    use std::path;

    #[test]
    fn test_gen_words_single_digit() {
        let mask = "?d";
        let word_gen = CharsetGenerator::new(mask, None, None, &vec![]).unwrap();

        assert_eq!(word_gen.mask, mask);
        assert_eq!(word_gen.minlen, 1);
        assert_eq!(word_gen.maxlen, 1);
        assert_eq!(word_gen.charsets.len(), 1);
        assert_eq!(word_gen.min_word, "0".as_bytes());

        let res = assert_gen(Box::new(word_gen), "single-digits.txt");

        // paranoid test of assert_gen
        assert_eq!(res, "0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
    }

    #[test]
    fn test_gen_upper_lower_1_4() {
        let mask = "?u?l?u?l";
        let word_gen = CharsetGenerator::new(mask, Some(1), None, &vec![]).unwrap();

        assert_eq!(word_gen.mask, mask);
        assert_eq!(word_gen.minlen, 1);
        assert_eq!(word_gen.maxlen, 4);
        assert_eq!(word_gen.charsets.len(), 4);
        assert_eq!(word_gen.min_word, "AaAa".as_bytes());

        assert_gen(Box::new(word_gen), "upper-lower-1-4.txt");
    }

    #[test]
    fn test_gen_pwd_upper_lower_year_1_4() {
        let mask = "pwd?u?l201?1";
        let word_gen = CharsetGenerator::new(mask, Some(1), None, &vec!["56789"]).unwrap();

        assert_eq!(word_gen.mask, mask);
        assert_eq!(word_gen.minlen, 1);
        assert_eq!(word_gen.maxlen, 9);
        assert_eq!(word_gen.charsets.len(), 9);
        assert_eq!(word_gen.min_word, "pwdAa2015".as_bytes());

        assert_gen(Box::new(word_gen), "upper-lower-year-1-4.txt");
    }

    #[test]
    fn test_invalid_custom_charset() {
        let result = CharsetGenerator::new("?1", None, None, &vec![]);

        assert_eq!(
            result.err().unwrap().to_string(),
            "mask contains ?1 charset but only 0 custom charsets defined",
        );
    }

    #[test]
    fn test_get_word_generator_charset() {
        let mask = "?d?d?d?d";
        let word_gen =
            get_word_generator(mask, Some(4), None, vec![].as_ref(), vec![].as_ref()).unwrap();
        assert_eq!(word_gen.combinations(), 10000.to_biguint().unwrap());
    }

    #[test]
    fn test_get_word_generator_wordlist() {
        let mask = "?d?d?d?d?w1";
        let wordlist_fname = wordlist_fname("wordlist1.txt");
        let wordlists = vec![wordlist_fname.to_str().unwrap()];
        let word_gen =
            get_word_generator(mask, None, None, vec![].as_ref(), wordlists.as_ref()).unwrap();
        assert_eq!(word_gen.combinations(), 100000.to_biguint().unwrap());
    }

    #[test]
    fn test_word_generator_wordlist_simple() {
        let mask = "?w1";
        let wordlist1 = wordlist_fname("wordlist1.txt");
        let wordlists = vec![wordlist1.to_str().unwrap()];
        let word_gen =
            get_word_generator(mask, None, None, vec![].as_ref(), wordlists.as_ref()).unwrap();

        assert_eq!(word_gen.combinations(), 10.to_biguint().unwrap());
        assert_gen(word_gen, "wordlist-simple.txt");
    }

    #[test]
    fn test_word_generator_wordlist_and_custom_charset() {
        let mask = "?w1?d?w2?l?w1?1";
        let wordlist1 = wordlist_fname("wordlist1.txt");
        let wordlist2 = wordlist_fname("wordlist2.txt");
        let charsets = vec!["!@#"];
        let wordlists = vec![wordlist1.to_str().unwrap(), wordlist2.to_str().unwrap()];
        let word_gen =
            get_word_generator(mask, None, None, charsets.as_ref(), wordlists.as_ref()).unwrap();

        assert_eq!(
            word_gen.combinations(),
            (10 * 10 * 12 * 26 * 10 * 3).to_biguint().unwrap()
        );
        assert_gen(word_gen, "wordlists-mix.txt");
    }

    fn assert_gen<'a>(w: Box<dyn WordGenerator + 'a>, fname: &str) -> String {
        let mut buf: Vec<u8> = Vec::new();
        let mut cur = Cursor::new(&mut buf);
        w.gen(Some(Box::new(&mut cur))).unwrap();

        let result = String::from_utf8(buf).unwrap();
        let expected = fs::read_to_string(wordlist_fname(fname)).unwrap();

        let mut s2 = fname.to_owned();
        s2.push_str("_expected.txt");
        fs::write(wordlist_fname(&s2), &result).unwrap();
        assert_eq!(result, expected);
        result
    }

    fn wordlist_fname(fname: &str) -> path::PathBuf {
        let mut d = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.extend(vec!["test-resources", fname]);
        d
    }

    #[test]
    fn test_gen_stats() {
        let custom_charsets = vec!["abcd", "01"];
        let combinations = vec![
            ("?d?s?u?l?a?b", "5368197120", None, None),
            ("?d?d?d?d?d?d?d?d", "111111110", Some(1), Some(8)),
            ("?d?d?d?d?d?d?d?d", "10000", Some(4), Some(4)),
            ("?d?d?d?d?d?d?d?d", "100000000", None, Some(8)),
            ("?1?2", "8", None, None),
            ("?1?2abc", "8", None, None),
            ("?d?1?2", "80", None, None),
            ("?d?s?u?l?a?b?1?2", "42945576960", None, None),
            ("?d?1?2?d", "930", Some(1), None),
            (
                "?b?b?b?b?b?b?b?b?b?b",
                "1208925819614629174706176",
                None,
                None,
            ),
        ];

        for (mask, result, minlen, maxlen) in combinations {
            let word_gen = CharsetGenerator::new(mask, minlen, maxlen, &custom_charsets).unwrap();
            assert_eq!(
                word_gen.combinations(),
                BigUint::parse_bytes(result.as_bytes(), 10).unwrap()
            );
        }
    }
}
