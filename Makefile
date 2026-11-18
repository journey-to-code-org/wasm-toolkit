# Builds the toolkit and a game to one .wasm with clang and nothing else, and the same code to a
# native program for the tests.
CC := clang
GAME ?= shootout
TOOLKIT := toolkit/wtk.c toolkit/font.c toolkit/sintab.c
WARN := -std=c99 -Wall -Wextra -Werror -Wno-unused-parameter
EXPORTS := wtk_init wtk_input wtk_step wtk_screen wtk_palette

all: web/$(GAME).wasm

web/$(GAME).wasm: $(TOOLKIT) toolkit/wtk.h games/$(GAME)/*.c $(wildcard games/$(GAME)/*.h)
	$(CC) --target=wasm32 -O2 -nostdlib -ffreestanding $(WARN) \
	  -Wl,--no-entry -Wl,--allow-undefined -Wl,--strip-all $(foreach e,$(EXPORTS),-Wl,--export=$(e)) \
	  -o $@ $(TOOLKIT) games/$(GAME)/*.c

test: all build/test_native
	./build/test_native
	node test/smoke.mjs web/$(GAME).wasm

build/test_native: $(TOOLKIT) toolkit/wtk.h test/test_native.c
	mkdir -p build
	$(CC) -O1 -g -fsanitize=undefined -fsanitize-trap=undefined $(WARN) -o $@ $(TOOLKIT) test/test_native.c

# The Rust engine and the pack it plays. The wasm target comes from `rustup target add
# wasm32-unknown-unknown`; where that is not to be had, ENGINE_FLAGS="-Zbuild-std=std,panic_abort"
# with RUSTC_BOOTSTRAP=1 builds the standard library from its source.
PACK ?= default
ENGINE_FLAGS ?=
ENGINE_WASM := engine/target/wasm32-unknown-unknown/release/engine.wasm

engine: web/engine.wasm web/$(PACK).pack

web/engine.wasm: engine/Cargo.toml engine/src/*.rs
	cd engine && cargo build --release --target wasm32-unknown-unknown $(ENGINE_FLAGS)
	cp $(ENGINE_WASM) $@

web/$(PACK).pack: content/$(PACK)/* tools/pack.py tools/paint.py
	python3 tools/pack.py content/$(PACK) $@

test-engine: engine
	cd engine && cargo test
	node test/engine.mjs web/engine.wasm web/$(PACK).pack

serve: all
	cd web && python3 -m http.server 8080

clean:
	rm -rf build web/*.wasm web/*.pack engine/target

art:
	python3 tools/paint.py

.PHONY: all test serve clean art engine test-engine
