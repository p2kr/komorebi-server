package crawlers

import (
	"net/url"
	"strings"
	"sync"

	"github.com/PuerkitoBio/goquery"
	"github.com/andybalholm/cascadia"
	"github.com/maypok86/otter/v2"
	"github.com/ohler55/ojg/jp"
	"github.com/rs/zerolog/log"
	"golang.org/x/net/html"
)

// JpExprCache is Global jp.Expr Cache for crawlers
var JpExprCache = sync.OnceValue(func() *otter.Cache[string, jp.Expr] {
	r, err := otter.New(&otter.Options[string, jp.Expr]{
		MaximumSize: 1000,
	})
	if err != nil {
		log.Err(err).Msg("Failed to instantiate crawler cache")
	}
	return r
})

var CssMatcherCache = sync.OnceValue(func() *otter.Cache[string, goquery.Matcher] {
	r, err := otter.New(&otter.Options[string, goquery.Matcher]{
		MaximumSize: 1000,
	})
	if err != nil {
		log.Err(err).Msg("Failed to instantiate crawler cache")
	}
	return r
})

var emptyString = ""

func getJpExpr(s string) (jp.Expr, bool) {
	return JpExprCache().ComputeIfAbsent(s, func() (newValue jp.Expr, cancel bool) {
		r, err := jp.ParseString(s)
		if err != nil {
			log.Err(err).Str("expr", s).Msg("Failed to parse jp expr")
			return r, true
		}
		return r, false
	})
}

func getMatcher(s string) goquery.Matcher {
	r, ok := CssMatcherCache().ComputeIfAbsent(s, func() (newValue goquery.Matcher, cancel bool) {
		r, err := cascadia.Compile(s)
		if err != nil {
			log.Err(err).Str("expr", s).Msg("Failed to parse css expr")
			return r, true
		}
		return r, false
	})
	if !ok {
		return invalidMatcher{}
	}
	return r
}

func findWithMatcher(s *goquery.Selection, text string) *goquery.Selection {
	return s.FindMatcher(getMatcher(text))
}

// invalidMatcher is a Matcher that always fails to match.
type invalidMatcher struct{}

func (invalidMatcher) Match(n *html.Node) bool             { return false }
func (invalidMatcher) MatchAll(n *html.Node) []*html.Node  { return nil }
func (invalidMatcher) Filter(ns []*html.Node) []*html.Node { return nil }

func truncate(s string, limit ...int) string {
	max := 15
	if len(limit) > 0 {
		max = limit[0]
	}
	i := 0
	for pos := range s {
		if i == max {
			return s[:pos] + "..."
		}
		i++
	}
	return s
}

// ReplaceInURL takes a raw URL template and a map of replacements,
// safely applying them to both the path and query string.
func ReplaceInURL(rawURL string, replacements map[string]string) (string, error) {
	u, err := url.Parse(rawURL)
	if err != nil {
		return "", err
	}

	for oldStr, newStr := range replacements {
		// 1. Replace in Path
		u.Path = strings.ReplaceAll(u.Path, oldStr, newStr)

		// 2. Replace in Query
		q := u.Query()
		for key, values := range q {
			for i, val := range values {
				q[key][i] = strings.ReplaceAll(val, oldStr, newStr)
			}
		}
		u.RawQuery = q.Encode()
	}

	return u.String(), nil
}

// If title is still empty and the link is a magnet URI, derive
// title (and optionally size) from the magnet's own metadata.
func parseTitleAndSize(link string) (title, size string) {
	if strings.HasPrefix(link, "magnet:") {
		if u, err := url.Parse(link); err == nil {
			q := u.Query()
			if dn := q.Get("dn"); dn != "" {
				title = dn
			}
			if xl := q.Get("xl"); xl != "" {
				size = xl
			}
		}
	}

	return title, size
}
