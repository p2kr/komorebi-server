package crawlers

import "github.com/maypok86/otter/v2"

var canCrawlCacheConfig = otter.Options[string, bool]{
	MaximumSize: 100,
}

func truncate(s string, maxLen ...int) string {
	limit := 15
	if len(maxLen) > 0 {
		limit = maxLen[0]
	}
	runes := []rune(s)
	if len(runes) > limit {
		return string(runes[:limit]) + "..."
	}
	return s
}
