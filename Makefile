all:
	@cargo build

test:
	@cargo test -p base
	@cargo test -p gl
	@cargo test -p gfx
	@cargo test -p math
	@cargo test -p game
	@cargo test

tests: test

.PHONY: all test tests
