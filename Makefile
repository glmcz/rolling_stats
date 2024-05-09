.PHONY: all build build-std clean

SRC_DIR := src
TARGET_DIR := target
CARGO_CMD := cargo build

CARGO_FLAGS :=
CARGO_STD_FLAGS := --features std
CARGO_NO_STD_FLAGS := --no-default-features --features std

all: build

build:
	$(CARGO_CMD) $(CARGO_FLAGS) $(CARGO_NO_STD_FLAGS)

build-std:
	$(CARGO_CMD) $(CARGO_FLAGS) $(CARGO_STD_FLAGS)

clean:
	cargo clean

