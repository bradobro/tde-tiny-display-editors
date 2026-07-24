package keyboard

import "io"

// ScriptedKeys is a test-only KeySource that replays a fixed slice of keys and
// then reports io.EOF. Editor command flows drive it in unit tests so a hung
// loop (one that reads past the script) fails fast instead of blocking. It is
// the analog of the Rust and Zig test fakes.
type ScriptedKeys struct {
	keys []Key
	pos  int
}

// NewScriptedKeys returns a ScriptedKeys that will yield the given keys in order.
func NewScriptedKeys(keys ...Key) *ScriptedKeys {
	return &ScriptedKeys{keys: keys}
}

// NextKey returns the next scripted key, or io.EOF once the script is exhausted.
func (s *ScriptedKeys) NextKey() (Key, error) {
	if s.pos >= len(s.keys) {
		return Key{}, io.EOF
	}
	k := s.keys[s.pos]
	s.pos++
	return k, nil
}
