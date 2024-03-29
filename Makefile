SHELL := $(shell which bash)
STARTDATE := $(shell date -u +'%Y-%m-%dT%H-%M-%S')
PROJECT_VERSION := $(shell grep -oE 'version = ".*"' Cargo.toml -m 1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')
RELEASE_TARBALL = ./cracken-v${PROJECT_VERSION}.tar.gz
GPG_SIGN_KEY = 647290A426CF53EF

build-crate:
	cargo package

release:
	cargo build --release
	rm -rf ./build/*
	mkdir -p ./build/
	cp ./target/release/cracken ./build/

	cd build && \
		tar -pcvzf "${RELEASE_TARBALL}" "./cracken" && \
		gpg --detach-sign --default-key "${GPG_SIGN_KEY}" --armor "${RELEASE_TARBALL}" && \
		gpg --verify "${RELEASE_TARBALL}.asc"

validate:
	cargo fmt -- --check
	cargo clippy

test: validate
	cargo test -- --nocapture

test-release: release
	rm -rf ./tmp/*
	mkdir -p ./tmp

	@echo "testing mask with minlen and maxlen"
	cd ./tmp && \
		cp "../build/${RELEASE_TARBALL}" . && \
		tar xzvf "../build/${RELEASE_TARBALL}" && \
		./cracken --help && \
		time ./cracken -m 1 -x 4 -o './cracken-${STARTDATE}.txt' '?u?l?u?l' && \
		cmp ../test-resources/upper-lower-1-4.txt './cracken-${STARTDATE}.txt'

	@echo "testing mask with wordlists and custom charsets"
	cd ./tmp && \
		time ./cracken -c '#!@' \
			-w ../test-resources/wordlist1.txt \
			-w ../test-resources/wordlist2.txt \
			-o './cracken-${STARTDATE}-wl.txt' \
			'?w1?d?w2?l?w1?1' && \
		cmp ../test-resources/wordlists-mix.txt './cracken-${STARTDATE}-wl.txt'
	rm -rf ./tmp/*

print-release-version:
	./build/cracken --help | grep cracken-v

bench:
	cargo bench

coverage:
	cargo tarpaulin -o Html --avoid-cfg-tarpaulin

ci: test test-release coverage build-crate print-release-version

fmt:
	cargo fmt
