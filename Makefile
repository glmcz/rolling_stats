.PHONY: all build build-std clean

SRC_DIR := src
TARGET_DIR := target

CARGO_FLAGS :=
CARGO_STD_FLAGS := --features std
CARGO_NO_STD_FLAGS := --no-default-features --features no-std

all: build

build:
	$(CARGO_BUILD) $(CARGO_FLAGS)

build-std:
	$(CARGO_BUILD) $(CARGO_FLAGS) $(CARGO_STD_FLAGS)

build-no-std:
	$(CARGO_BUILD) $(CARGO_FLAGS) -- $(CARGO_NO_STD_FLAGS)
	
clean:
	cargo clean

