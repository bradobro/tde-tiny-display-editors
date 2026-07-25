BINARY := zde
MODULE := zde

# Release build flags: -s -w strip the symbol table and DWARF debug info
# (they're never used once the editor is installed, only during
# development), and -trimpath drops build-machine file paths from the
# binary. This is the standalone-.com-file-sized-binary goal from the
# project README (see CLAUDE.md: "Goal: a minimal editor").
LDFLAGS_RELEASE := -s -w

.PHONY: run build test release clean

run:
	go run $(MODULE)

build:
	go build -o $(BINARY) $(MODULE)

test:
	go test ./...

release:
	go build -trimpath -ldflags="$(LDFLAGS_RELEASE)" -o $(BINARY) $(MODULE)

clean:
	rm -f $(BINARY)
