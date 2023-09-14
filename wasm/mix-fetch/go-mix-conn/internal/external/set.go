package external

// SOURCE: https://github.com/jub0bs/fcors/blob/main/internal/util/set.go

// A Set represents a mathematical set whose elements have type E.
type Set[E comparable] map[E]struct{}

// NewSet returns a Set that contains all of es (and no other elements).
func NewSet[E comparable](es ...E) Set[E] {
	set := make(Set[E], len(es))
	for _, e := range es {
		set.Add(e)
	}
	return set
}

// Add adds e to s.
func (s Set[E]) Add(e E) {
	s[e] = struct{}{}
}

// Contains returns true if e is an element of s, and false otherwise.
func (s Set[E]) Contains(e E) bool {
	_, found := s[e]
	return found
}
