package tests

//go:fix inline
func ptr[T any](v T) *T {
	return new(v)
}
