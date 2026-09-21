package adapters

import (
	"komorebi-server/src/adapters/anilist"
	"komorebi-server/src/adapters/mal"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"

	"resty.dev/v3"
)

type MediaClient interface {
	GetAnimeList(params dto.MediaClientParams) (dto.PaginatedResponse, error)
	GetMangaList(params dto.MediaClientParams) (dto.PaginatedResponse, error)
	ValidateNewUser(accessToken string) error
	ExchangeOauthToken(code, codeVerifier string) (string, error)
}

func GetMediaClient(provider dto.MediaProvider, client *resty.Client, user *models.User) MediaClient {
	switch provider {
	case dto.MediaProviderMAL:
		return mal.NewMalClient(client, user)
	case dto.MediaProviderAnilist:
		return anilist.NewAnilistClient(client, user)
	default:
		return anilist.NewAnilistClient(client, user)
	}
}
