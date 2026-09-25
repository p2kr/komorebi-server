package controllers

import (
	"context"
	"fmt"
	"net"
	"net/http"
	"net/url"
	"strings"
	"sync"
	"uuid"

	"komorebi-server/configs"
	"komorebi-server/src/db"
	"komorebi-server/src/models"

	"github.com/labstack/echo/v5"
	"github.com/rs/zerolog/log"
	"gorm.io/gorm"
	"gorm.io/gorm/clause"
	"resty.dev/v3"

	"github.com/maypok86/otter/v2"

	"github.com/vmihailenco/msgpack/v5"
)

type SuccessResponse[T any] struct {
	Success bool `json:"success" tstype:"true"`
	Data    T    `json:"data"`
}

type FailureResponse struct {
	Success    bool   `json:"success" tstype:"false | undefined"`
	StatusCode int    `json:"status_code"`
	Message    string `json:"message"`
	Details    any    `json:"details,omitempty"`
}

func success[T any](c *echo.Context, data T, customStatus ...int) error {
	if strings.Contains(c.Request().Header.Get("Accept"), "application/x-msgpack") {
		return successMsgPack(c, data)
	}
	status := http.StatusOK
	if len(customStatus) > 0 {
		status = customStatus[0]
	}
	return c.JSON(status, SuccessResponse[T]{
		Success: true,
		Data:    data,
	})
}

func successMsgPack[T any](c *echo.Context, data T) error {
	out, err := msgpack.Marshal(SuccessResponse[T]{
		Success: true,
		Data:    data,
	})
	if err != nil {
		return err
	}
	return c.Blob(200, "application/x-msgpack", out)
}

func fail(c *echo.Context, status int, error error, details ...any) error {
	if strings.Contains(c.Request().Header.Get("Accept"), "application/x-msgpack") {
		return failMsgPack(c, status, error, details...)
	}
	return c.JSON(status, FailureResponse{
		Success:    false,
		StatusCode: status,
		Message:    fmt.Sprintf("%s", error),
		Details:    details,
	})
}

func failMsgPack(c *echo.Context, status int, error error, details ...any) error {
	out, err := msgpack.Marshal(FailureResponse{
		Success: false,
		Message: error.Error(),
		Details: details,
	})
	if err != nil {
		return err
	}
	return c.Blob(status, "application/x-msgpack", out)
}

var httpClient *resty.Client

func InitClient() {
	c := resty.NewWithTransportSettings(&resty.TransportSettings{MaxIdleConnsPerHost: 10})

	if configs.GetConfig().HttpClient.Debug {
		c.SetLogger(&RestyLogger{})
		c.SetDebug(true)
	}
	if configs.GetConfig().HttpClient.CurlCmd {
		c.SetCurlCmdGenerate(true)
	}

	c.SetHeaders(map[string]string{
		"User-Agent":      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
		"Accept":          "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
		"Accept-Language": "en-US,en;q=0.9",
	})

	httpClient = c
}

type RestyLogger struct{}

func (r *RestyLogger) Errorf(format string, v ...any) {
	log.Error().Msgf(format, v...)
}

func (r *RestyLogger) Warnf(format string, v ...any) {
	log.Warn().Msgf(format, v...)
}

func (r *RestyLogger) Debugf(format string, v ...any) {
	log.Debug().Msgf(format, v...)
}

// Cache is global Cache for controllers
var Cache = sync.OnceValue(func() *otter.Cache[string, any] {
	c, err := otter.New[string, any](&otter.Options[string, any]{
		MaximumSize: 1000,
	})
	if err != nil {
		log.Err(err).Msg("failed to initialize controller cache")
	}
	return c
})

func getCrawlerConfigs(ctx context.Context) ([]models.CrawlerConfig, bool) {
	r, ok := Cache().ComputeIfAbsent("configs", func() (any, bool) {
		var c []models.CrawlerConfig
		c, err := gorm.G[models.CrawlerConfig](db.GetDb()).Find(ctx)
		if err != nil {
			log.Err(err).Any("config from db", c).Msg("failed to get db config")
			return c, true
		}
		return c, false
	})
	m, ok := r.([]models.CrawlerConfig)
	return m, ok
}

// IsRestrictedIP checks if an IP belongs to loopback, private, link-local, or multicast blocks.
func IsRestrictedIP(ip net.IP) bool {
	if ip == nil {
		return true
	}
	return ip.IsLoopback() ||
		ip.IsPrivate() ||
		ip.IsLinkLocalUnicast() ||
		ip.IsLinkLocalMulticast() ||
		ip.IsInterfaceLocalMulticast() ||
		ip.IsMulticast() ||
		ip.IsUnspecified()
}

// validateUrl performs a basic upfront validation on the URL scheme and IP.
func validateUrl(u string) error {
	link, err := url.Parse(u)
	if err != nil {
		return fmt.Errorf("invalid url: %w", err)
	}

	// 1. Validate Scheme
	if link.Scheme != "http" && link.Scheme != "https" && link.Scheme != "magnet" {
		return fmt.Errorf("unsupported url scheme: %s", link.Scheme)
	}

	// Magnet links do not connect to a single HTTP host, so IP validation is skipped.
	if link.Scheme == "magnet" {
		return nil
	}

	hostname := link.Hostname()
	if hostname == "" {
		return fmt.Errorf("missing hostname")
	}

	// 2. Check if the hostname is a literal IP address (e.g., "192.168.1.1")
	ip := net.ParseIP(hostname)
	if ip != nil {
		if IsRestrictedIP(ip) {
			return fmt.Errorf("access to private IP blocked: %s", hostname)
		}
		return nil
	}

	// 3. If it's a domain name (e.g., "github.com"), perform a basic DNS lookup
	ips, err := net.LookupIP(hostname)
	if err != nil {
		return fmt.Errorf("could not resolve hostname %s: %w", hostname, err)
	}

	// 4. Ensure none of the resolved IPs are restricted
	for _, resolvedIP := range ips {
		if IsRestrictedIP(resolvedIP) {
			return fmt.Errorf("domain %s resolves to a blocked IP: %s", hostname, resolvedIP.String())
		}
	}

	return nil
}

var vaultItemsCache = sync.OnceValue(func() *otter.Cache[uuid.UUID, models.VaultItem] {
	cache, err := otter.New(&otter.Options[uuid.UUID, models.VaultItem]{
		MaximumSize: 1000,
	})
	if err != nil {
		log.Err(err).Msg("failed to create vault items cache")
	}
	return cache
})

func GetCachedVaultItems(key uuid.UUID) (models.VaultItem, bool) {
	return vaultItemsCache().ComputeIfAbsent(key, func() (newValue models.VaultItem, cancel bool) {
		var vaultItem models.VaultItem
		err := db.GetDb().Preload(clause.Associations).Where("id = ?", key).
			Find(&vaultItem).Error
		if err != nil {
			log.Err(err).Msg("failed to fetch vault items")
			return models.VaultItem{}, true
		}
		return vaultItem, false
	})
}
