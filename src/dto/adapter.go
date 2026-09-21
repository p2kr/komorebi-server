package dto

import (
	"uuid"

	"github.com/Oudwins/zog"
)

type MediaClientParams struct {
	UserId uuid.UUID `json:"user_id,omitempty"`
	Status *string   `json:"status,omitempty"`
	Sort   *string   `json:"sort,omitempty"`
	Limit  *int      `json:"limit,omitempty"`
	Offset *int      `json:"offset,omitempty"`
}

type ExchangeOauthParams struct {
	Code         string        `json:"code,omitempty"`
	CodeVerifier string        `json:"code_verifier,omitempty"`
	Provider     MediaProvider `json:"provider,omitempty"`
}

var IExchangeOauthParams = zog.Struct(zog.Shape{
	"Code":         zog.String().Min(1),
	"CodeVerifier": zog.String().Min(1),
	"Provider":     zog.StringLike[MediaProvider]().OneOf([]MediaProvider{MediaProviderAnilist, MediaProviderMAL}),
})
