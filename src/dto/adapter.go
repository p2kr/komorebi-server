package dto

import "uuid"

type MediaClientParams struct {
	UserId uuid.UUID
	Status *string
	Sort   *string
	Limit  *int
	Offset *int
}
