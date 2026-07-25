//! Word wrap, reformat, margins, tabs, centering — the on-screen `^O` family and
//! paragraph reflow (`^B`). Corresponds to the ASM formatter (`zde17.asm:5214`).
//!
//! Pure column/word math over `[]const u21` (the buffer's own element type, so
//! no string conversion is needed at the call sites in `editor.zig`), unit-
//! testable without a terminal. Ported from `rust/src/format.rs`.
//!
//! ## No soft-space bookkeeping
//!
//! The original distinguishes "soft" spaces the reformatter inserted (freely
//! removable/regenerable) from "hard" spaces the user typed, via a high bit on
//! the byte (ASM `Cmprs`, `zde17.asm:2129`). `doc/adr/0002` drops that
//! distinction: reflow always recomputes spacing from the words on the line
//! rather than decompressing stored state. Hard CRs are still preserved —
//! reflow only touches spacing within a paragraph, not paragraph breaks.
//!
//! ## No hyphenation
//!
//! Per `doc/adr/0004` (hyphenation dropped), word wrap simply leaves an
//! overlong word alone rather than splitting it — see `findWrapPoint`.

const std = @import("std");
const Allocator = std.mem.Allocator;

/// Result of a right-margin check: whether the current word should wrap.
pub const WrapDecision = enum { fits, wrap_word };

/// Column of `line[0..char_index]`, expanding hard tabs to `tab_width`-wide
/// stops. Mirrors `buffer.columnOf`'s math exactly (same ASM column update,
/// `zde17.asm:5378`) but works on an extracted slice so the rest of this
/// module can stay buffer-agnostic and unit-testable on plain slices.
pub fn displayColumn(line: []const u21, char_index: usize, tab_width: usize) usize {
    var col: usize = 0;
    for (line[0..@min(char_index, line.len)]) |c| {
        col = if (c == '\t') (col / tab_width + 1) * tab_width else col + 1;
    }
    return col;
}

/// Next variable tab stop strictly past `col`, from a 0-terminated stop list
/// (ASM `VTList`, `zde17.asm:162`). Returns `null` if none is beyond `col`, so
/// the caller can fall back to a hard-tab stop.
pub fn nextVariableTabStop(stops: []const u8, col: usize) ?usize {
    for (stops) |s| {
        if (s == 0) break; // 0 terminates the list
        if (s > col) return s;
    }
    return null;
}

fn stopCount(tabs: []const u8) usize {
    var len: usize = 0;
    while (len < tabs.len and tabs[len] != 0) : (len += 1) {}
    return len;
}

/// Add `col` to the sorted, 0-terminated variable-tab list (simplified
/// `VTSet`, `zde17.asm:3926`, single-column form only — the ASM's `@n`
/// evenly-spaced and `#` explicit-group shorthand aren't ported; enter one
/// column at a time instead). Returns `false` (a no-op) for `col == 0`, a
/// column already present, or a list with no free slot left.
pub fn insertTabStop(tabs: []u8, col: u8) bool {
    if (col == 0) return false;
    const len = stopCount(tabs);
    if (len == tabs.len) return false;
    for (tabs[0..len]) |s| if (s == col) return false;
    var pos: usize = len;
    for (tabs[0..len], 0..) |s, i| {
        if (s > col) {
            pos = i;
            break;
        }
    }
    var i = len;
    while (i > pos) : (i -= 1) tabs[i] = tabs[i - 1];
    tabs[pos] = col;
    return true;
}

/// Remove `col` from the variable-tab list, closing the gap so the list
/// stays 0-terminated (ASM `VTClr`, `zde17.asm:4013`). Returns `false` if
/// `col` wasn't a configured stop.
pub fn removeTabStop(tabs: []u8, col: u8) bool {
    const len = stopCount(tabs);
    var pos: ?usize = null;
    for (tabs[0..len], 0..) |s, i| {
        if (s == col) {
            pos = i;
            break;
        }
    }
    const p = pos orelse return false;
    var i = p;
    while (i + 1 < len) : (i += 1) tabs[i] = tabs[i + 1];
    tabs[len - 1] = 0;
    return true;
}

/// Whether the column just reached (1-based, matching `Editor.cur_col`) has
/// pushed the line past the right margin (ASM `ChkRM`, `zde17.asm:5273`).
/// `right_margin <= 1` means "off" (the ASM's `SetRM`/`WdWrap` convention).
pub fn checkRightMargin(col: usize, right_margin: u8) WrapDecision {
    if (right_margin <= 1 or col <= right_margin) return .fits;
    return .wrap_word;
}

