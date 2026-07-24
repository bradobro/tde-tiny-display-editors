//! Build script for the Zig port of ZDE.
//!
//! Zero dependencies (see `build.zig.zon`): the terminal backend is raw ANSI +
//! termios via `std.posix`, so there is nothing to fetch. Two steps:
//!   `zig build run  -- FILE`  — build and run the editor
//!   `zig build test`          — run every module's inline `test` blocks
//!
//! `src/main.zig` is the single compilation root; it `@import`s every module,
//! and its test block references each one so `zig build test` picks up the
//! tests in all of them (Zig only compiles tests in files reachable from the
//! test root).

const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const root = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });

    const exe = b.addExecutable(.{
        .name = "zde",
        .root_module = root,
    });
    b.installArtifact(exe);

    // `zig build run -- FILE`
    const run_cmd = b.addRunArtifact(exe);
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| run_cmd.addArgs(args);
    const run_step = b.step("run", "Run the editor");
    run_step.dependOn(&run_cmd.step);

    // `zig build test`
    const tests = b.addTest(.{ .root_module = root });
    const run_tests = b.addRunArtifact(tests);
    const test_step = b.step("test", "Run all unit tests");
    test_step.dependOn(&run_tests.step);
}
