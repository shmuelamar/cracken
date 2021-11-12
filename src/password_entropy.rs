use std::collections::HashSet;
use std::fs::File;
use std::path::Path;

use ordered_float::OrderedFloat;
use pathfinding::astar;
use simple_error::SimpleError;

use crate::charsets::SYMBOL2CHARSET;
use crate::helpers::RawFileReader;
use crate::BoxResult;

const SYMBOLS_SPACE: &[u8; 32] = b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

pub struct EntropyEstimator {
    words: Vec<(String, HashSet<Vec<u8>>)>,
}

#[derive(PartialEq, Debug)]
pub struct PasswordEntropyResult {
    pub mask_entropy: f64,
    pub subword_entropy: f64,
    pub min_subword_mask: String,
    pub subword_entropy_min_split: Vec<String>,
}

impl EntropyEstimator {
    pub fn from_files<P: AsRef<Path>>(filenames: &[P]) -> BoxResult<Self> {
        let mut words = Vec::with_capacity(filenames.len() + SYMBOL2CHARSET.len());

        for charset in SYMBOL2CHARSET {
            let set = charset
                .chars
                .iter()
                .map(|ch| vec![ch.to_owned()])
                .collect::<HashSet<_>>();
            words.push((charset.symbol.to_string(), set));
        }

        for (i, filename) in filenames.iter().enumerate() {
            words.push((format!("w{}", i + 1), Self::load_vocab(filename)?));
        }

        words.sort_by_key(|(_, set)| set.len());
        Ok(EntropyEstimator { words })
    }

    pub fn estimate_password_entropy(&self, pwd: &[u8]) -> BoxResult<PasswordEntropyResult> {
        let (subword_entropy, subword_entropy_min_split, min_subword_mask) =
            self.compute_password_subword_entropy(pwd)?;
        let mask_entropy = password_mask_entropy(pwd);
        Ok(PasswordEntropyResult {
            mask_entropy,
            subword_entropy,
            min_subword_mask,
            subword_entropy_min_split,
        })
    }

    pub fn compute_password_subword_entropy(
        &self,
        pwd: &[u8],
    ) -> BoxResult<(f64, Vec<String>, String)> {
        // load vocab file
        let amatch = astar(
            &0usize,
            |&n| {
                let mut neighbours = vec![];
                for (_, set) in self.words.iter() {
                    for i in (n..=pwd.len()).rev() {
                        if set.contains(&pwd[n..i]) {
                            neighbours.push((i, OrderedFloat::<f64>((set.len() as f64).log2())));
                        }
                    }
                }
                neighbours
            },
            |_| OrderedFloat::<f64>(0f64),
            |&n| n == pwd.len(),
        );
        let (best_path, entropy) =
            amatch.ok_or_else(|| SimpleError::new("bad characters in password"))?;

        let mut best_split = Vec::with_capacity(best_path.len() - 1);
        let mut best_mask = String::with_capacity(best_path.len() - 1);
        let mut prev = 0usize;
        for i in best_path.into_iter().skip(1) {
            let word_i = &pwd[prev..i];
            let mut found = false;
            for (symbol, set) in self.words.iter() {
                if set.contains(word_i) {
                    found = true;
                    best_mask.push('?');
                    best_mask.push_str(symbol.as_str());
                    break;
                }
            }
            if !found {
                panic!("cannot find a matched subword {:?}", word_i);
            }
            best_split.push(String::from_utf8_lossy(word_i).to_string());
            prev = i;
        }
        Ok((entropy.into_inner(), best_split, best_mask))
    }

    fn load_vocab<P: AsRef<Path>>(fname: P) -> BoxResult<HashSet<Vec<u8>>> {
        let mut words: HashSet<_> = HashSet::new();

        let file = File::open(fname)?;
        let reader = RawFileReader::new(file);
        for word in reader
            .into_iter()
            .filter(|s| s.is_err() || !s.as_ref().unwrap().is_empty())
        {
            let mut word = word?;
            word.shrink_to_fit();
            words.insert(word);
        }

        words.shrink_to_fit();
        Ok(words)
    }
}

