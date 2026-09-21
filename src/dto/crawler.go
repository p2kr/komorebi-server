package dto

import (
	"encoding/json"
	"net/url"
	"reflect"
	"strings"

	"github.com/Oudwins/zog"
	mapset "github.com/deckarep/golang-set/v3"
)

type CrawlerResult struct {
	Title      string    `json:"title"`
	Link       string    `json:"link"`
	Source     string    `json:"source"`
	Popularity *string   `json:"popularity"`
	Size       *string   `json:"size"`
	Category   MediaType `json:"category"`

	ParsedTitle *ParsedTitle `json:"parsed_title"`
}

var ICrawlerResult = zog.Struct(zog.Shape{
	"Title": zog.String().Min(1),
	"Link": zog.String().TestFunc(func(v *string, ctx zog.Ctx) bool {
		u, err := url.Parse(*v)
		return err == nil && u.Scheme != ""
	}),
})

type ParsedTitle struct {
	AudioTerm          mapset.Set[string] `json:"audio_term,omitempty"`
	Device             mapset.Set[string] `json:"device,omitempty"`
	Episode            mapset.Set[string] `json:"episode,omitempty"`
	EpisodeTitle       mapset.Set[string] `json:"episode_title,omitempty"`
	FileChecksum       mapset.Set[string] `json:"file_checksum,omitempty"`
	FileExtension      mapset.Set[string] `json:"file_extension,omitempty"`
	Language           mapset.Set[string] `json:"language,omitempty"`
	Other              mapset.Set[string] `json:"other,omitempty"`
	Part               mapset.Set[string] `json:"part,omitempty"`
	ReleaseGroup       mapset.Set[string] `json:"release_group,omitempty"`
	ReleaseInformation mapset.Set[string] `json:"release_information,omitempty"`
	ReleaseVersion     mapset.Set[string] `json:"release_version,omitempty"`
	Season             mapset.Set[string] `json:"season,omitempty"`
	Source             mapset.Set[string] `json:"source,omitempty"`
	Subtitles          mapset.Set[string] `json:"subtitles,omitempty"`
	Title              mapset.Set[string] `json:"title,omitempty"`
	VideoResolution    mapset.Set[string] `json:"video_resolution,omitempty"`
	VideoTerm          mapset.Set[string] `json:"video_term,omitempty"`
	Volume             mapset.Set[string] `json:"volume,omitempty"`
	Year               mapset.Set[string] `json:"year,omitempty"`
	EpisodeAlt         mapset.Set[string] `json:"episode_alt,omitempty"`
	Date               mapset.Set[string] `json:"date,omitempty"`
	Kind               mapset.Set[string] `json:"kind,omitempty"`
	Unknown            mapset.Set[string] `json:"unknown,omitempty"`
}

func (p *ParsedTitle) UnmarshalJSON(data []byte) error {
	var aux map[string][]string
	if err := json.Unmarshal(data, &aux); err != nil {
		return err
	}

	val := reflect.ValueOf(p).Elem()
	typ := val.Type()

	for i := 0; i < typ.NumField(); i++ {
		field := typ.Field(i)
		tag := field.Tag.Get("json")
		jsonKey, _, _ := strings.Cut(tag, ",")

		if jsonKey == "" {
			jsonKey = field.Name
		}

		if slice, ok := aux[jsonKey]; ok && slice != nil {
			val.Field(i).Set(reflect.ValueOf(mapset.NewSet(slice...)))
		}
	}
	return nil
}
