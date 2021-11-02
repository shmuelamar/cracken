use std::env;
use std::fs::File;
use std::io::{BufWriter, ErrorKind, Write};

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

use crate::create_smartlist::{SmartlistBuilder, SmartlistTokenizer};
use crate::generators::get_word_generator;
use crate::helpers::RawFileReader;
use crate::one_of_validator;
use crate::password_entropy::EntropyEstimator;
use crate::{built_info, BoxResult};

const EXAMPLE_USAGE: &str = r#"Example Usage:

  # all digits from 00000000 to 99999999
  cracken ?d?d?d?d?d?d?d?d

  # all digits from 0 to 99999999
  cracken -m 1 ?d?d?d?d?d?d?d?d

  # words with pwd prefix - pwd0000 to pwd9999
  cracken pwd?d?d?d?d

  # all passwords of length 8 starting with upper then 6 lowers then digit
  cracken ?u?l?l?l?l?l?l?d

  # same as above, write output to pwds.txt instead of stdout
  cracken -o pwds.txt ?u?l?l?l?l?l?l?d

  # custom charset - all hex values
  cracken -c "0123456789abcdef" "?1?1?1?1"

  # 4 custom charsets - the order determines the id of the charset
  cracken -c "01" -c="ab" -c="de" -c="ef" "?1?2?3?4"

  # 4 lowercase chars with years 2000-2019 suffix
  cracken -c "01" "?l?l?l?l20?1?d"

  # starts with firstname from wordlist followed by 4 digits
  cracken -w "firstnames.txt" "?w1?d?d?d?d"

  # starts with firstname from wordlist with lastname from wordlist ending with symbol
  cracken -w "firstnames.txt" -w "lastnames.txt" -c "!@#$" "?w1?w2?1"

  # repeating wordlists multiple times and combining charsets
  cracken -w "verbs.txt" -w "nouns.txt" "?w1?w2?w1?w2?w2?d?d?d"

  # estimating entropy of a password
  cracken entropy --smartlist vocab.txt "helloworld123!"

  # estimating the entropy of a passwords file
  cracken entropy --smartlist vocab.txt -p passwords.txt
"#;