pub fn password_mask_entropy(pwd: &[u8]) -> f64 {
    pwd.iter()
        .map(|ch| {
            if ch.is_ascii_digit() {
                10f64.log2()
            } else if ch.is_ascii_alphabetic() {
                26f64.log2()
            } else if SYMBOLS_SPACE.contains(ch) {
                (SYMBOLS_SPACE.len() as f64).log2()
            } else {
                256f64.log2()
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use crate::password_entropy::EntropyEstimator;
    use crate::password_entropy::{password_mask_entropy, PasswordEntropyResult};
    use crate::test_util::wordlist_fname;

    #[test]
    fn test_compute_password_entropy() {
        let fname = wordlist_fname("vocab.txt");
        let pwd = "helloworld123!";
        let est = EntropyEstimator::from_files(vec![fname].as_ref()).unwrap();
        let res = est
            .compute_password_subword_entropy(pwd.as_bytes())
            .unwrap();
        assert_eq!(
            res,
            (
                30.823060867312257,
                vec!["helloworld", "123", "!"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                "?w1?d?d?d?s".to_string(),
            ),
        );
    }

    #[test]
    fn test_compute_password_entropy_long_password() {
        let pwd = "helloworld123!helloworld123!helloworld123!";
        let fname = wordlist_fname("vocab.txt");
        let est = EntropyEstimator::from_files(vec![fname].as_ref()).unwrap();
        let min_split = vec![
            "helloworld",
            "123",
            "!",
            "helloworld",
            "123",
            "!",
            "helloworld",
            "123",
            "!",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<String>>();
        let res = est
            .compute_password_subword_entropy(pwd.as_bytes())
            .unwrap();
        assert_eq!(
            res,
            (
                92.46918260193678,
                min_split.to_vec(),
                "?w1?d?d?d?s?w1?d?d?d?s?w1?d?d?d?s".to_string()
            ),
        );
        assert_eq!(
            est.estimate_password_entropy(pwd.as_bytes()).unwrap(),
            PasswordEntropyResult {
                mask_entropy: 185.91054439821917,
                subword_entropy: 92.46918260193678,
                min_subword_mask: "?".to_string(),
                subword_entropy_min_split: min_split,
            }
        );
    }

    #[test]
    fn test_compute_password_entropy_random_password() {
        let pwd = "E93gtaaE6yF7xDOWv3ww2QE6qD-Wye4mk8O3Vaerem8";
        let fname = wordlist_fname("vocab.txt");
        let min_split = vec![
            "E", "9", "3", "g", "t", "a", "a", "E", "6", "y", "F", "7", "x", "DOW", "v", "3", "w",
            "w", "2", "QE", "6", "q", "D-", "W", "y", "e", "4", "m", "k", "8", "O", "3", "V", "a",
            "e", "r", "e", "m", "8",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<String>>();
        let est = EntropyEstimator::from_files(vec![fname].as_ref()).unwrap();
        let res = est
            .compute_password_subword_entropy(pwd.as_bytes())
            .unwrap();
        assert_eq!(
            res,
            (206.14950164576396, min_split.to_vec(), "?u".to_string()),
        );
        assert_eq!(
            est.estimate_password_entropy(pwd.as_bytes()).unwrap(),
            PasswordEntropyResult {
                mask_entropy: 187.25484030613498,
                subword_entropy: 206.14950164576396,
                min_subword_mask: "?".to_string(),
                subword_entropy_min_split: min_split,
            }
        );
    }

    #[test]
    fn test_password_mask_cost() {
        let cases: Vec<(&str, f64)> = vec![
            ("Aa123456!", 34.33244800560635),
            ("0123456789", 33.219280948873624),
            ("😃", 32.0),
            ("!@#$%^&*()", 50.0),
            (
                "E93gtaaE6yF7xDOWv3ww2QE6qD-Wye4mk8O3Vaerem8",
                187.25484030613498,
            ),
        ];
        for (pwd, expected_cost) in cases {
            assert_eq!(password_mask_entropy(pwd.as_bytes()), expected_cost);
        }
    }
}
