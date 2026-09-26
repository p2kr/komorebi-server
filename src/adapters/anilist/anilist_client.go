package anilist

import (
	"encoding/json"
	"errors"
	"fmt"
	"strings"

	"komorebi-server/configs"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"

	"github.com/rs/zerolog/log"
	"github.com/tidwall/gjson"
	"resty.dev/v3"
)

type AnilistClient struct {
	client *resty.Client
	user   *models.User
}

func NewAnilistClient(client *resty.Client, user *models.User) *AnilistClient {
	return &AnilistClient{client: client, user: user}
}

const GraphqlUrl = "https://graphql.anilist.co"

const MediaListQuery = `
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

const UserInfoQuery = `
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
			variables["status"] = strings.ToUpper(string(status))
		}
	}

	payload := map[string]any{
		"query":     MediaListQuery,
		"variables": variables,
	}

	log.Info().Any("payload", payload).Any("user", c.user.Id).Msg("Sending anilist request ")

	req := c.client.R().SetBody(payload)

	if c.user.AccessToken != nil {
		req.SetAuthToken(*c.user.AccessToken)
	}

	req.SetQueryParam("client_id", configs.GetConfig().Env.AnilistClientId)

	resp, err := req.Post(GraphqlUrl)
	if err != nil || resp.StatusCode() != 200 {
		log.Err(err).Str("request", req.CurlCmd()).Msg("Failed to get anilist list")
		return dto.PaginatedResponse{}, fmt.Errorf("%w response %s", err, resp.Status())
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
		"query":     UserInfoQuery,
		"client_id": configs.GetConfig().Env.AnilistClientId,
	}).SetAuthToken(accessToken).SetHeader("Content-Type", "application/json")

	resp, err := req.Post(GraphqlUrl)
	if err != nil || resp.StatusCode() != 200 {
		log.Err(err).Str("request", req.CurlCmd()).Msg("Failed to get anilist validation")
		return fmt.Errorf("%w response %s", err, resp.Status())
	}

	name := gjson.GetBytes(resp.Bytes(), "data.Viewer.name")
	id := gjson.GetBytes(resp.Bytes(), "data.Viewer.id")
	avatar := gjson.GetBytes(resp.Bytes(), "data.Viewer.avatar.medium")

	if !name.Exists() {
		log.Err(err).Str("response", resp.String()).Msg("Failed to unmarshal resp")
		return err
	}

	c.user.Username = name.String()
	c.user.ProviderId = new(id.String())
	c.user.AvatarUrl = new(avatar.String())
	c.user.Provider = dto.MediaProviderAnilist
	c.user.AccessToken = &accessToken
	c.user.IsSandbox = false

	return nil
}

func (c *AnilistClient) ExchangeOauthToken(code, codeVerifier string) (string, error) {
	code, codeVerifier = strings.TrimSpace(code), strings.TrimSpace(codeVerifier)
	if code == "" || codeVerifier == "" {
		msg := "code and codeVerifier both cannot be empty"
		log.Error().Msg(msg)
		return "", errors.New(msg)
	}

	if code != "" {
		return code, nil
	}

	return codeVerifier, nil
}