fn parse_args(args: Option<Vec<&str>>) -> ArgMatches<'static> {
    let osargs: Vec<String>;
    let mut args = match args {
        Some(itr) => itr,
        None => {
            osargs = env::args().collect();
            osargs.iter().map(|s| s.as_ref()).collect()
        }
    };

    // workaround for default subcommand
    if args.len() >= 2 && !vec!["generate", "entropy", "create", "--help"].contains(&args[1]) {
        args.insert(1, "generate");
    }

    App::new(format!(
        "Cracken v{} - {}",
        built_info::PKG_VERSION,
        built_info::PKG_DESCRIPTION
    )).setting(AppSettings::DisableHelpSubcommand)
        .setting(AppSettings::ArgRequiredElseHelp)
    .after_help(
        format!(
            "{}\n{}-v{} {}-{} compiler: {}\nmore info at: {}",
            EXAMPLE_USAGE,
            built_info::PKG_NAME,
            built_info::PKG_VERSION,
            built_info::CFG_OS,
            built_info::CFG_TARGET_ARCH,
            built_info::RUSTC_VERSION,
            built_info::PKG_HOMEPAGE,
        )
        .as_str())
        .subcommand(SubCommand::with_name("generate")
        .about("(default) - Generates newline separated words according to given mask and wordlist files")
        .display_order(0)
    .arg(
        Arg::with_name("mask")
            .long_help(
                r#"the wordlist mask to generate.
available masks are:
    builtin charsets:
    ?d - digits: "0123456789"
    ?l - lowercase: "abcdefghijklmnopqrstuvwxyz"
    ?u - uppercase: "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    ?s - symbols: " !\"\#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
    ?a - all characters: "?d?l?u?s"
    ?b - all binary values: (0-255)

    custom charsets ?1 to ?9:
    ?1 - first custom charset specified by --charset 'mychars'

    wordlists ?w1 to ?w9:
    ?w1 - first wordlist specified by --wordlist 'my-wordlist.txt'
"#,
            )
            .takes_value(true)
            .required(true),
    )
    .arg(
        Arg::with_name("min-length")
            .short("m")
            .long("minlen")
            .help("minimum length of the mask to start from")
            .takes_value(true)
            .required(false),
    )
    .arg(
        Arg::with_name("max-length")
            .short("x")
            .long("maxlen")
            .help("maximum length of the mask to start from")
            .takes_value(true)
            .required(false),
    )
    .arg(
        Arg::with_name("stats")
            .short("s")
            .long("stats")
            .help("prints the number of words this command will generate and exits")
            .takes_value(false)
            .required(false),
    ).arg(
        Arg::with_name("custom-charset")
            .short("c")
            .long("custom-charset")
            .help("custom charset (string of chars). up to 9 custom charsets - ?1 to ?9. use ?1 on the mask for the first charset")
            .takes_value(true)
            .required(false)
            .multiple(true)
            .number_of_values(1)
            .max_values(9),
    )
    .arg(
        Arg::with_name("wordlist")
            .short("w")
            .long("wordlist")
            .help("filename containing newline (0xA) separated words. note: currently all wordlists loaded to memory")
            .takes_value(true)
            .required(false)
            .multiple(true)
            .number_of_values(1)
            .max_values(9),
    )
    .arg(
        Arg::with_name("output-file")
            .short("o")
            .long("output-file")
            .help("output file to write the wordlist to, defaults to stdout")
            .takes_value(true)
            .required(false),
    )).subcommand(SubCommand::with_name("entropy")
        .about(r#"
Computes the estimated entropy of password or password file.
The entropy of a password is the log2(len(keyspace)) of the password.

There are two type of keyspace size estimations:
  * mask - keyspace of each char (digit=10, lowercase=26...).
  * subword - finding minimal split into subwords given a sorted by frequency smartlist and multiply the word ranks (line number in the smarlist).
  * min - minimum between the two above.

"#)
        .arg(
        Arg::with_name("smartlist")
            .short("f")
            .long("smartlist")
            .help("smartlist input file to estimate entropy with, a newline separated text file")
            .takes_value(true)
            .required(true),
        ).arg(
        Arg::with_name("password")
            .help("password to estimate entropy for")
            .takes_value(true)
            .required(false),
        ).arg(
        Arg::with_name("passwords-file")
            .short("p")
            .long("passwords-file")
            .help("newline separated password file to estimate entropy for")
            .takes_value(true)
            .required(false)
            .conflicts_with("password"),
        ).arg(
        Arg::with_name("summary")
            .short("s")
            .long("summary")
            .help("output summary of entropy for password")
            .takes_value(false)
            .required(false)
            .conflicts_with("password"),
        ).arg(
        Arg::with_name("entropy_type")
            .short("t")
            .long("entropy-type")
            .help("type of entropy to output, one of: mask, subword, min (minimum between the two, the default)")
            .takes_value(true)
            .required(false)
            .validator(one_of_validator!(vec!["min", "subword", "mask"], "entropy type must be one of: min,subword,mask"))
            .conflicts_with("password"),
        )
    ).subcommand(SubCommand::with_name("create")
        .about("Create a new smartlist from input file(s)")
        .arg(
        Arg::with_name("file")
            .short("f")
            .long("file")
            .help("input filename, can be specified multiple times for multiple files")
            .takes_value(true)
            .required(true)
            .multiple(true)
        )
        .arg(
            Arg::with_name("smartlist")
            .short("o")
            .long("smartlist")
            .help("output smartlist filename")
            .takes_value(true)
            .required(true)
        )
        .arg(
            Arg::with_name("full_smartlist")
            .long("full-smartlist")
            .help("output full smartlist filename. will write full (unfiltered) smartlist to this file")
            .takes_value(true)
            .required(false)
        )
        .arg(
        Arg::with_name("tokenizer")
            .short("t")
            .long("tokenizer")
            .help("tokenizer to use, can be specified multiple times.\none of: bpe,unigram,wordpiece")
            .takes_value(true)
            .validator(one_of_validator!(vec!["bpe", "unigram", "wordpiece"], "tokenizers must be one of: bpe,unigram,wordpiece"))
            .required(false)
            .multiple(true)
            .default_value("unigram")
        )
        .arg(
            Arg::with_name("quiet")
            .short("q")
            .long("quiet")
            .help("disables printing progress bar")
            .takes_value(false)
            .required(false)
        )
        .arg(
            Arg::with_name("vocab_max_size")
            .short("m")
            .long("vocab-max-size")
            .help("max vocabulary size")
            .takes_value(false)
            .required(false)
            .default_value("10000")
        )
        .arg(
            Arg::with_name("min_frequency")
            .long("min-frequency")
            .help("minimum frequency of a word, relevant only for BPE tokenizer")
            .takes_value(false)
            .required(false)
            .default_value("0")
        )
        .arg(
            Arg::with_name("numbers_max_size")
            .long("numbers-max-size")
            .help("filters numbers (all digits) longer than the specified size")
            .takes_value(true)
            .required(false)
            .default_value("")
        )
        .arg(
            Arg::with_name("overlapping_word_max_size")
            .long("overlapping-word-max-size")
            .help("filters words having other overlapping word(s) larger than the specified size")
            .takes_value(true)
            .required(false)
            .default_value("")
        )
        .arg(
            Arg::with_name("min_word_len")
            .long("min-word-len")
            .short("l")
            .help("filters words shorter than the specified length")
            .takes_value(true)
            .required(false)
            .default_value("1")
        )
    )
    .get_matches_from(args)
}