/// Where to break the current line for word wrap: the index (within `line`,
/// the line's text up to and including the just-typed character) of the
/// space that separates the last word from the rest — that word is the one
/// that moves to the next line. `null` when there's no space to break at (a
/// single word already longer than the margin); per the module doc, that
/// word is simply left to overflow rather than hyphenated.
pub fn findWrapPoint(line: []const u21) ?usize {
    var i = line.len;
    while (i > 0) {
        i -= 1;
        if (line[i] == ' ') return i;
    }
    return null;
}

fn isWhitespace(c: u21) bool {
    return c == ' ' or c == '\t' or c == '\n' or c == '\r';
}

/// Split `text` on runs of whitespace (mirrors Rust's `str::split_whitespace`).
/// The returned words borrow `text`; caller frees the outer slice only.
fn splitWhitespace(alloc: Allocator, text: []const u21) ![][]const u21 {
    var words: std.ArrayList([]const u21) = .empty;
    errdefer words.deinit(alloc);
    var i: usize = 0;
    while (i < text.len) {
        while (i < text.len and isWhitespace(text[i])) : (i += 1) {}
        const start = i;
        while (i < text.len and !isWhitespace(text[i])) : (i += 1) {}
        if (i > start) try words.append(alloc, text[start..i]);
    }
    return words.toOwnedSlice(alloc);
}

/// Reflow a paragraph's text (no blank lines inside it, hard CRs already
/// stripped by the caller) into lines that fit between `left_margin` and
/// `right_margin` (both 1-based columns), breaking only at word boundaries —
/// ASM `Reform` (`zde17.asm:5477`), minus the soft-space bookkeeping (see
/// module doc). Words wider than the field are placed on their own
/// (overflowing) line rather than hyphenated. Caller frees the result.
pub fn reflowParagraph(alloc: Allocator, paragraph: []const u21, left_margin: usize, right_margin: usize) ![]u21 {
    const words = try splitWhitespace(alloc, paragraph);
    defer alloc.free(words);
    var out: std.ArrayList(u21) = .empty;
    errdefer out.deinit(alloc);
    if (words.len == 0) return out.toOwnedSlice(alloc);

    const indent = left_margin -| 1;
    const field_width = right_margin -| left_margin + 1;

    var current: std.ArrayList(u21) = .empty;
    defer current.deinit(alloc);
    var wrote_line = false;

    for (words) |word| {
        const joined_len = current.items.len + (if (current.items.len == 0) @as(usize, 0) else 1) + word.len;
        if (current.items.len != 0 and joined_len > field_width) {
            if (wrote_line) try out.append(alloc, '\n');
            wrote_line = true;
            try out.appendNTimes(alloc, ' ', indent);
            try out.appendSlice(alloc, current.items);
            current.clearRetainingCapacity();
        }
        if (current.items.len != 0) try current.append(alloc, ' ');
        try current.appendSlice(alloc, word);
    }
    if (wrote_line) try out.append(alloc, '\n');
    try out.appendNTimes(alloc, ' ', indent);
    try out.appendSlice(alloc, current.items);
    return out.toOwnedSlice(alloc);
}

/// Center or flush-right a single line's text between the margins (ASM
/// `Center`, `zde17.asm:5691`): trim the line's own leading/trailing spaces,
/// then pad on the left with half the leftover field width (center) or all
/// of it (flush right). Caller is responsible for the "`right_margin == 1`
/// means off" no-op check (ASM `RET Z` at the top of `Center`), and frees the
/// result.
pub fn centerLine(alloc: Allocator, text: []const u21, left_margin: usize, right_margin: usize, flush_right: bool) ![]u21 {
    var lo: usize = 0;
    var hi: usize = text.len;
    while (lo < hi and isWhitespace(text[lo])) : (lo += 1) {}
    while (hi > lo and isWhitespace(text[hi - 1])) : (hi -= 1) {}
    const trimmed = text[lo..hi];

    const field_width = right_margin -| left_margin + 1;
    const available = field_width -| trimmed.len;
    const padding = if (flush_right) available else available / 2;
    const indent = (left_margin -| 1) + padding;

    var out: std.ArrayList(u21) = .empty;
    errdefer out.deinit(alloc);
    try out.appendNTimes(alloc, ' ', indent);
    try out.appendSlice(alloc, trimmed);
    return out.toOwnedSlice(alloc);
}

// --- tests ---------------------------------------------------------------

const testing = std.testing;

fn u21s(comptime s: []const u8) [s.len]u21 {
    var out: [s.len]u21 = undefined;
    for (s, 0..) |c, i| out[i] = c;
    return out;
}

test "displayColumn expands hard tabs" {
    const line = u21s("a\tb");
    try testing.expectEqual(@as(usize, 1), displayColumn(&line, 1, 4));
    try testing.expectEqual(@as(usize, 4), displayColumn(&line, 2, 4));
    try testing.expectEqual(@as(usize, 5), displayColumn(&line, 3, 4));
}

