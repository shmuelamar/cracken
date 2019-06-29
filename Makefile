SHELL := $(shell which bash)
STARTDATE := $(shell date -u +'%Y-%m-%dT%H-%M-%S')
PROJECT_VERSION := 0.1.0
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
	cd ./tmp && \
		cp "../build/${RELEASE_TARBALL}" . && \
		tar xzvf "../build/${RELEASE_TARBALL}" && \
		./cracken --help && \
		time ./cracken -m 1 -x 4 -o './cracken-${STARTDATE}.txt' '?u?l?u?l' && \
		cmp ../test-resources/upper-lower-1-4.txt './cracken-${STARTDATE}.txt'
	rm -rf ./tmp/*

bench:
	cargo bench

coverage:
	cargo tarpaulin -o Html

ci: test test-release build-crate coverage

fmt:
	cargo fmt
