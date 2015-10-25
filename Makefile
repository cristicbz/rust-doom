all:
	@cargo build

test:
	@cargo test -p common
	@cargo test -p gfx
	@cargo test -p math
	@cargo test -p game
	@cargo test -p wad
	@cargo test

update:
	@cargo update -p common
	@cargo update -p gfx
	@cargo update -p math
	@cargo update -p game
	@cargo update -p wad
	@cargo update

clean:
	@cargo clean -p common
	@cargo clean -p gfx
	@cargo clean -p math
	@cargo clean -p game
	@cargo clean -p wad
	@cargo clean

tests: test

.PHONY: all test tests update clean