test "nextVariableTabStop finds the next stop past a column" {
    const stops = [_]u8{ 6, 11, 16, 0, 0, 0, 0, 0 };
    try testing.expectEqual(@as(?usize, 6), nextVariableTabStop(&stops, 0));
    try testing.expectEqual(@as(?usize, 11), nextVariableTabStop(&stops, 6));
    try testing.expectEqual(@as(?usize, null), nextVariableTabStop(&stops, 16));
}

test "insertTabStop keeps the list sorted" {
    var tabs = [_]u8{ 6, 16, 0, 0, 0, 0, 0, 0 };
    try testing.expect(insertTabStop(&tabs, 11));
    try testing.expectEqualSlices(u8, &.{ 6, 11, 16, 0, 0, 0, 0, 0 }, &tabs);
}

test "insertTabStop rejects zero, duplicates and full lists" {
    var tabs = [_]u8{ 6, 11, 16, 21, 0, 0, 0, 0 };
    try testing.expect(!insertTabStop(&tabs, 0));
    try testing.expect(!insertTabStop(&tabs, 11)); // already set
    var full = [_]u8{ 1, 2, 3, 4, 5, 6, 7, 8 };
    try testing.expect(!insertTabStop(&full, 9));
}

test "removeTabStop closes the gap" {
    var tabs = [_]u8{ 6, 11, 16, 0, 0, 0, 0, 0 };
    try testing.expect(removeTabStop(&tabs, 11));
    try testing.expectEqualSlices(u8, &.{ 6, 16, 0, 0, 0, 0, 0, 0 }, &tabs);
    try testing.expect(!removeTabStop(&tabs, 99)); // not present
}

test "checkRightMargin fits at or before the margin" {
    try testing.expectEqual(WrapDecision.fits, checkRightMargin(65, 65));
    try testing.expectEqual(WrapDecision.wrap_word, checkRightMargin(66, 65));
}

test "checkRightMargin is off when right_margin is 1" {
    try testing.expectEqual(WrapDecision.fits, checkRightMargin(200, 1));
}

test "findWrapPoint locates the last space" {
    const with_space = u21s("hello world");
    try testing.expectEqual(@as(?usize, 5), findWrapPoint(&with_space));
    const without_space = u21s("onelongword");
    try testing.expectEqual(@as(?usize, null), findWrapPoint(&without_space));
}

test "reflowParagraph packs words within the field" {
    const text = u21s("the quick brown fox jumps over the lazy dog");
    const out = try reflowParagraph(testing.allocator, &text, 1, 15);
    defer testing.allocator.free(out);
    const expected = u21s("the quick brown\nfox jumps over\nthe lazy dog");
    try testing.expectEqualSlices(u21, &expected, out);
}

test "reflowParagraph indents to the left margin" {
    const text = u21s("ab cd");
    const out = try reflowParagraph(testing.allocator, &text, 5, 20);
    defer testing.allocator.free(out);
    const expected = u21s("    ab cd");
    try testing.expectEqualSlices(u21, &expected, out);
}

test "reflowParagraph is idempotent" {
    const text = u21s("the quick brown fox jumps over the lazy dog");
    const once = try reflowParagraph(testing.allocator, &text, 1, 15);
    defer testing.allocator.free(once);
    const twice = try reflowParagraph(testing.allocator, once, 1, 15);
    defer testing.allocator.free(twice);
    try testing.expectEqualSlices(u21, once, twice);
}

test "reflowParagraph leaves an overlong word on its own line" {
    const text = u21s("a supercalifragilisticexpialidocious word");
    const out = try reflowParagraph(testing.allocator, &text, 1, 10);
    defer testing.allocator.free(out);
    const expected = u21s("a\nsupercalifragilisticexpialidocious\nword");
    try testing.expectEqualSlices(u21, &expected, out);
}

test "centerLine pads half the leftover field on each conceptual side" {
    // field is columns 1..=11 (width 11), text "hi" (2 chars) -> 9 leftover, 4 padding.
    const text = u21s("hi");
    const out = try centerLine(testing.allocator, &text, 1, 11, false);
    defer testing.allocator.free(out);
    const expected = u21s("    hi");
    try testing.expectEqualSlices(u21, &expected, out);
}

test "centerLine flush right pads the full leftover" {
    const text = u21s("hi");
    const out = try centerLine(testing.allocator, &text, 1, 11, true);
    defer testing.allocator.free(out);
    const expected = u21s("         hi");
    try testing.expectEqualSlices(u21, &expected, out);
}

test "centerLine trims existing whitespace first" {
    const text = u21s("  hi  ");
    const out = try centerLine(testing.allocator, &text, 1, 11, true);
    defer testing.allocator.free(out);
    const expected = u21s("         hi");
    try testing.expectEqualSlices(u21, &expected, out);
}
