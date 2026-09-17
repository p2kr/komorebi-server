package tests

import (
	"bytes"
	"komorebi-server/src/controllers"
	"net/http"
	"net/http/httptest"
	"resty.dev/v3"
	"testing"

	"github.com/stretchr/testify/assert"

	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
)

func TestRestyLogging(t *testing.T) {
	var buf bytes.Buffer
	log.Logger = zerolog.New(&buf).Level(zerolog.DebugLevel)

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"message":"hello"}`))
	}))
	defer server.Close()

	client := resty.New()
	client.SetLogger(&controllers.RestyLogger{})
	client.SetDebug(true)
	resp, err := client.R().Get(server.URL)
	assert.NoError(t, err, "unexpected error")
	assert.Equal(t, 200, resp.StatusCode(), "expected 200 status code")

	logged := buf.String()
	t.Logf("Captured log output:\n%s", logged)
	assert.NotEmpty(t, logged, "expected resty logs, but got none!")
}

func TestRestyLoggerLevels(t *testing.T) {
	var buf bytes.Buffer
	log.Logger = zerolog.New(&buf).Level(zerolog.DebugLevel)

	logger := &controllers.RestyLogger{}

	logger.Debugf("debug code=%d name=%s", 1, "foo")
	assert.Contains(t, buf.String(), "debug code=1 name=foo", "unexpected debug output")
	buf.Reset()

	logger.Warnf("warn code=%d name=%s", 2, "bar")
	assert.Contains(t, buf.String(), "warn code=2 name=bar", "unexpected warn output")
	buf.Reset()

	logger.Errorf("error code=%d name=%s", 3, "baz")
	assert.Contains(t, buf.String(), "error code=3 name=baz", "unexpected error output")
}
