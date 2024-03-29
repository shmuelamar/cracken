use std::collections::HashSet;
use std::fs::File;
use std::hash::Hash;
use std::io::{BufRead, BufReader};
use std::iter::FromIterator;
use std::path::Path;

use aho_corasick::AhoCorasick;
use itertools::Itertools;
use tokenizers::decoders::byte_level::ByteLevel;
use tokenizers::models::bpe::{BpeTrainerBuilder, BPE};
use tokenizers::models::unigram::{Unigram, UnigramTrainer};
use tokenizers::models::wordpiece::{WordPiece, WordPieceTrainerBuilder};
use tokenizers::normalizers::{Sequence, StripAccents, NFD};
use tokenizers::pre_tokenizers::delimiter::CharDelimiterSplit;
use tokenizers::{
    Decoder, Model, Normalizer, PostProcessor, PreTokenizer, TokenizerBuilder, TokenizerImpl,
    Trainer,
};

use crate::BoxResult;

pub const DEFAULT_VOCAB_SIZE: u32 = 50000;
pub const DEFAULT_MIN_FREQUENCY: u32 = 0;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone)]
pub enum SmartlistTokenizer {
    Unigram,
    BPE,
    WordPiece,
}

pub struct SmartlistBuilder<P: AsRef<Path>> {
    infiles: Vec<P>,
    tokenizers: HashSet<SmartlistTokenizer>,
    vocab_max_size: u32,
    min_frequency: u32,
    print_progress: bool,
    numbers_max_size: Option<u32>,
    min_word_len: u32,
}

impl<P: AsRef<Path> + Sync> Default for SmartlistBuilder<P> {
    fn default() -> Self {
        SmartlistBuilder {
            infiles: vec![],
            tokenizers: HashSet::from_iter([SmartlistTokenizer::Unigram]),
            vocab_max_size: DEFAULT_VOCAB_SIZE,
            min_frequency: DEFAULT_MIN_FREQUENCY,
            print_progress: true,
            numbers_max_size: None,
            min_word_len: 1,
        }
    }
}

impl<P: AsRef<Path> + Sync> SmartlistBuilder<P> {
    pub fn new() -> SmartlistBuilder<P> {
        SmartlistBuilder::default()
    }
    pub fn infiles(mut self, infiles: Vec<P>) -> Self {
        self.infiles = infiles;
        self
    }
    pub fn vocab_max_size(mut self, vocab_max_size: u32) -> Self {
        self.vocab_max_size = vocab_max_size;
        self
    }
    pub fn min_frequency(mut self, min_frequency: u32) -> Self {
        self.min_frequency = min_frequency;
        self
    }
    pub fn tokenizers(mut self, tokenizers: impl Iterator<Item = SmartlistTokenizer>) -> Self {
        self.tokenizers = HashSet::from_iter(tokenizers);
        self
    }
    pub fn print_progress(mut self, print_progress: bool) -> Self {
        self.print_progress = print_progress;
        self
    }
    pub fn numbers_max_size(mut self, numbers_max_size: Option<u32>) -> Self {
        self.numbers_max_size = numbers_max_size;
        self
    }
    pub fn min_word_len(mut self, min_word_len: u32) -> Self {
        self.min_word_len = min_word_len;
        self
    }

    pub fn build(&self) -> BoxResult<Vec<String>> {
        let mut vocab = HashSet::with_capacity(self.vocab_max_size as usize);
        let mut tokenizers_types = self.tokenizers.iter().collect::<Vec<_>>();
        tokenizers_types.sort_unstable();

        for tokenizer_type in tokenizers_types {
            let v = match tokenizer_type {
                SmartlistTokenizer::Unigram => self.train_unigram(),
                SmartlistTokenizer::BPE => self.train_bpe(),
                SmartlistTokenizer::WordPiece => self.train_wordpiece(),
            };
            match v {
                Ok(v) => {
                    vocab.extend(v);
                }
                Err(err) => {
                    bail!(err.to_string());
                }
            }
        }

        // dedup words
        let vocab = vocab.into_iter().unique().collect::<Vec<_>>();

        // sort by frequency of words in original input files
        let mut vocab = self.sort_vocab(vocab)?;

        // apply filters
        if self.min_word_len > 0 {
            vocab = remove_shorter_than_len(vocab, self.min_word_len as usize);
        }
        if let Some(numbers_max_size) = self.numbers_max_size {
            vocab = remove_long_numbers(vocab, numbers_max_size as usize);
        }

        // truncate to desired maxsize (or less)
        vocab.truncate(self.vocab_max_size as usize);
        Ok(vocab)
    }

    fn train_bpe(&self) -> tokenizers::Result<Vec<String>> {
        let model = BPE::default();
        let mut tokenizer = self.build_tokenizer(model)?;

        let mut trainer = BpeTrainerBuilder::new()
            .show_progress(self.print_progress)
            .vocab_size(self.vocab_max_size as usize)
            .min_frequency(self.min_frequency)
            .special_tokens(vec![])
            .build();
        self.train_tokenizer(&mut tokenizer, &mut trainer)
    }

