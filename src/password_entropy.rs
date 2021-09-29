use crate::BoxResult;
use num_bigint::{BigUint, ToBigUint};
use ordered_float::OrderedFloat;
use pathfinding::astar;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn compute_password_entropy(pwd: &str) -> BoxResult<BigUint> {
    // load vocab file
    let word2rank = load_vocab("/home/samar/dev/cracken/vocab.txt")?;

    let amatch = astar(
        &0usize,
        |n| {
            (*n..=pwd.len())
                .filter_map(|i| {
                    let prefix_i = &pwd[*n..i];
                    word2rank
                        .get(prefix_i)
                        .map(|rank| (i, OrderedFloat::<f64>((*rank as f64).log2())))
                })
                .collect::<Vec<_>>()
        },
        |_| OrderedFloat::<f64>(0f64), // TODO: can we add good heuristic?
        |&n| n == pwd.len(),
    );

    let best_path;
    match amatch {
        Some(m) => {
            best_path = m.0;
        }
        None => {
            bail!("bad characters in password");
        }
    };

    let mut best_split = Vec::with_capacity(best_path.len() - 1);
    let mut best_cost: BigUint = 1.to_biguint().unwrap();
    let mut prev = 0usize;
    for i in best_path.into_iter().skip(1) {
        let word_i = &pwd[prev..i];
        best_split.push(word_i);
        prev = i;
        best_cost *= word2rank[word_i];
    }
    Ok(best_cost)
}

fn load_vocab(fname: &str) -> BoxResult<HashMap<String, usize>> {
    let file = File::open(fname)?;
    let reader = BufReader::new(file);
    let mut vocab: HashMap<String, usize> = reader
        .lines()
        .filter_map(|s| s.ok())
        .filter(|s| !s.is_empty())
        .enumerate()
        .map(|(i, s)| (s, i + 1))
        .collect();
    vocab.shrink_to_fit();
    Ok(vocab)
}

#[cfg(test)]
mod tests {
    use crate::password_entropy;
    use num_bigint::ToBigUint;

    #[test]
    fn test_run_smoke() {
        let pwd = "helloworld123!";
        assert_eq!(
            password_entropy::compute_password_entropy(pwd).unwrap(),
            1899616264.to_biguint().unwrap()
        );
    }
}
