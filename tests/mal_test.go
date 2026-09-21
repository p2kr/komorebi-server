package tests

import (
	"encoding/json"
	"testing"
	"uuid"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	"komorebi-server/src/adapters/mal"
	"komorebi-server/src/dto"
)

func TestMalResponseUnmarshalAndConvertAnime(t *testing.T) {
	rawJSON := `{
		"data": [
			{
				"node": {
					"id": 12345,
					"title": "Sousou no Frieren",
					"alternative_titles": {
						"en": "Frieren: Beyond Journey's End",
						"ja": "葬送のフリーレン"
					},
					"main_picture": {
						"medium": "https://example.com/medium.jpg",
						"large": "https://example.com/large.jpg"
					},
					"genres": [
						{"id": 1, "name": "Adventure"},
						{"id": 2, "name": "Fantasy"}
					],
					"synopsis": "After the party defeated the demon king...",
					"mean": 9.14,
					"popularity": 45,
					"num_episodes": 28,
					"average_episode_duration": 1440,
					"nsfw": "white",
					"media_type": "tv",
					"status": "finished_airing"
				},
				"list_status": {
					"status": "watching",
					"score": 9.5,
					"num_episodes_watched": 14,
					"is_rewatching": false,
					"num_times_rewatched": 0,
					"tags": ["favorite", "fantasy"],
					"comments": "Masterpiece",
					"updated_at": "2024-03-01T12:00:00+00:00"
				}
			}
		],
		"paging": {
			"next": "https://api.myanimelist.net/v2/users/test/animelist?offset=50",
			"previous": null
		}
	}`

	var res mal.MalResponse
	err := json.Unmarshal([]byte(rawJSON), &res)
	require.NoError(t, err)

	paginated := res.ToPaginatedResponse()

	if len(paginated.Data) != 1 {
		t.Fatalf("expected 1 entry, got %d", len(paginated.Data))
	}

	entry := paginated.Data[0]

	// Verify Media
	assert.Equal(t, uuid.Nil(), entry.Media.ID)
	assert.Equal(t, dto.MediaProviderMAL, entry.Media.Provider)
	assert.Equal(t, dto.MediaFormatTv, entry.Media.Format)
	if entry.Media.Title.Romanized == nil || *entry.Media.Title.Romanized != "Sousou no Frieren" {
		t.Errorf("expected romanized title Sousou no Frieren, got %v", entry.Media.Title.Romanized)
	}
	if entry.Media.Title.English == nil || *entry.Media.Title.English != "Frieren: Beyond Journey's End" {
		t.Errorf("expected english title Frieren: Beyond Journey's End, got %v", entry.Media.Title.English)
	}
	assert.NotNil(t, entry.Media.Title.Native)
	assert.Equal(t, "葬送のフリーレン", *entry.Media.Title.Native)
	assert.NotNil(t, entry.Media.Cover.ExtraLarge)
	assert.Equal(t, "https://example.com/large.jpg", *entry.Media.Cover.ExtraLarge)
	assert.NotNil(t, entry.Media.Popularity)
	assert.Equal(t, 45, *entry.Media.Popularity)
	assert.NotNil(t, entry.Media.Duration)
	assert.Equal(t, 1440, *entry.Media.Duration)
	assert.Equal(t, dto.NsfwLevelSafe, entry.Media.Nsfw)
	assert.NotNil(t, entry.ListEntry.Score)
	assert.Equal(t, float32(9.5), *entry.ListEntry.Score)
	assert.False(t, entry.ListEntry.IsRepeating)
	if len(entry.ListEntry.Tags) != 2 || entry.ListEntry.Tags[0] != "favorite" {
		t.Errorf("expected tags [favorite fantasy], got %v", entry.ListEntry.Tags)
	}
	assert.NotNil(t, entry.ListEntry.Notes)
	assert.Equal(t, "Masterpiece", *entry.ListEntry.Notes)

	// Verify Paging
	assert.True(t, paginated.Paging.HasNext)
	assert.NotNil(t, paginated.Paging.NextCursor)
	assert.Equal(t, "https://api.myanimelist.net/v2/users/test/animelist?offset=50", *paginated.Paging.NextCursor)
}

