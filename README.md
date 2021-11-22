# Cracken
[![cracken crate](https://img.shields.io/crates/v/cracken)](https://crates.io/crates/cracken)
[![cracken version](https://img.shields.io/crates/l/cracken)](https://crates.io/crates/cracken)
[![cracken documentation](https://img.shields.io/docsrs/cracken)](https://docs.rs/cracken)
[![cracken total downloads](https://img.shields.io/crates/d/cracken)](https://crates.io/crates/cracken)

Cracken is a fast password wordlist generator, Smartlist creation and password hybrid-mask analysis tool
written in pure safe Rust (more on [talk/][talk]). Inspired by great tools like [maskprocessor][mp], [hashcat][hashcat], [Crunch][crunch] and 🤗 HuggingFace's [tokenizers][tokenizers].

## What? Why? Woot??

At [DeepSec2021][talk-abstract] we presented a new method for analysing passwords as Hybrid-Masks exploiting common substrings in passwords by utilizing NLP tokenizers (more info on [talk/][talk]).

Our method splits a password into its subwords instead of just a characters mask. `HelloWorld123!` splitted into `['Hello', 'World', '123!']` as these three subwords are very common in other passwords.

### Hybrid Masks & Smartlists

* 📄 **Smartlists** - Compact & representative subword lists created from passwords by utilizing NLP tokenizers
* 🎭 **Hybrid-Mask** - A representation of a password as a combination of wordlists & characters (e.g. `?w1?w2?l?d`)

#### Analyzing RockYou Passwords with Smartlists & Hybrid-Masks:

![Top25 Hybrid Masks from RockYou](./talk/top25_rockyou_hybrid-masks_count.png)

full table [here](./talk/rockyou_hybrid-masks_counts.csv)


### Cracken 🐙 is used for:
* ✅ Generating **`Hybrid-Masks`** very VERY FAST 🦸⚡💨 (see [performance](#performance) section)
* ✅ Building **`Smartlists`** - compact & representative list of subwords from given passwords files (using 🤗 HuggingFace's [tokenizers][tokenizers])
* ✅ Analyzing passwords for their `Hybrid-Masks` - building statistics for better password candidates (again very fast)


### Possible workflows with Cracken:

#### Simple:
1. Generate wordlist candidates from a hybrid mask - e.g. `cracken -w rockyou.txt -w 100-most-common.txt '?w1?w2?d?d?d?d?s'`
2. You can pipe the passwords Cracken generates into `hashcat`, `john` or your favorite password cracker


#### Advanced:
1. Create a Smartlist from existing passwords - `cracken create`
2. Analyze a passwords list of plaintext passwords - `cracken entropy`
3. use most frequent `Hybrid-Masks` to generate password candidates fast - `cracken generate -i hybrid-masks.txt`

For more details see [Usage](#usage-info) section


## Getting Started

**download (linux only currently):** [latest release 🔗][releases]

*for more installation options see `installation` section*

**run Cracken:**

generate all words of length 8 starting with uppercase followed by 6 lowercase chars and then a digit:

```bash
$ cracken -o pwdz.lst '?u?l?l?l?l?l?l?d'
```

generate words from two wordlists with year suffix (1000-2999) `<firstname><lastname><year>`

```bash
$ cracken --wordlist firstnames.txt --wordlist lastnames.lst --charset '12' '?w1?w2?1?d?d?d'
```

create a Smartlist of size 50k from subwords extracted from rockyou.txt

```bash
$ cracken create -f rockyou.txt -m 50000 --smartlist smart.lst
```

estimate the entropy of hybrid mask of the password HelloWorld123! using a smartlist

```bash
$ cracken entropy -f smart.lst 'HelloWorld123!'

hybrid-min-split: ["hello", "world1", "2", "3", "!"]
hybrid-mask: ?w1?w1?d?d?s
hybrid-min-entropy: 42.73
--
charset-mask: ?l?l?l?l?l?l?l?l?l?l?d?d?d?s
charset-mask-entropy: 61.97
```

## Performance

As of writing this, Cracken is probably the world's fastest wordlist generator:

![bechmarks results](./benchmarks/bench-results.svg)

Cracken has around 25% increased performance over hashcat's fast [maskprocessor][mp] thats written in C.

Cracken can generate around 2 GB/s per core.

more details on [benchmarks/ 🔗](./benchmarks/README.md)

Why speed is important? A typical GPU can test billions passwords per second depending on the password hash function.
When the wordlist generator produces fewer words per second than the cracking tool can handle - the cracking speed will degrade.

### Hybrid-Masks Analysis Performance

Cracken uses `A*` algorithm to analyze passwords very fast. it can find the minimal Hybrid-Mask of passwords file at rate of **~100k Passwords/sec** (`cracken entropy -f words1.txt -f words2.txt ... -p pwds.txt`)


## Installation

install Cracken or compile from source


### Download Binary (Linux Only Currently)

download latest release from [releases 🔗][releases]

### Build From Source (All Platforms)

Cracken is written in Rust and needs rustc to get compiled. Cracken should support all Platforms that Rust support.

installation instructions for [cargo 🔗][rustc-installation]

there are two options building from source - installing with cargo from crates.io (preferred) or compiling manually from source.


#### 1. install from crates.io (preferred)

**install with cargo:**

```bash
$ cargo install cracken
```

#### 2. build from source


**clone Cracken:**

```bash
$ git clone https://github.com/shmuelamar/cracken
```

**build Cracken:**

```bash
$ cd cracken
$ cargo build --release
```

**run it:**

```bash
$ ./target/release/cracken --help
```


## Usage Info


```
$ cracken --help
Cracken v1.0.0 - a fast password wordlist generator 

USAGE:
    cracken [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    generate    (default) - Generates newline separated words according to given mask and wordlist files
    create      Create a new smartlist from input file(s)
    entropy     
                Computes the estimated entropy of password or password file.
                The entropy of a password is the log2(len(keyspace)) of the password.
                
                There are two types of keyspace size estimations:
                  * mask - keyspace of each char (digit=10, lowercase=26...).
                  * hybrid - finding minimal split into subwords and charsets.


For specific subcommand help run: cracken <subcommand> --help


Example Usage:

  ## Generate Subcommand Examples:

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
  cracken -c 0123456789abcdef '?1?1?1?1'

  # 4 custom charsets - the order determines the id of the charset
  cracken -c 01 -c ab -c de -c ef '?1?2?3?4'

  # 4 lowercase chars with years 2000-2019 suffix
  cracken -c 01 '?l?l?l?l20?1?d'

  # starts with firstname from wordlist followed by 4 digits
  cracken -w firstnames.txt '?w1?d?d?d?d'

  # starts with firstname from wordlist with lastname from wordlist ending with symbol
  cracken -w firstnames.txt -w lastnames.txt -c '!@#$' '?w1?w2?1'

  # repeating wordlists multiple times and combining charsets
  cracken -w verbs.txt -w nouns.txt '?w1?w2?w1?w2?w2?d?d?d'


  ## Create Smartlists Subcommand Examples:

  # create smartlist from single file into smart.txt
  cracken create -f rockyou.txt --smartlist smart.txt

  # create smartlist from multiple files with multiple tokenization algorithms
  cracken create -t bpe -t unigram -t wordpiece -f rockyou.txt -f passwords.txt -f wikipedia.txt --smartlist smart.txt

  # create smartlist with minimum subword length of 3 and max numbers-only subwords of size 6
  cracken create -f rockyou.txt --min-word-len 3 --numbers-max-size 6 --smartlist smart.txt


  ## Entropy Subcommand Examples:

  # estimating entropy of a password
  cracken entropy --smartlist vocab.txt 'helloworld123!'

  # estimating entropy of a passwords file with a charset mask entropy (default is hybrid)
  cracken entropy --smartlist vocab.txt -t charset -p passwords.txt

  # estimating the entropy of a passwords file
  cracken entropy --smartlist vocab.txt -p passwords.txt

cracken-v1.0.0 linux-x86_64 compiler: rustc 1.56.1 (59eed8a2a 2021-11-01)
more info at: https://github.com/shmuelamar/cracken
```

### Generate Subcommand Usage Info

```
$ cracken generate --help
cracken-generate 
(default) - Generates newline separated words according to given mask and wordlist files

USAGE:
    cracken generate [FLAGS] [OPTIONS] <mask> --masks-file <masks-file>

FLAGS:
    -h, --help       
            Prints help information

    -s, --stats      
            prints the number of words this command will generate and exits

    -V, --version    
            Prints version information


OPTIONS:
    -c, --custom-charset <custom-charset>...    
            custom charset (string of chars). up to 9 custom charsets - ?1 to ?9. use ?1 on the mask for the first charset

    -i, --masks-file <masks-file>               
            a file containing masks to generate

    -x, --maxlen <max-length>                   
            maximum length of the mask to start from

    -m, --minlen <min-length>                   
            minimum length of the mask to start from

    -o, --output-file <output-file>             
            output file to write the wordlist to, defaults to stdout

    -w, --wordlist <wordlist>...                
            filename containing newline (0xA) separated words. note: currently all wordlists loaded to memory


ARGS:
    <mask>    
            the wordlist mask to generate.
            available masks are:
                builtin charsets:
                ?d - digits: "0123456789"
                ?l - lowercase: "abcdefghijklmnopqrstuvwxyz"
                ?u - uppercase: "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
                ?s - symbols: " !\"\#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
                ?a - all characters: ?d + ?l + ?u + ?s
                ?b - all binary values: (0-255)
            
                custom charsets ?1 to ?9:
                ?1 - first custom charset specified by --charset 'mychars'
            
                wordlists ?w1 to ?w9:
                ?w1 - first wordlist specified by --wordlist 'my-wordlist.txt'
```

### Create Smartlist Subcommand Usage Info

```
$ cracken create --help  
cracken-create 
Create a new smartlist from input file(s)

USAGE:
    cracken create [FLAGS] [OPTIONS] --file <file>... --smartlist <smartlist>

FLAGS:
    -h, --help       Prints help information
    -q, --quiet      disables printing progress bar
    -V, --version    Prints version information

OPTIONS:
    -f, --file <file>...                         input filename, can be specified multiple times for multiple files
        --min-frequency <min_frequency>          minimum frequency of a word, relevant only for BPE tokenizer
    -l, --min-word-len <min_word_len>            filters words shorter than the specified length
        --numbers-max-size <numbers_max_size>    filters numbers (all digits) longer than the specified size
    -o, --smartlist <smartlist>                  output smartlist filename
    -t, --tokenizer <tokenizer>...               tokenizer to use, can be specified multiple times.
                                                 one of: bpe,unigram,wordpiece [default: bpe]  [possible values: bpe, unigram, wordpiece]
    -m, --vocab-max-size <vocab_max_size>        max vocabulary size
```

### Entropy Subcommand Usage Info

```
$ cracken entropy --help
cracken-entropy 

Computes the estimated entropy of password or password file.
The entropy of a password is the log2(len(keyspace)) of the password.

There are two types of keyspace size estimations:
  * mask - keyspace of each char (digit=10, lowercase=26...).
  * hybrid - finding minimal split into subwords and charsets.


USAGE:
    cracken entropy [FLAGS] [OPTIONS] <password> --smartlist <smartlist>...

FLAGS:
    -h, --help       Prints help information
    -s, --summary    output summary of entropy for password
    -V, --version    Prints version information

OPTIONS:
    -t, --mask-type <mask_type>              type of mask to output, one of: charsets(charsets only), hybrid(charsets+wordlists) [possible values: hybrid, charset]
    -p, --passwords-file <passwords-file>    newline separated password file to estimate entropy for
    -f, --smartlist <smartlist>...           smartlist input file to estimate entropy with, a newline separated text file

ARGS:
    <password>    password to
```

## License

Cracken is licensed under MIT. **THIS PROJECT MUST BE USED FOR LEGAL PURPOSES ONLY ⚖️**


## Contributing

Cracken is under active development, if you wish to help below is this the partial roadmap for this project.
Feel free to submit PRs and open issues.


[mp]: https://hashcat.net/wiki/doku.php?id=maskprocessor
[hashcat]: https://hashcat.net
[crunch]: https://github.com/crunchsec/crunch
[releases]: https://github.com/shmuelamar/cracken/releases
[talk]: ./talk
[rustc-installation]: https://www.rust-lang.org/tools/install
[talk-abstract]: https://deepsec.net/speaker.html#PSLOT517
[tokenizers]: https://github.com/huggingface/tokenizers
