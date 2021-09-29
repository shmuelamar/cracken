use crate::BoxResult;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// a buffer containing words of the same length
#[derive(Debug)]
struct WordsBuf {
    len: usize,
    words: Vec<u8>,
}

#[derive(Debug)]
pub struct Wordlist {
    words_bufs: Vec<WordsBuf>,
}

pub struct WordlistIterator<'a> {
    wordlist: &'a Wordlist,
    wordbuf_pos: usize,
    word_pos: usize,
}

impl Wordlist {
    pub fn from_file(fname: &str) -> BoxResult<Wordlist> {
        let fp = BufReader::new(File::open(fname)?);
        let mut len2words = HashMap::new();

        fp.split(b'\n')
            .try_for_each::<_, Result<(), std::io::Error>>(|word| {
                let mut word = word?;
                if !word.is_empty() {
                    if word.last() == Some(&b'\n') {
                        word.pop();
                    }

                    let lenvec = len2words.entry(word.len()).or_insert_with(Vec::new);
                    lenvec.extend_from_slice(&word);

                    // avoid over allocating memory for large wordlists
                    lenvec.reserve_exact(word.len() * 1024 * 1024);
                }
                Ok(())
            })?;

        len2words
            .iter_mut()
            .for_each(|(_, words)| words.shrink_to_fit());

        let mut words_bufs: Vec<_> = len2words
            .into_iter()
            .map(|(len, words)| WordsBuf { len, words })
            .collect();

        words_bufs.sort_unstable_by(|a, b| a.len.cmp(&b.len));
        Ok(Wordlist { words_bufs })
    }

    #[inline]
    pub fn iter(&self) -> WordlistIterator {
        WordlistIterator {
            wordlist: self,
            wordbuf_pos: 0,
            word_pos: 0,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.words_bufs
            .iter()
            .map(|wb| wb.words.len() / wb.len)
            .sum()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a> Iterator for WordlistIterator<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let mut word_buf = &self.wordlist.words_bufs[self.wordbuf_pos];

        // advance next word
        if self.word_pos < word_buf.words.len() {
            let prev_word_pos = self.word_pos;
            self.word_pos += word_buf.len;

            Some(&word_buf.words[prev_word_pos..self.word_pos])

        // finished the current wordbuf - advance to next one
        } else if self.wordbuf_pos < self.wordlist.words_bufs.len() - 1 {
            self.wordbuf_pos += 1;
            word_buf = &self.wordlist.words_bufs[self.wordbuf_pos];
            self.word_pos = word_buf.len;

            Some(&word_buf.words[..self.word_pos])

        // finished all wordbufs on the wordlist - no next item
        } else {
            None
        }
    }
}

impl<'a> WordlistIterator<'a> {
    /// returns the current length of word of this iterator
    #[inline]
    pub fn current_len(&self) -> usize {
        let word_buf = &self.wordlist.words_bufs[self.wordbuf_pos];
        word_buf.len
    }
}

#[cfg(test)]
mod tests {
    use std::path;

    use super::Wordlist;

    #[test]
    fn test_wordlist_from_file() {
        let wordlist = Wordlist::from_file(&wordlist_fname("wordlist1.txt")).unwrap();

        let words = wordlist
            .iter()
            .map(|c| String::from_utf8(c.to_vec()).unwrap())
            .collect::<Vec<_>>();

        let expected: Vec<_> = r#"12345
123456
qwerty
123123
111111
abc123
1234567
password
12345678
123456789"#
            .split("\n")
            .map(|s| s.to_owned())
            .collect();
        assert_eq!(words, expected);
    }

    fn wordlist_fname(fname: &str) -> String {
        let mut d = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.extend(vec!["test-resources", fname]);
        d.to_str().unwrap().to_owned()
    }
}