func TestMalResponseUnmarshalAndConvertManga(t *testing.T) {
	rawJSON := `{
		"data": [
			{
				"node": {
					"id": 99999,
					"title": "Berserk",
					"media_type": "manga",
					"status": "currently_publishing",
					"num_chapters": 376,
					"num_volumes": 42,
					"nsfw": "black",
					"my_list_status": {
						"status": "reading",
						"score": 10.0,
						"num_chapters_read": 360,
						"num_volumes_read": 41,
						"is_rereading": true,
						"num_times_reread": 3
					}
				}
			}
		],
		"paging": {
			"previous": "https://api.myanimelist.net/v2/users/test/mangalist?offset=0"
		}
	}`

	var res mal.MalResponse
	err := json.Unmarshal([]byte(rawJSON), &res)
	require.NoError(t, err)

	paginated := res.ToPaginatedResponse()

	if len(paginated.Data) != 1 {
		t.Fatalf("expected 1 entry, got %d", len(paginated.Data))
	}

	entry := paginated.Data[0]

	assert.Equal(t, dto.MediaTypeManga, entry.Media.MediaType)
	assert.Equal(t, dto.ReleaseStatusReleasing, entry.Media.ReleaseStatus)
	if entry.Media.Volumes == nil || *entry.Media.Volumes != 42 {
		t.Errorf("expected volumes 42, got %v", entry.Media.Volumes)
	}
	assert.Equal(t, dto.NsfwLevelNsfw, entry.Media.Nsfw)
	assert.True(t, entry.ListEntry.IsRepeating)
	assert.NotNil(t, entry.ListEntry.RepeatCount)
	assert.Equal(t, 3, *entry.ListEntry.RepeatCount)
	assert.NotNil(t, entry.ListEntry.ProgressVolumes)
	assert.Equal(t, 41, *entry.ListEntry.ProgressVolumes)
	if paginated.Paging.PrevCursor == nil || *paginated.Paging.PrevCursor != "https://api.myanimelist.net/v2/users/test/mangalist?offset=0" {
		t.Errorf("expected prev cursor URL, got %v", paginated.Paging.PrevCursor)
	}
}

func TestMalItemWithoutTitleOrIdSkipped(t *testing.T) {
	res := mal.MalResponse{
		Data: []mal.MalItem{
			{
				Node: mal.MalNode{
					ID: nil, // missing ID
				},
			},
			{
				Node: mal.MalNode{
					ID:    new(int64(1)),
					Title: nil, // missing Title
				},
			},
			{
				Node: mal.MalNode{
					ID:    new(int64(2)),
					Title: new("Valid Title"),
				},
			},
		},
	}

	paginated := res.ToPaginatedResponse()
	if len(paginated.Data) != 1 {
		t.Fatalf("expected 1 valid entry, got %d", len(paginated.Data))
	}
	if paginated.Data[0].Media.ProviderID != "2" {
		t.Errorf("expected provider ID 2, got %s", paginated.Data[0].Media.ProviderID)
	}
}

func TestParseMalStatus(t *testing.T) {
	tests := []struct {
		status   string
		isManga  bool
		expected mal.MalStatus
		hasError bool
	}{
		{"current", false, mal.MalStatusWatching, false},
		{"current", true, mal.MalStatusReading, false},
		{"watching", false, mal.MalStatusWatching, false},
		{"reading", true, mal.MalStatusReading, false},
		{"planning", false, mal.MalStatusPlanToWatch, false},
		{"planning", true, mal.MalStatusPlanToRead, false},
		{"plan_to_watch", false, mal.MalStatusPlanToWatch, false},
		{"plan_to_read", true, mal.MalStatusPlanToRead, false},
		{"on_hold", false, mal.MalStatusOnHold, false},
		{"paused", false, mal.MalStatusOnHold, false},
		{"completed", false, mal.MalStatusCompleted, false},
		{"dropped", false, mal.MalStatusDropped, false},
		{"invalid", false, "", true},
	}

	for _, tt := range tests {
		got, err := mal.ParseMalStatus(tt.status, tt.isManga)
		if tt.hasError && err == nil {
			t.Errorf("expected error for %s (isManga: %v)", tt.status, tt.isManga)
		}
		if !tt.hasError && (err != nil || got != tt.expected) {
			t.Errorf("status %s (isManga: %v): expected %s, got %s (err: %v)", tt.status, tt.isManga, tt.expected, got, err)
		}
	}
}
