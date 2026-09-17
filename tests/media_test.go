package tests

import (
	"encoding/json"
	"testing"
	"uuid"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	"komorebi-server/src/dto"
)

func TestMediaJSON(t *testing.T) {
	u, err := uuid.Parse("01a09f14-a14d-7a42-8e4f-2ccbbb33e32e")
	require.NoError(t, err, "failed to parse uuid")

	score := float32(8.5)
	episodes := 12

	media := dto.Media{
		ID:            u,
		ProviderID:    "12345",
		Provider:      dto.MediaProviderMAL,
		MediaType:     dto.MediaTypeAnime,
		Format:        dto.MediaFormatTv,
		ReleaseStatus: dto.ReleaseStatusFinished,
		Title: dto.MediaTitle{
			Romanized: ptr("Frieren"),
		},
		Cover: dto.CoverImage{
			Large: ptr("https://example.com/cover.jpg"),
		},
		MeanScore: &score,
		Episodes:  &episodes,
		Genres:    []string{"Adventure", "Drama", "Fantasy"},
		Nsfw:      dto.NsfwLevelSafe,
	}

	entry := dto.MediaEntry{
		Media: media,
		ListEntry: dto.ListEntry{
			Status:   dto.ListStatusCompleted,
			Score:    &score,
			Progress: &episodes,
			Tags:     []string{"favorite"},
		},
	}

	resp := dto.PaginatedResponse{
		Data: []dto.MediaEntry{entry},
		Paging: dto.Paging{
			HasNext: false,
		},
	}

	data, err := json.Marshal(resp)
	require.NoError(t, err, "failed to marshal json")

	var decoded dto.PaginatedResponse
	err = json.Unmarshal(data, &decoded)
	require.NoError(t, err, "failed to unmarshal json")

	assert.Equal(t, u, decoded.Data[0].Media.ID, "expected uuid")
	assert.Equal(t, dto.MediaProviderMAL, decoded.Data[0].Media.Provider, "expected provider MAL")
}
