package mal

import (
	"encoding/json"
	"fmt"

	"komorebi-server/configs"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"

	"github.com/rs/zerolog/log"
	"github.com/tidwall/gjson"
	"resty.dev/v3"
)

const (
	DEFAULT_MAL_BASE_URL = "https://api.myanimelist.net/v2"
	HEADER_NAME          = "X-MAL-CLIENT-ID"
	MAL_ANIME_FIELDS     = "synopsis,media_type,my_list_status,rating,mean,num_episodes,popularity,alternative_titles,genres"
	MAL_MANGA_FIELDS     = "synopsis,media_type,my_list_status,mean,num_chapters,num_volumes,popularity,alternative_titles,genres"
	USER_INFO_URL        = "https://api.myanimelist.net/v2/users/@me?fields=id,name,picture"
	MAL_TOKEN_URL        = "https://myanimelist.net/v1/oauth2/token"
	CORS_HEADER          = "Access-Control-Allow-Origin"
)

type MalClient struct {
	client *resty.Client
	user   *models.User
}

func NewMalClient(client *resty.Client, user *models.User) *MalClient {
	return &MalClient{client: client, user: user}
}

func (c *MalClient) fetchList(params *dto.MediaClientParams, isManga bool) (dto.PaginatedResponse, error) {
	var endpoint string
	var fields string
	if isManga {
		endpoint = "managalist"
		fields = MAL_MANGA_FIELDS
	} else {
		endpoint = "animelist"
		fields = MAL_ANIME_FIELDS
	}

	url := DEFAULT_MAL_BASE_URL + "/users/" + c.user.Username + "/" + endpoint

	req := c.client.R()

	if params != nil {
		if params.Status != nil {
			if malStatus, err := ParseMalStatus(*params.Status, isManga); err == nil {
				req.SetQueryParam("status", string(malStatus))
			} else {
				req.SetQueryParam("status", *params.Status)
			}
		}
		if params.Sort != nil {
			req.SetQueryParam("sort", *params.Sort)
		}
		if params.Limit == nil {
			req.SetQueryParam("limit", "50")
		} else {
			req.SetQueryParamAny("limit", *params.Limit)
		}
		if params.Offset == nil {
			req.SetQueryParam("offset", "0")
		} else {
			req.SetQueryParamAny("offset", *params.Offset)
		}
	} else {
		req.SetQueryParam("limit", "50")
		req.SetQueryParam("offset", "0")
	}

	if c.user.AccessToken != nil {
		req.SetAuthToken(*c.user.AccessToken)
	}

	req.SetQueryParam("fields", fields)
	req.SetHeader(HEADER_NAME, configs.GetConfig().Env.MalClientId)

	resp, err := req.Get(url)
	if err != nil || resp.StatusCode() != 200 {
		log.Err(err).Str("request", req.CurlCmd()).Bool("isManga", isManga).Msg("Failed to get mal list")
		return dto.PaginatedResponse{}, fmt.Errorf("%w response %s", err, resp.Status())
	}

	var malResp MalResponse
	err = json.Unmarshal(resp.Bytes(), &malResp)
	if err != nil {
		log.Err(err).Str("response", resp.String()).Msg("Failed to parse response in get mal list")
		return dto.PaginatedResponse{}, err
	}
	return malResp.ToPaginatedResponse(), nil
}

func (c *MalClient) GetAnimeList(params dto.MediaClientParams) (dto.PaginatedResponse, error) {
	return c.fetchList(&params, false)
}

func (c *MalClient) GetMangaList(params dto.MediaClientParams) (dto.PaginatedResponse, error) {
	return c.fetchList(&params, true)
}

func (c *MalClient) ValidateNewUser(accessToken string) error {
	req := c.client.R().SetHeader(HEADER_NAME, configs.GetConfig().Env.MalClientId).SetAuthToken(*c.user.AccessToken)
	resp, err := req.Get(USER_INFO_URL)
	if err != nil || resp.StatusCode() != 200 {
		log.Err(err).Any("request", req.Body).Msg("Failed to ValidateNewUser in MAL")
		return fmt.Errorf("%w response %s", err, resp.Status())
	}

	name := gjson.GetBytes(resp.Bytes(), "name")
	id := gjson.GetBytes(resp.Bytes(), "id")
	picture := gjson.GetBytes(resp.Bytes(), "picture")

	if !name.Exists() {
		log.Err(err).Str("response", resp.String()).Msg("Failed to unmarshal resp")
		return err
	}

	c.user.Username = name.String()
	c.user.ProviderId = new(id.String())
	c.user.AvatarUrl = new(picture.String())
	c.user.Provider = dto.MediaProviderMAL
	c.user.AccessToken = &accessToken
	c.user.IsSandbox = false

	return nil
}

func (c *MalClient) ExchangeOauthToken(code, codeVerifier string) (string, error) {
	req := c.client.R().SetHeader(CORS_HEADER, MAL_TOKEN_URL).SetFormData(map[string]string{
		"client_id":     configs.GetConfig().Env.MalClientId,
		"code":          code,
		"code_verifier": codeVerifier,
		"grant_type":    "authorization_code",
		"redirect_uri":  configs.GetConfig().Env.DefaultHostedAuthPage,
	})

	resp, err := req.Post(MAL_TOKEN_URL)
	if err != nil {
		log.Err(err).Str("request", req.CurlCmd()).Msg("Failed to send req for ExchangeOauthToken")
		return "", err
	}

	result := gjson.GetBytes(resp.Bytes(), "access_token")
	if !result.Exists() {
		log.Err(err).Str("response", resp.String()).Msg("Failed to unmarshal resp")
		return "", err
	}

	return result.String(), nil
}
