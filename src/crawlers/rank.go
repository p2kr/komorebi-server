package crawlers

import (
	"cmp"
	"math"
	"slices"
	"strconv"
	"strings"

	"komorebi-server/src/dto"

	"github.com/lithammer/fuzzysearch/fuzzy"
)

// Should always sum upto 1.0
const (
	WeightTitle      = 0.60
	WeightPopularity = 0.25
	WeightSource     = 0.15
)

// PopularSources contains Lowercase sources
var PopularSources = []string{"nyaa"}

func RankCrawlerResults(query string, dtos []dto.CrawlerResult) {
	scores := make([]float64, len(dtos))

	for i, result := range dtos {
		var popScore float64
		var titleScore float64
		var sourceScore float64

		// Popularity Heuristic (0.0 to 1.0 scale, capped at 10,000 seeders)
		if result.Popularity != nil {
			if pop, err := strconv.ParseFloat(*result.Popularity, 64); err == nil && pop > 0 {
				// log10(10001) is ~4.0, dividing by 4 normalizes it to ~1.0 at 10k seeders
				popScore = math.Log10(pop+1) / 4.0
				// Cap it at 1.0 for anything above 10,000 seeders
				popScore = math.Min(popScore, 1.0)
			}
		}

		// Title Match Heuristic (Converted from distance to similarity: 0.0 to 1.0)
		fuzzDist := fuzzy.RankMatchFold(query, result.Title)
		if fuzzDist != -1 {
			maxLen := float64(max(len(result.Title), len(query)))
			if maxLen > 0 {
				// Distance ratio inverted so 1.0 = identical, 0.0 = completely different
				similarity := 1.0 - (float64(fuzzDist) / maxLen)
				titleScore = math.Max(0.0, similarity)
			}
		}

		// Source heuristic (popular sources like Nyaa)
		sourceLower := strings.ToLower(result.Source)
		for _, source := range PopularSources {
			if strings.Contains(sourceLower, source) {
				sourceScore = 1.0
				break
			}
		}

		// Weighted combination
		scores[i] = (titleScore * WeightTitle) +
			(popScore * WeightPopularity) +
			(sourceScore * WeightSource)
	}

	indices := make([]int, len(dtos))
	for i := range indices {
		indices[i] = i
	}

	// Sort descending (highest score first)
	slices.SortFunc(indices, func(a, b int) int {
		return cmp.Compare(scores[b], scores[a])
	})

	temp := make([]dto.CrawlerResult, len(dtos))
	for i, idx := range indices {
		temp[i] = dtos[idx]
	}
	copy(dtos, temp)
}
