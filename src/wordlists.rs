use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::Index;

#[derive(Debug)]
pub struct Wordlist {
    words: Vec<Vec<u8>>,
}

// TODO: size + next
impl Wordlist {
    pub fn from_file(fname: &str) -> std::io::Result<Wordlist> {
        let numlines = {
            let fp = BufReader::new(File::open(fname)?);
            fp.split(b'\n').count()
        };

        let fp = BufReader::new(File::open(fname)?);
        let mut words = Vec::with_capacity(numlines + 1);

        fp.split(b'\n')
            .map(|word| {
                let mut word = word?;
                if !word.is_empty() {
                    if *word.last().unwrap() == b'\n' {
                        word.pop();
                    }
                    word.shrink_to_fit();
                    words.push(word);
                }
                Ok(())
            })
            .collect::<Result<(), std::io::Error>>()?;
        words.shrink_to_fit();
        words.sort_unstable_by(|a, b| a.len().cmp(&b.len()));
        Ok(Wordlist { words })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.words.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Index<usize> for Wordlist {
    type Output = Vec<u8>;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.words[index]
    }
}

#[cfg(test)]
mod tests {
    use std::path;

    use super::Wordlist;

    #[test]
    fn test_wordlist_from_file() {
        let wordlist = Wordlist::from_file(&wordlist_fname("wordlist1.txt")).unwrap();

        println!(
            "{:?}",
            wordlist
                .words
                .into_iter()
                .map(|c| String::from_utf8(c.to_vec()).unwrap())
                .collect::<Vec<_>>()
        );
    }

    fn wordlist_fname(fname: &str) -> String {
        let mut d = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.extend(vec!["test-resources", fname]);
        d.to_str().unwrap().to_owned()
    }
}
