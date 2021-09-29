use aho_corasick::AhoCorasick;
use num_bigint::{BigUint, ToBigUint};
use ordered_float::OrderedFloat;
use pathfinding::astar;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn compute_password_entropy(pwd: &str) -> BigUint {
    // load vocab file
    let word2rank = load_vocab("/home/samar/dev/cracken/vocab.txt");

    // for run_id in 0..10000 {
    let ac = AhoCorasick::new(word2rank.keys());

    let mut matches = HashMap::new();
    for mat in ac.find_overlapping_iter(pwd) {
        matches
            .entry(mat.start())
            .or_insert_with(Vec::new)
            .push(mat);
    }
    let empty_vec = Vec::with_capacity(0);
    let amatch = astar(
        &0usize,
        |&n| {
            matches
                .get(&n)
                .unwrap_or_else(|| empty_vec.as_ref())
                .iter()
                .map(|ni| {
                    (
                        ni.end(),
                        OrderedFloat::<f64>((word2rank[&pwd[ni.start()..ni.end()]] as f64).log2()),
                    )
                })
        },
        |_| OrderedFloat::<f64>(0f64), // TODO: can we add good heuristic?
        |&n| n == pwd.len(),
    );
    let (best_path, _) = amatch.expect("invalid chars in password"); // TODO: raise

    let mut best_split = Vec::with_capacity(best_path.len() - 1);
    let mut best_cost: BigUint = 1.to_biguint().unwrap();
    let mut prev = 0usize;
    for i in best_path.into_iter().skip(1) {
        let word_i = &pwd[prev..i];
        best_split.push(word_i);
        prev = i;
        best_cost *= word2rank[word_i];
    }
    best_cost
}

fn load_vocab(fname: &str) -> HashMap<String, usize> {
    let file = File::open(fname).expect("file not found!"); // TODO: raise
    let reader = BufReader::new(file);
    let mut vocab: HashMap<String, usize> = reader
        .lines()
        .filter_map(|s| s.ok())
        .filter(|s| !s.is_empty())
        .enumerate()
        .map(|(i, s)| (s, i + 1))
        .collect();
    vocab.shrink_to_fit();
    vocab
}

#[cfg(test)]
mod tests {
    use crate::password_entropy;
    use num_bigint::ToBigUint;

    #[test]
    fn test_run_smoke() {
        let pwd = "helloworld123!";
        assert_eq!(
            password_entropy::compute_password_entropy(pwd),
            1899616264.to_biguint().unwrap()
        );
    }
}
