use std::collections::HashMap;
use std::fs::File;

use ordered_float::OrderedFloat;
use pathfinding::astar;
use simple_error::SimpleError;

use crate::helpers::RawFileReader;
use crate::BoxResult;

const SYMBOLS_SPACE: &[u8; 32] = b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

pub struct EntropyEstimator {
    word2entropy: HashMap<Vec<u8>, OrderedFloat<f64>>,
}

impl EntropyEstimator {
    pub fn from_file(filename: &str) -> BoxResult<Self> {
        Ok(EntropyEstimator {
            word2entropy: Self::load_vocab(filename)?,
        })
    }

    pub fn compute_password_min_entropy(&self, pwd: &[u8]) -> BoxResult<f64> {
        let cost1 = self.compute_password_subword_entropy(pwd)?.0;
        let cost2 = password_mask_cost(pwd);
        Ok(cost1.min(cost2))
    }

    pub fn compute_password_subword_entropy(&self, pwd: &[u8]) -> BoxResult<(f64, Vec<String>)> {
        // load vocab file
        let amatch = astar(
            &0usize,
            |&n| {
                (n..=pwd.len())
                    .rev()
                    .filter_map(|i| {
                        self.word2entropy
                            .get(&pwd[n..i])
                            .map(|word_entropy| (i, word_entropy.to_owned()))
                    })
                    .collect::<Vec<_>>()
            },
            |_| OrderedFloat::<f64>(0f64),
            |&n| n == pwd.len(),
        );
        let (best_path, entropy) =
            amatch.ok_or_else(|| SimpleError::new("bad characters in password"))?;

        let mut best_split = Vec::with_capacity(best_path.len() - 1);
        let mut prev = 0usize;
        for i in best_path.into_iter().skip(1) {
            let word_i = &pwd[prev..i];
            best_split.push(String::from_utf8_lossy(word_i).to_string());
            prev = i;
        }
        Ok((entropy.into_inner(), best_split))
    }

    fn load_vocab(fname: &str) -> BoxResult<HashMap<Vec<u8>, OrderedFloat<f64>>> {
        let mut word2rank: HashMap<_, _> = HashMap::new();

        let file = File::open(fname)?;
        let reader = RawFileReader::new(file);
        for (rank, word) in reader
            .into_iter()
            .filter(|s| s.is_err() || !s.as_ref().unwrap().is_empty())
            .enumerate()
        {
            let mut word = word?;
            word.shrink_to_fit();
            word2rank.insert(word, OrderedFloat::<f64>(((rank + 1) as f64).log2()));
        }

        let missing_rank = OrderedFloat::<f64>(((word2rank.len() + 1) as f64).log2());
        for ch in 0..=255u8 {
            word2rank.entry(vec![ch]).or_insert(missing_rank);
        }

        word2rank.shrink_to_fit();
        Ok(word2rank)
    }
}

pub fn password_mask_cost(pwd: &[u8]) -> f64 {
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
    use crate::password_entropy::password_mask_cost;
    use crate::password_entropy::EntropyEstimator;
    const SMARTLIST_FILENAME: &str = "/home/samar/dev/cracken/vocab.txt";

    #[test]
    fn test_compute_password_entropy() {
        let pwd = "helloworld123!";
        let est = EntropyEstimator::from_file(SMARTLIST_FILENAME).unwrap();
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
                    .collect()
            ),
        );
    }

    #[test]
    fn test_compute_password_entropy_long_password() {
        let pwd = "helloworld123!helloworld123!helloworld123!";
        let est = EntropyEstimator::from_file(SMARTLIST_FILENAME).unwrap();
        let res = est
            .compute_password_subword_entropy(pwd.as_bytes())
            .unwrap();
        assert_eq!(
            res,
            (
                92.46918260193678,
                vec![
                    "helloworld",
                    "123",
                    "!",
                    "helloworld",
                    "123",
                    "!",
                    "helloworld",
                    "123",
                    "!"
                ]
                .into_iter()
                .map(String::from)
                .collect()
            ),
        );
        assert_eq!(
            est.compute_password_min_entropy(pwd.as_bytes()).unwrap(),
            92.46918260193678
        );
    }

    #[test]
    fn test_compute_password_entropy_random_password() {
        let pwd = "E93gtaaE6yF7xDOWv3ww2QE6qD-Wye4mk8O3Vaerem8";
        let est = EntropyEstimator::from_file(SMARTLIST_FILENAME).unwrap();
        let res = est
            .compute_password_subword_entropy(pwd.as_bytes())
            .unwrap();
        assert_eq!(
            res,
            (
                206.14950164576396,
                vec![
                    "E", "9", "3", "g", "t", "a", "a", "E", "6", "y", "F", "7", "x", "DOW", "v",
                    "3", "w", "w", "2", "QE", "6", "q", "D-", "W", "y", "e", "4", "m", "k", "8",
                    "O", "3", "V", "a", "e", "r", "e", "m", "8"
                ]
                .into_iter()
                .map(String::from)
                .collect()
            ),
        );
        assert_eq!(
            est.compute_password_min_entropy(pwd.as_bytes()).unwrap(),
            187.25484030613498
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
            assert_eq!(password_mask_cost(pwd.as_bytes()), expected_cost);
        }
    }
}
