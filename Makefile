.PHONY: all build build-std clean

SRC_DIR := src
TARGET_DIR := target
CARGO_BUILD := cargo build
CARGO_TEST := cargo test

CARGO_FLAGS :=
CARGO_STD_FLAGS := --features std
CARGO_NO_STD_FLAGS := --no-default-features

all: build

build:
	$(CARGO_BUILD) $(CARGO_FLAGS) $(CARGO_NO_STD_FLAGS)

build-std:
	$(CARGO_BUILD) $(CARGO_FLAGS) $(CARGO_STD_FLAGS)


test:
	$(CARGO_TEST) $(CARGO_FLAGS) $(CARGO_NO_STD_FLAGS)

test-std:
	$(CARGO_TEST) $(CARGO_FLAGS) $(CARGO_STD_FLAGS)


clean:
	cargo clean

