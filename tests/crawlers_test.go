package tests

import (
	"bytes"
	"komorebi-server/src/crawlers"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
	"github.com/stretchr/testify/assert"
	"resty.dev/v3"
)

func TestCrawlerEngineLogging(t *testing.T) {
	var buf bytes.Buffer
	origLogger := log.Logger
	defer func() { log.Logger = origLogger }()
	log.Logger = zerolog.New(&buf).Level(zerolog.DebugLevel)

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`
			<html>
				<body>
					<div class="row">
						<a class="link" href="https://example.com/item1">Long title for item one here</a>
						<span class="pop">100</span>
						<span class="size">1.5GB</span>
					</div>
				</body>
			</html>
		`))
	}))
	defer server.Close()

	popSel := ".pop"
	sizeSel := ".size"
	configs := []models.CrawlerConfig{
		{
			Key:       "deleted_config",
			IsDeleted: true,
			Url:       server.URL + "?q={query}",
		},
		{
			Key:                "active_html_config",
			Category:           dto.MediaTypeAnime,
			IsDeleted:          false,
			Url:                server.URL + "?q={query}",
			RowSelector:        ".row",
			TitleSelector:      ".link",
			LinkSelector:       ".link",
			PopularitySelector: &popSel,
			SizeSelector:       &sizeSel,
		},
	}

	client := resty.New()
	engine := crawlers.CrawlerEngine{
		Client:    client,
		Query:     "test",
		MediaType: dto.MediaTypeAnime,
		Configs:   &configs,
	}

	results, err := engine.Crawl()
	assert.NoError(t, err)
	assert.Len(t, results, 1)
	assert.Equal(t, "Long title for item one here", results[0].Title)
	assert.Equal(t, "https://example.com/item1", results[0].Link)
	assert.Equal(t, dto.MediaTypeAnime, results[0].Category)
	assert.Equal(t, "active_html_config", results[0].Source)

	logged := buf.String()
	t.Logf("Captured crawler logs:\n%s", logged)

	// Verify that deleted_config did NOT log
	assert.NotContains(t, logged, "deleted_config")

	// Verify that active_html_config (18 chars) logged truncated config key (15 chars + "...")
	assert.Contains(t, logged, `"config":"active_html_con..."`)
	assert.Contains(t, logged, "Crawling Info")
	assert.Contains(t, logged, `"crawler":"html"`)
	assert.Contains(t, logged, `"results":1`)
	assert.Contains(t, logged, `"title":"Long title for ..."`)
	assert.Contains(t, logged, `"link":"https://example..."`)
}

func TestCrawlerEngineFetchHtmlErrorHandling(t *testing.T) {
	var buf bytes.Buffer
	origLogger := log.Logger
	defer func() { log.Logger = origLogger }()
	log.Logger = zerolog.New(&buf).Level(zerolog.DebugLevel)

	// Server returning 500 status code
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusInternalServerError)
	}))
	defer server.Close()

	configs := []models.CrawlerConfig{
		{
			Key:       "failing_config",
			IsDeleted: false,
			Url:       server.URL + "?q={query}",
		},
	}

	client := resty.New()
	engine := crawlers.CrawlerEngine{
		Client:    client,
		Query:     "test",
		MediaType: dto.MediaTypeAnime,
		Configs:   &configs,
	}

	_, err := engine.Crawl()
	assert.Error(t, err)

	logged := buf.String()
	t.Logf("Captured error logs:\n%s", logged)
	assert.Contains(t, logged, "Failed to get [url]")
	assert.Contains(t, logged, "fetch html err")
	assert.Contains(t, logged, "Crawling Info")
}

func TestJsonCrawlerLogging(t *testing.T) {
	var buf bytes.Buffer
	origLogger := log.Logger
	defer func() { log.Logger = origLogger }()
	log.Logger = zerolog.New(&buf).Level(zerolog.DebugLevel)

	jsonContent := `[
		{"name": "Solo Leveling", "url": "https://example.com/solo"},
		{"name": "", "url": "not-a-valid-url"}
	]`

	config := models.CrawlerConfig{
		Key:           "json_test",
		Category:      dto.MediaTypeAnime,
		RowSelector:   "$[*]",
		TitleSelector: "$[*].name",
		LinkSelector:  "$[*].url",
	}

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(jsonContent))
	}))
	defer server.Close()

	config.Url = server.URL

	configs := []models.CrawlerConfig{config}
	client := resty.New()
	engine := crawlers.CrawlerEngine{
		Client:    client,
		Query:     "solo",
		MediaType: dto.MediaTypeAnime,
		Configs:   &configs,
	}

	results, err := engine.Crawl()
	assert.NoError(t, err)
	assert.Len(t, results, 1)
	assert.Equal(t, "Solo Leveling", results[0].Title)
	assert.Equal(t, "https://example.com/solo", results[0].Link)
	assert.Equal(t, "json_test", results[0].Source)
	assert.Equal(t, dto.MediaTypeAnime, results[0].Category)

	logged := buf.String()
	t.Logf("Captured JSON crawler logs:\n%s", logged)
	assert.Contains(t, logged, "invalid dto")
	assert.Contains(t, logged, `"crawler":"json"`)
	assert.Contains(t, logged, `"results":1`)
}
