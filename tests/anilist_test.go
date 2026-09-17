package tests

import (
	"encoding/json"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
	"uuid"

	"komorebi-server/src/adapters/anilist"
	"komorebi-server/src/dto"
)

func TestAniListResponseUnmarshalAndConvertAnime(t *testing.T) {
	rawJSON := `{
		"data": {
			"Page": {
				"pageInfo": {
					"total": 50,
					"perPage": 20,
					"currentPage": 1,
					"lastPage": 3,
					"hasNextPage": true
				},
				"mediaList": [
					{
						"id": 1001,
						"status": "CURRENT",
						"score": 8.5,
						"progress": 6,
						"progressVolumes": null,
						"repeat": 0,
						"notes": "Great animation",
						"updatedAt": 1690000000,
						"media": {
							"id": 5001,
							"idMal": 4001,
							"type": "ANIME",
							"format": "TV",
							"status": "RELEASING",
							"title": {
								"romaji": "Sousou no Frieren",
								"english": "Frieren: Beyond Journey's End",
								"native": "葬送のフリーレン",
								"userPreferred": "Frieren"
							},
							"coverImage": {
								"extraLarge": "https://example.com/xl.jpg",
								"large": "https://example.com/l.jpg",
								"medium": "https://example.com/m.jpg",
								"color": "#f0e68c"
							},
							"description": "An elf mage on a journey...",
							"meanScore": 91,
							"popularity": 100000,
							"episodes": 28,
							"duration": 24,
							"chapters": null,
							"volumes": null,
							"genres": ["Adventure", "Drama", "Fantasy"],
							"isAdult": false
						}
					}
				]
			}
		}
	}`

	var res anilist.AniListResponse
	err := json.Unmarshal([]byte(rawJSON), &res)
	require.NoError(t, err)

	paginated := res.ToPaginatedResponse()

	if len(paginated.Data) != 1 {
		t.Fatalf("expected 1 entry, got %d", len(paginated.Data))
	}

	entry := paginated.Data[0]

	// Verify Media
	assert.Equal(t, uuid.Nil(), entry.Media.ID)
	assert.Equal(t, dto.MediaProviderAnilist, entry.Media.Provider)
	assert.Equal(t, dto.MediaFormatTv, entry.Media.Format)
	if entry.Media.Title.Romanized == nil || *entry.Media.Title.Romanized != "Sousou no Frieren" {
		t.Errorf("expected romaji title, got %v", entry.Media.Title.Romanized)
	}
	if entry.Media.Title.English == nil || *entry.Media.Title.English != "Frieren: Beyond Journey's End" {
		t.Errorf("expected english title, got %v", entry.Media.Title.English)
	}
	assert.NotNil(t, entry.Media.Title.Native)
	assert.Equal(t, "葬送のフリーレン", *entry.Media.Title.Native)
	assert.NotNil(t, entry.Media.Cover.ExtraLarge)
	assert.Equal(t, "https://example.com/xl.jpg", *entry.Media.Cover.ExtraLarge)
	// AniList meanScore 91 normalized to 9.1
	assert.NotNil(t, entry.Media.MeanScore)
	assert.Equal(t, float32(9.1), *entry.Media.MeanScore)
	assert.NotNil(t, entry.Media.Episodes)
	assert.Equal(t, 28, *entry.Media.Episodes)
	assert.Equal(t, dto.NsfwLevelSafe, entry.Media.Nsfw)
	assert.NotNil(t, entry.ListEntry.Score)
	assert.Equal(t, float32(8.5), *entry.ListEntry.Score)
	assert.False(t, entry.ListEntry.IsRepeating)
	if entry.ListEntry.Notes == nil || *entry.ListEntry.Notes != "Great animation" {
		t.Errorf("expected notes 'Great animation', got %v", entry.ListEntry.Notes)
	}
	assert.NotNil(t, entry.ListEntry.UpdatedAt)
	assert.Equal(t, "1690000000", *entry.ListEntry.UpdatedAt)
	assert.NotNil(t, paginated.Paging.NextCursor)
	assert.Equal(t, "2", *paginated.Paging.NextCursor)
}

func TestAniListResponseUnmarshalAndConvertManga(t *testing.T) {
	rawJSON := `{
		"data": {
			"Page": {
				"pageInfo": {
					"total": 100,
					"perPage": 20,
					"currentPage": 2,
					"lastPage": 5,
					"hasNextPage": true
				},
				"mediaList": [
					{
						"id": 2001,
						"status": "COMPLETED",
						"score": 10.0,
						"progress": 300,
						"progressVolumes": 30,
						"repeat": 1,
						"media": {
							"id": 6001,
							"type": "MANGA",
							"format": "MANGA",
							"status": "FINISHED",
							"chapters": 300,
							"volumes": 30,
							"isAdult": true
						}
					}
				]
			}
		}
	}`

	var res anilist.AniListResponse
	err := json.Unmarshal([]byte(rawJSON), &res)
	require.NoError(t, err)

	paginated := res.ToPaginatedResponse()

	if len(paginated.Data) != 1 {
		t.Fatalf("expected 1 entry, got %d", len(paginated.Data))
	}

	entry := paginated.Data[0]

	assert.Equal(t, dto.MediaTypeManga, entry.Media.MediaType)
	assert.Equal(t, dto.ReleaseStatusFinished, entry.Media.ReleaseStatus)

	// Repeat = 1 means IsRepeating is true
	assert.True(t, entry.ListEntry.IsRepeating)
	assert.NotNil(t, entry.ListEntry.RepeatCount)
	assert.Equal(t, 1, *entry.ListEntry.RepeatCount)

	// Page 2 paging: prev should be "1", next should be "3"
	assert.True(t, paginated.Paging.HasNext)
	assert.NotNil(t, paginated.Paging.NextCursor)
	assert.Equal(t, "3", *paginated.Paging.NextCursor)
}

func TestAniListMediaWithoutIdOrMediaSkipped(t *testing.T) {
	res := anilist.AniListResponse{
		Data: &anilist.AniListData{
			Page: &anilist.AniListPage{
				MediaList: []anilist.AniListMediaListEntry{
					{
						Media: nil, // missing media
					},
					{
						Media: &anilist.AniListMedia{
							ID: nil, // missing ID
						},
					},
					{
						Media: &anilist.AniListMedia{
							ID: ptr(int64(7001)),
						},
					},
				},
			},
		},
	}

	paginated := res.ToPaginatedResponse()
	if len(paginated.Data) != 1 {
		t.Fatalf("expected 1 valid entry, got %d", len(paginated.Data))
	}
	if paginated.Data[0].Media.ProviderID != "7001" {
		t.Errorf("expected provider ID 7001, got %s", paginated.Data[0].Media.ProviderID)
	}
}

func TestAniListNilDataHandled(t *testing.T) {
	res := anilist.AniListResponse{
		Data: nil,
	}
	paginated := res.ToPaginatedResponse()
	if len(paginated.Data) != 0 {
		t.Errorf("expected 0 entries for nil data, got %d", len(paginated.Data))
	}
	assert.False(t, paginated.Paging.HasNext)
}
