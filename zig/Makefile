.PHONY: run build test release clean

build:
	zig build

run:
	zig build run

test:
	zig build test

release:
	zig build -Doptimize=ReleaseSmall

clean:
	rm -rf zig-out .zig-cache
