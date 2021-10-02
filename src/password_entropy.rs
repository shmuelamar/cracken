use crate::BoxResult;
use num_bigint::{BigUint, ToBigUint};
use ordered_float::OrderedFloat;
use pathfinding::astar;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

const SYMBOLS_SPACE: &[u8; 32] = b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

pub fn compute_password_entropy(pwd: &str) -> BoxResult<(BigUint, Vec<String>)> {
    // load vocab file
    let word2rank = load_vocab("/home/samar/dev/cracken/vocab.txt")?;
    let raw_pwd = pwd.as_bytes();
    let amatch = astar(
        &0usize,
        |&n| {
            (n..=raw_pwd.len())
                .rev()
                .filter_map(|i| {
                    word2rank
                        .get(&raw_pwd[n..i])
                        .map(|rank| (i, OrderedFloat::<f64>((*rank as f64).log2())))
                })
                .collect::<Vec<_>>()
        },
        |_| OrderedFloat::<f64>(0f64),
        |&n| n == raw_pwd.len(),
    );

    let best_path = match amatch {
        Some((path, _)) => path,
        None => bail!("bad characters in password"),
    };

    let mut best_split = Vec::with_capacity(best_path.len() - 1);
    let mut best_cost: BigUint = 1.to_biguint().unwrap();
    let mut prev = 0usize;
    for i in best_path.into_iter().skip(1) {
        let word_i = &raw_pwd[prev..i];
        best_split.push(String::from_utf8_lossy(word_i).to_string());
        prev = i;
        best_cost *= word2rank[word_i];
    }
    Ok((best_cost, best_split))
}

pub fn password_mask_cost(pwd: &str) -> BigUint {
    pwd.bytes()
        .into_iter()
        .map(|ch| {
            if ch.is_ascii_digit() {
                10.to_biguint().unwrap()
            } else if ch.is_ascii_alphabetic() {
                26.to_biguint().unwrap()
            } else if SYMBOLS_SPACE.contains(&ch) {
                SYMBOLS_SPACE.len().to_biguint().unwrap()
            } else {
                256.to_biguint().unwrap()
            }
        })
        .product()
}

fn load_vocab(fname: &str) -> BoxResult<HashMap<Vec<u8>, usize>> {
    let file = File::open(fname)?;
    let mut reader = BufReader::new(file);
    let mut buffer: Vec<u8> = Vec::with_capacity(256);
    let mut word2rank: HashMap<Vec<u8>, usize> = HashMap::new();

    let mut rank = 1;

    loop {
        match reader.read_until(b'\n', &mut buffer)? {
            0 => break,
            _ => {
                if buffer.pop().is_some() {
                    let mut word = buffer.to_vec();
                    word.shrink_to_fit();
                    word2rank.insert(word, rank);
                    rank += 1;
                };
                buffer.clear();
            }
        }
    }

    let missing_rank = word2rank.len() + 1;
    for ch in 0..=255u8 {
        word2rank.entry(vec![ch]).or_insert(missing_rank);
    }

    word2rank.shrink_to_fit();
    Ok(word2rank)
}

#[cfg(test)]
mod tests {
    use crate::password_entropy;
    use crate::password_entropy::password_mask_cost;
    use num_bigint::{BigUint, ToBigUint};

    #[test]
    fn test_compute_password_entropy() {
        let pwd = "helloworld123!";
        let res = password_entropy::compute_password_entropy(pwd).unwrap();
        assert_eq!(
            res,
            (
                1899616264.to_biguint().unwrap(),
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
        let res = password_entropy::compute_password_entropy(pwd).unwrap();
        assert_eq!(
            res,
            (
                BigUint::parse_bytes(b"6854844978407404468080607744", 10).unwrap(),
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
    }

    #[test]
    fn test_compute_password_entropy_random_password() {
        let pwd = "E93gtaaE6yF7xDOWv3ww2QE6qD-Wye4mk8O3Vaerem8";
        let res = password_entropy::compute_password_entropy(pwd).unwrap();
        assert_eq!(
            res,
            (
                BigUint::parse_bytes(
                    b"114073190002634044113434716879079970003599839404826329203343360",
                    10
                )
                .unwrap(),
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
    }

    #[test]
    fn test_password_mask_cost() {
        let cases: Vec<(&str, &[u8])> = vec![
            ("Aa123456!", b"21632000000"),
            ("0123456789", b"10000000000"),
            ("😃", b"4294967296"),
            ("!@#$%^&*()", b"1125899906842624"),
            (
                "E93gtaaE6yF7xDOWv3ww2QE6qD-Wye4mk8O3Vaerem8",
                b"234058148586890860482317269664048830525682483200000000000",
            ),
        ];
        for (pwd, expected_cost) in cases {
            let cost = BigUint::parse_bytes(expected_cost, 10).unwrap();
            assert_eq!(password_mask_cost(pwd), cost);
        }
    }
}
