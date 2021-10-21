use std::io::{BufRead, BufReader, Error, Read};

pub struct RawFileReader<R> {
    reader: BufReader<R>,
    buffer: Vec<u8>,
}

impl<R: Read> RawFileReader<R> {
    pub fn new(reader: R) -> RawFileReader<R> {
        RawFileReader {
            reader: BufReader::new(reader),
            buffer: Vec::with_capacity(256),
        }
    }
}

impl<R: Read> Iterator for RawFileReader<R> {
    type Item = Result<Vec<u8>, Error>;

    fn next(&mut self) -> Option<Result<Vec<u8>, Error>> {
        self.buffer.clear();
        match self.reader.read_until(b'\n', &mut self.buffer) {
            Ok(0) => None,
            Ok(_) => {
                self.buffer.pop();
                Some(Ok(self.buffer.to_vec()))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

/// creates validator function for one of the given allowed_values
#[macro_export]
macro_rules! one_of_validator {
    ($allowed_values:expr, $error_msg: expr) => {
        |s| {
            if $allowed_values.contains(&s.as_str()) {
                Ok(())
            } else {
                Err($error_msg.to_string())
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use crate::helpers::RawFileReader;
    use crate::test_util::wordlist_fname;

    #[test]
    fn test_reader() {
        let file = File::open(wordlist_fname("vocab.txt")).unwrap();
        let reader = RawFileReader::new(file);
        let expected: Vec<_> = vec!["a", "e", "1", "i", "o"]
            .iter()
            .map(|s| s.as_bytes())
            .collect();
        let lines = reader.take(5).map(|s| s.unwrap()).collect::<Vec<_>>();
        assert_eq!(lines, expected);
    }
}
