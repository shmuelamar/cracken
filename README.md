# Cracken
[![cracken crate](https://img.shields.io/crates/v/cracken.svg)](https://crates.io/crates/cracken)
[![cracken version](https://img.shields.io/crates/l/cracken.svg)](https://crates.io/crates/cracken)

Cracken is a fast password wordlist generator written in pure safe Rust. Inspired by great tools like [maskprocessor][mp] and [Crunch][crunch].


## Getting Started

**download (linux only):** [latest release 🔗][releases]

**run Cracken:** this will generate all words of length 8 starting with uppercase followed by 6 lowercase chars and then a digit:

```bash
$ ./cracken -o pwdz.lst '?u?l?l?l?l?l?l?d'
```

for more installation options see `installation` section


## Performance

As of writing this, Cracken is probably the world's fastest wordlist generator:

![bechmarks results](./benchmarks/bench-results.png)

Cracken has around 20% increased performance over hashcat's fast (and awesome) [maskprocessor][mp] thats written in C.

Cracken can generate around 1.5 GiB/s per core.

more details on [benchmarks/ 🔗](./benchmarks/README.md)


## Installation

install Cracken or compile from source


### Download Binary (Linux Only)

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


``` cracken --help
Cracken v0.1.0 - a fast password wordlist generator

USAGE:
    cracken [OPTIONS] <mask>

FLAGS:
    -h, --help       
            Prints help information

    -V, --version    
            Prints version information


OPTIONS:
    -x, --maxlen <max-length>          
            maximum length of the mask to start from

    -m, --minlen <min-length>          
            minimum length of the mask to start from

    -o, --output-file <output-file>    
            output file to write the wordlist to, defaults to stdout


ARGS:
    <mask>    
            the wordlist mask to generate.
            available masks are:
                ?d - digits: "0123456789"
                ?l - lowercase: "abcdefghijklmnopqrstuvwxyz"
                ?u - uppercase: "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
                ?s - symbols: " !\"\#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
                ?a - all characters: "?d?l?u?s"
                ?b - all binary values: (0-255)

Example Usage:
  # all digits from 00000000 to 99999999
  cracken ?d?d?d?d?d?d?d?d

  # all digits from 0 to 99999999
  cracken -m 1 ?d?d?d?d?d?d?d?d

  # all passwords of length 8 starting with upper then 6 lowers then digit
  cracken ?u?l?l?l?l?l?l?d

  # same as above, write output to pwds.txt instead of stdout
  cracken -o pwds.txt ?u?l?l?l?l?l?l?d

cracken-v0.1.0 linux-x86_64 compiler: rustc 1.35.0 (3c235d560 2019-05-20)
more info at: https://github.com/shmuelamar/cracken
```


## License

Cracken is licensed under MIT. **THIS PROJECT SHOULD BE USED FOR LEGAL PURPOSES ONLY**


## Contributing

Cracken is under active development, if you wish to help below is this the partial roadmap for this project.
Feel free to submit PRs and open issues.

### Features List

* [x] min/max word length
* [ ] custom charset
* [ ] wordlist(s) as input
* [ ] input file of list of masks
* [ ] fixed chars
* [ ] number of total passwords to generate
* [ ] stderr status tracker thread
* [ ] compression
* [ ] multithreading


[mp]: https://hashcat.net/wiki/doku.php?id=maskprocessor
[crunch]: https://github.com/crunchsec/crunch
[releases]: https://github.com/shmuelamar/cracken/releases
[rustc-installation]: https://www.rust-lang.org/tools/install
