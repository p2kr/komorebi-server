package crawlers

import "github.com/maypok86/otter/v2"

var canCrawlCacheConfig = otter.Options[string, bool]{
	MaximumSize: 100,
}
