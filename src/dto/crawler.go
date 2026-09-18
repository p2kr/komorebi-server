package dto

import mapset "github.com/deckarep/golang-set/v3"

type CrawlerResult struct {
	Title      string    `json:"title"`
	Link       string    `json:"link"`
	Source     string    `json:"source"`
	Popularity *string   `json:"popularity"`
	Size       *string   `json:"size"`
	Category   MediaType `json:"category"`

	ParsedTitle *ParsedTitle `json:"parsed_title"`
}

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
