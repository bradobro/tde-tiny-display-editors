package search

import (
	"testing"

	"zde/internal/buffer"
)

func query(find string) *Query {
	return &Query{Find: []rune(find)}
}

func TestQueryDefaultsAreForwardCaseSensitive(t *testing.T) {
	var q Query
	if q.IgnoreCase || q.Backward || q.Replace != nil {
		t.Errorf("zero Query = %+v, want all-false/nil", q)
	}
}

func TestFindFromLocatesTheNextMatch(t *testing.T) {
	buf := buffer.FromString("the quick brown fox")
	if pos, ok := FindFrom(buf, 0, query("brown")); !ok || pos != 10 {
		t.Errorf("FindFrom = %d,%v, want 10,true", pos, ok)
	}
}

func TestFindFromSkipsTheMatchAtFromWhenSearchingForwardPastIt(t *testing.T) {
	buf := buffer.FromString("aaaa")
	if pos, ok := FindFrom(buf, 1, query("a")); !ok || pos != 1 {
		t.Errorf("FindFrom(1) = %d,%v, want 1,true", pos, ok)
	}
	if _, ok := FindFrom(buf, 4, query("a")); ok {
		t.Error("FindFrom(4) ok = true, want false")
	}
}

func TestFindFromSearchesBackwardStrictlyBeforeFrom(t *testing.T) {
	buf := buffer.FromString("brown fox, brown dog")
	q := &Query{Find: []rune("brown"), Backward: true}
	if pos, ok := FindFrom(buf, 21, q); !ok || pos != 11 {
		t.Errorf("FindFrom(21) = %d,%v, want 11,true", pos, ok)
	}
	if pos, ok := FindFrom(buf, 11, q); !ok || pos != 0 {
		t.Errorf("FindFrom(11) = %d,%v, want 0,true", pos, ok)
	}
	if _, ok := FindFrom(buf, 0, q); ok {
		t.Error("FindFrom(0) backward ok = true, want false")
	}
}

func TestFindFromIsCaseInsensitiveWhenRequested(t *testing.T) {
	buf := buffer.FromString("Hello World")
	q := &Query{Find: []rune("world"), IgnoreCase: true}
	if pos, ok := FindFrom(buf, 0, q); !ok || pos != 6 {
		t.Errorf("FindFrom = %d,%v, want 6,true", pos, ok)
	}
}

func TestFindFromReturnsNoneWhenAbsent(t *testing.T) {
	buf := buffer.FromString("no match here")
	if _, ok := FindFrom(buf, 0, query("xyz")); ok {
		t.Error("FindFrom ok = true, want false")
	}
}

func TestFindFromReturnsNoneForAnEmptyQuery(t *testing.T) {
	buf := buffer.FromString("anything")
	if _, ok := FindFrom(buf, 0, &Query{}); ok {
		t.Error("FindFrom empty-query ok = true, want false")
	}
}