    fn train_unigram(&self) -> tokenizers::Result<Vec<String>> {
        let model = Unigram::default();
        let mut tokenizer = self.build_tokenizer(model)?;

        let mut trainer = UnigramTrainer::builder()
            .vocab_size(self.vocab_max_size)
            .show_progress(self.print_progress)
            .build()?;
        self.train_tokenizer(&mut tokenizer, &mut trainer)
    }

    fn train_wordpiece(&self) -> tokenizers::Result<Vec<String>> {
        let model = WordPiece::default();
        let model = model;
        let mut tokenizer = self.build_tokenizer(model)?;

        let mut trainer = WordPieceTrainerBuilder::new()
            .show_progress(self.print_progress)
            .vocab_size(self.vocab_max_size as usize)
            .special_tokens(vec![])
            .build();
        let vocab = self.train_tokenizer(&mut tokenizer, &mut trainer)?;
        Ok(vocab
            .into_iter()
            .map(|word| {
                if let Some(stripped) = word.strip_prefix("##") {
                    stripped.to_string()
                } else {
                    word
                }
            })
            .collect::<Vec<String>>())
    }

    fn build_tokenizer<M: Model>(
        &self,
        model: M,
    ) -> Result<
        TokenizerImpl<M, Sequence, CharDelimiterSplit, ByteLevel, ByteLevel>,
        tokenizers::Error,
    > {
        TokenizerBuilder::new()
            .with_model(model)
            .with_normalizer(Some(Sequence::new(vec![NFD.into(), StripAccents.into()])))
            .with_pre_tokenizer(Some(CharDelimiterSplit::new('\n')))
            .with_post_processor(Some(ByteLevel::default()))
            .with_decoder(Some(ByteLevel::default()))
            .build()
    }

    fn train_tokenizer<M, N, PT, PP, D, TR>(
        &self,
        tokenizer: &mut TokenizerImpl<M, N, PT, PP, D>,
        trainer: &mut TR,
    ) -> tokenizers::Result<Vec<String>>
    where
        M: Model + Send + Sync,
        N: Normalizer + Send + Sync,
        PT: PreTokenizer + Send + Sync,
        PP: PostProcessor + Send + Sync,
        D: Decoder + Send + Sync,
        TR: Trainer<Model = M> + Sync,
    {
        let files: Result<Vec<_>, _> = self
            .infiles
            .iter()
            .map(|fname| File::open(fname).map(BufReader::new))
            .collect();
        let files = files?;
        let input_data = files
            .into_iter()
            .map(|fp| {
                fp.lines()
                    .map(|line| line.unwrap_or_else(|_| "".to_string()))
            })
            .flatten();

        tokenizer.train(trainer, input_data)?;
        let vocab = tokenizer.get_vocab(false).into_keys().collect::<Vec<_>>();
        Ok(vocab)
    }

    fn sort_vocab(&self, vocab: Vec<String>) -> BoxResult<Vec<String>> {
        let ac = AhoCorasick::new(vocab.to_vec());
        let mut word2count = vec![0i64; vocab.len()];

        let files: Result<Vec<_>, _> = self
            .infiles
            .iter()
            .map(|fname| File::open(fname).map(BufReader::new))
            .collect();
        let files = files?;
        let input_data = files
            .into_iter()
            .map(|f| {
                f.lines()
                    .map(|line| line.unwrap_or_else(|_| "".to_string()))
            })
            .flatten();

        for line in input_data {
            for mat in ac.find_overlapping_iter(&line) {
                let word = mat.pattern();
                word2count[word] += 1;
            }
        }

        Ok(vocab
            .into_iter()
            .enumerate()
            .sorted_by_key(|(idx, s)| (-(word2count[*idx] as i64), s.to_string()))
            .map(|(_, s)| s)
            .collect::<Vec<_>>())
    }
}

pub fn remove_long_numbers(vocab: Vec<String>, max_len: usize) -> Vec<String> {
    vocab
        .into_iter()
        .filter(|s| !s.chars().all(char::is_numeric) || s.len() <= max_len)
        .collect()
}

pub fn remove_shorter_than_len(vocab: Vec<String>, min_word_len: usize) -> Vec<String> {
    vocab
        .into_iter()
        .filter(|s| s.len() >= min_word_len)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::create_smartlist::{SmartlistBuilder, SmartlistTokenizer};
    use crate::test_util;

    #[test]
    fn test_build_vocab() {
        let fname = test_util::wordlist_fname("wordlist1.txt");
        let mut vocab = SmartlistBuilder::new()
            .infiles(vec![fname.to_str().unwrap()])
            .min_frequency(0)
            .vocab_max_size(25)
            .tokenizers(
                vec![
                    SmartlistTokenizer::BPE,
                    SmartlistTokenizer::WordPiece,
                    SmartlistTokenizer::Unigram,
                ]
                .into_iter(),
            )
            .print_progress(true)
            .build()
            .unwrap();

        let mut expected_vocab: Vec<_> = [
            "1", "11", "12", "123", "12345", "123456", "2", "3", "4", "5", "6", "7", "8", "9", "a",
            "b", "c", "d", "e", "o", "p", "q", "r", "s", "w",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        expected_vocab.sort();
        vocab.sort();
        assert_eq!(vocab, expected_vocab);
    }
}
