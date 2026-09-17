package anilist

import (
	"encoding/json"
	"errors"
	"komorebi-server/configs"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"
	"strconv"
	"strings"

	"github.com/rs/zerolog/log"
	"resty.dev/v3"
)

type AnilistClient struct {
	client *resty.Client
	user   *models.User
}

func NewAnilistClient(client *resty.Client, user *models.User) *AnilistClient {
	return &AnilistClient{client: client, user: user}
}

const ANILIST_GRAPHQL_URL = "https://graphql.anilist.co"

const MEDIA_LIST_QUERY = `
query ($userName: String, $type: MediaType, $status: MediaListStatus, $page: Int, $perPage: Int) {
  Page(page: $page, perPage: $perPage) {
    pageInfo {
      total
      perPage
      currentPage
      lastPage
      hasNextPage
    }
    mediaList(userName: $userName, type: $type, status: $status) {
      id
      status
      score(format: POINT_10_DECIMAL)
      progress
      progressVolumes
      repeat
      notes
      updatedAt
      media {
        id
        idMal
        type
        format
        status
        title {
          romaji
          english
          native
          userPreferred
        }
        coverImage {
          extraLarge
          large
          medium
          color
        }
        description
        meanScore
        popularity
        episodes
        duration
        chapters
        volumes
        genres
        isAdult
      }
    }
  }
}`

const USER_INFO_QUERY = `
query {
  Viewer {
    id
    name
    about
    avatar {
      medium
    }
    bannerImage
  }
}`

func normalizeAnilistStatus(status string) dto.ListStatus {
	switch strings.ToUpper(status) {
	case "CURRENT", "WATCHING", "READING":
		return dto.ListStatusCurrent
	case "PLANNING", "PLAN_TO_WATCH", "PLAN_TO_READ":
		return dto.ListStatusPlanning
	case "COMPLETED":
		return dto.ListStatusCompleted
	case "DROPPED":
		return dto.ListStatusDropped
	case "PAUSED", "ON_HOLD":
		return dto.ListStatusPaused
	case "REPEATING", "REWATCHING", "REREADING":
		return dto.ListStatusRepeating
	default:
		return ""
	}
}

func (c *AnilistClient) fetchList(params *dto.MediaClientParams, mediaType string) (dto.PaginatedResponse, error) {
	perPage := 50
	page := 1
	if params != nil {
		if params.Limit != nil && *params.Limit > 0 {
			perPage = *params.Limit
		}
		if params.Offset != nil && *params.Offset > 0 {
			page = *params.Offset
		}
	}

	variables := map[string]any{
		"userName": c.user.Username,
		"type":     mediaType,
		"page":     page,
		"perPage":  perPage,
	}

	if params != nil && params.Status != nil {
		status := normalizeAnilistStatus(*params.Status)
		if status != "" {
			variables["status"] = status
		}
	}

	payload := map[string]any{
		"query":     MEDIA_LIST_QUERY,
		"variables": variables,
	}

	log.Info().Any("payload", payload).Any("user", c.user.Id).Msg("Sending anilist request ")

	req := c.client.R().SetBody(payload)

	if c.user.AccessToken != nil {
		req.SetAuthToken(*c.user.AccessToken)
	}

	req.SetQueryParam("client_id", configs.GetConfig().Env.AnilistClientId)

	resp, err := req.Post(ANILIST_GRAPHQL_URL)
	if err != nil || resp.StatusCode() != 200 {
		log.Err(err).Str("request", req.CurlCmd()).Msg("Failed to get anilist list")
		return dto.PaginatedResponse{}, errors.Join(err, errors.New("response : "+resp.Status()))
	}

	var anilistResp AniListResponse
	err = json.Unmarshal(resp.Bytes(), &anilistResp)
	if err != nil {
		log.Err(err).Str("response", resp.String()).Msg("Failed to parse response in get anilist list")
		return dto.PaginatedResponse{}, err
	}

	return anilistResp.ToPaginatedResponse(), nil
}

func (c *AnilistClient) GetAnimeList(params dto.MediaClientParams) (dto.PaginatedResponse, error) {
	return c.fetchList(&params, "ANIME")
}

func (c *AnilistClient) GetMangaList(params dto.MediaClientParams) (dto.PaginatedResponse, error) {
	return c.fetchList(&params, "MANGA")
}

func (c *AnilistClient) ValidateNewUser(accessToken string) error {
	req := c.client.R().SetBody(map[string]any{
		"query":     USER_INFO_QUERY,
		"client_id": configs.GetConfig().Env.AnilistClientId,
	}).SetAuthToken(accessToken).SetHeader("Content-Type", "application/json")

	resp, err := req.Post(ANILIST_GRAPHQL_URL)
	if err != nil || resp.StatusCode() != 200 {
		log.Err(err).Str("request", req.CurlCmd()).Msg("Failed to get anilist validation")
		return errors.Join(err, errors.New("response : "+resp.Status()))
	}

	var result struct {
		Data struct {
			Viewer struct {
				Name   string `json:"name"`
				Id     int    `json:"id"`
				Avatar struct {
					Medium string `json:"medium"`
				} `json:"avatar"`
			} `json:"viewer"`
		} `json:"data"`
	}

	err = json.Unmarshal(resp.Bytes(), &result)
	if err != nil {
		log.Err(err).Str("response", resp.String()).Msg("Failed to unmarshal resp")
		return err
	}

	id := strconv.Itoa(result.Data.Viewer.Id)
	c.user.Username = result.Data.Viewer.Name
	c.user.ProviderId = &id
	c.user.AvatarUrl = &result.Data.Viewer.Avatar.Medium
	c.user.Provider = dto.MediaProviderAnilist
	c.user.AccessToken = &accessToken
	c.user.IsSandbox = false

	return nil
}

func (c *AnilistClient) ExchangeOauthToken(code string, codeVerifier string) (string, error) {
	code, codeVerifier = strings.TrimSpace(code), strings.TrimSpace(codeVerifier)
	if code == "" || codeVerifier == "" {
		msg := "code and codeVerifier both cannot be empty"
		log.Error().Msg(msg)
		return "", errors.New(msg)
	} else {
		if code != "" {
			return code, nil
		} else {
			return codeVerifier, nil
		}
	}
}