pub fn run(args: Option<Vec<&str>>) -> BoxResult<()> {
    // parse args
    let arg_matches = parse_args(args);

    match arg_matches.subcommand() {
        ("generate", Some(matches)) => run_wordlist_generator(matches),
        ("entropy", Some(matches)) => run_entropy_estimator(matches),
        ("create", Some(matches)) => run_create_smartlist(matches),
        (_, None) => bail!("invalid command"),
        _ => unreachable!("oopsie, subcommand is required"),
    }
}

pub fn run_wordlist_generator(args: &ArgMatches) -> BoxResult<()> {
    let mask = args.value_of("mask").unwrap();

    // TODO: result should fail on bad input not default value
    let minlen = value_t!(args.value_of("min-length"), usize).ok();
    let maxlen = value_t!(args.value_of("max-length"), usize).ok();
    let outfile = args.value_of("output-file");

    // create output file
    let out: Option<Box<dyn Write>> = match outfile {
        Some(fname) => match File::create(fname) {
            Ok(fp) => Some(Box::new(fp)),
            Err(e) => bail!("cannot open file {}: {}", fname, e),
        },
        None => None,
    };

    // TODO: check len(custom-charset) < max(mask). index error on mask
    let custom_charsets: Vec<&str> = args
        .values_of("custom-charset")
        .map(|x| x.collect())
        .unwrap_or_else(Vec::new);

    let wordlists: Vec<&str> = args
        .values_of("wordlist")
        .map(|x| x.collect())
        .unwrap_or_else(Vec::new);

    let word_generator = get_word_generator(mask, minlen, maxlen, &custom_charsets, &wordlists)?;
    if args.is_present("stats") {
        let combs = word_generator.combinations();
        println!("{}", combs);
        return Ok(());
    }

    match word_generator.gen(out) {
        Ok(_) => Ok(()),
        Err(e) => {
            match e.kind() {
                // ignore broken pipe, (e.g. happens when using head)
                ErrorKind::BrokenPipe => Ok(()),
                _ => bail!("error occurred writing to out: {}", e),
            }
        }
    }
}

pub fn run_entropy_estimator(args: &ArgMatches) -> BoxResult<()> {
    let smartlist_file = args.value_of("smartlist").unwrap();
    let est = EntropyEstimator::from_file(smartlist_file)?;
    let is_summary_only = args.is_present("summary");
    let entropy_type = args.value_of("entropy_type").unwrap_or("min");
    let mut total_entropy = 0f64;
    let mut pwd_count = 0usize;

    if let Some(pwd) = args.value_of("password") {
        let entropy_result = est.compute_password_min_entropy(pwd.as_bytes())?;
        println!("min-entropy: {}", entropy_result.min_entropy);
        println!("subword-entropy: {}", entropy_result.subword_entropy);
        println!(
            "subword-entropy-minimal-split: {:?}",
            entropy_result.subword_entropy_min_split
        );
        println!("mask-entropy: {}", entropy_result.mask_entropy);
    } else if let Some(pwd_file) = args.value_of("passwords-file") {
        let file = File::open(pwd_file)?;
        let reader = RawFileReader::new(file);
        for pwd in reader.into_iter() {
            let entropy_result = est.compute_password_min_entropy(&pwd?)?;
            let pwd_entropy = match entropy_type {
                "subword" => entropy_result.subword_entropy,
                "mask" => entropy_result.mask_entropy,
                "min" => entropy_result.min_entropy,
                _ => unreachable!("oopsie"),
            };
            if !is_summary_only {
                println!("{}", pwd_entropy);
            } else {
                total_entropy += pwd_entropy;
            }
            pwd_count += 1;
        }

        if is_summary_only {
            println!("avg entropy: {}", total_entropy / pwd_count as f64);
        }
    }
    Ok(())
}

