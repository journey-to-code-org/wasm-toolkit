# Builds the toolkit and a game to one .wasm with clang and nothing else, and the same code to a
# native program for the tests.
CC := clang
GAME ?= shootout
TOOLKIT := toolkit/wtk.c toolkit/font.c toolkit/sintab.c
WARN := -std=c99 -Wall -Wextra -Werror -Wno-unused-parameter
EXPORTS := wtk_init wtk_input wtk_step wtk_screen wtk_palette

all: web/$(GAME).wasm

web/$(GAME).wasm: $(TOOLKIT) toolkit/wtk.h games/$(GAME)/*.c
	$(CC) --target=wasm32 -O2 -nostdlib -ffreestanding $(WARN) \
	  -Wl,--no-entry -Wl,--allow-undefined -Wl,--strip-all $(foreach e,$(EXPORTS),-Wl,--export=$(e)) \
	  -o $@ $(TOOLKIT) games/$(GAME)/*.c

test: all build/test_native
	./build/test_native
	node test/smoke.mjs web/$(GAME).wasm

build/test_native: $(TOOLKIT) toolkit/wtk.h test/test_native.c
	mkdir -p build
	$(CC) -O1 -g -fsanitize=undefined -fsanitize-trap=undefined $(WARN) -o $@ $(TOOLKIT) test/test_native.c

serve: all
	cd web && python3 -m http.server 8080

clean:
	rm -rf build web/*.wasm

.PHONY: all test serve clean