pub fn run_create_smartlist(args: &ArgMatches) -> BoxResult<()> {
    let outfile = args.value_of("smartlist").unwrap();
    let full_outfile = args.value_of("full_smartlist");
    let infiles = args.values_of("file").map(|x| x.collect()).unwrap();
    // TODO: from config or optional or if let
    // TODO: value_t! does not raise in case of invalid value
    let vocab_max_size = value_t!(args.value_of("vocab_max_size"), u32).unwrap_or(10_000);
    let min_frequency = value_t!(args.value_of("min_frequency"), u32).unwrap_or(10_000);
    let print_progress = !args.is_present("quiet");
    let numbers_max_size = value_t!(args.value_of("numbers_max_size"), u32).ok();
    let overlapping_word_max_size = value_t!(args.value_of("overlapping_word_max_size"), u32).ok();
    let min_word_len = value_t!(args.value_of("min_word_len"), u32).unwrap_or(1);

    let tokenizers = args
        .values_of("tokenizer")
        .map(|x| x.collect())
        .unwrap_or_else(|| vec!["unigram"])
        .into_iter()
        .map(|x| match x {
            "bpe" => SmartlistTokenizer::BPE,
            "unigram" => SmartlistTokenizer::Unigram,
            "wordpiece" => SmartlistTokenizer::WordPiece,
            _ => unreachable!("invalid tokenizer {}", x),
        });

    let mut writer = BufWriter::new(File::create(outfile)?);
    let full_writer = match full_outfile {
        Some(full_outfile_path) => Some(BufWriter::new(File::create(full_outfile_path)?)),
        None => None,
    };
    let (vocab, full_vocab) = SmartlistBuilder::new()
        .infiles(infiles)
        .min_frequency(min_frequency)
        .vocab_max_size(vocab_max_size)
        .tokenizers(tokenizers.into_iter())
        .print_progress(print_progress)
        .numbers_max_size(numbers_max_size)
        .overlapping_word_max_size(overlapping_word_max_size)
        .min_word_len(min_word_len)
        .return_full_vocab(full_outfile.is_some())
        .build()?;

    // write to file
    for word in vocab.iter() {
        writer.write_all(word.as_bytes())?;
        writer.write_all(b"\n")?;
    }

    // write to file full vocab
    if let (Some(full_vocab), Some(mut full_writer)) = (full_vocab, full_writer) {
        for word in full_vocab.iter() {
            full_writer.write_all(word.as_bytes())?;
            full_writer.write_all(b"\n")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{runner, test_util};

    #[test]
    fn test_run_generate_smoke() {
        for args in vec![vec!["cracken", "generate", "?d"], vec!["cracken", "?d"]] {
            assert!(runner::run(Some(args)).is_ok());
        }
    }

    #[test]
    fn test_run_entropy_smoke() {
        let vocab_fname = test_util::wordlist_fname("vocab.txt");
        let args = Some(vec![
            "cracken",
            "entropy",
            "--smartlist",
            vocab_fname.to_str().unwrap(),
            "helloworld123!",
        ]);
        assert!(runner::run(args).is_ok());
    }

    #[test]
    fn test_run_dev_null() {
        let args = Some(vec!["cracken", "-o", "/dev/null", "?d"]);
        assert!(runner::run(args).is_ok());
    }

    #[test]
    fn test_run_custom_charset() {
        let args = Some(vec!["cracken", "-c=abcdef0123456789", "?1"]);
        assert!(runner::run(args).is_ok());
    }

    #[test]
    fn test_run_stats() {
        let args = Some(vec!["cracken", "-s", "?d?s?u?l?a?b"]);
        assert!(runner::run(args).is_ok());
    }

    #[test]
    fn test_run_perm_denied() {
        let args = Some(vec!["cracken", "-o", "/tmp/this/dir/not/exisT", "?d"]);
        assert!(runner::run(args).is_err());
    }

    #[test]
    fn test_run_bad_args() {
        let args = Some(vec!["cracken", "-m", "2", "?d"]);
        assert!(runner::run(args).is_err());
    }

    #[test]
    fn test_run_bad_args2() {
        let args = Some(vec!["cracken", "?x"]);
        assert!(runner::run(args).is_err());
    }

    #[test]
    fn test_run_bad_args3() {
        let args = Some(vec!["cracken", "-x", "5", "?d"]);
        assert!(runner::run(args).is_err());
    }
}
