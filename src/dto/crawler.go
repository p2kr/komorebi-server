package dto

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
	AudioTerm          Set[string] `json:"audio_term,omitempty"`
	Device             Set[string] `json:"device,omitempty"`
	Episode            Set[string] `json:"episode,omitempty"`
	EpisodeTitle       Set[string] `json:"episode_title,omitempty"`
	FileChecksum       Set[string] `json:"file_checksum,omitempty"`
	FileExtension      Set[string] `json:"file_extension,omitempty"`
	Language           Set[string] `json:"language,omitempty"`
	Other              Set[string] `json:"other,omitempty"`
	Part               Set[string] `json:"part,omitempty"`
	ReleaseGroup       Set[string] `json:"release_group,omitempty"`
	ReleaseInformation Set[string] `json:"release_information,omitempty"`
	ReleaseVersion     Set[string] `json:"release_version,omitempty"`
	Season             Set[string] `json:"season,omitempty"`
	Source             Set[string] `json:"source,omitempty"`
	Subtitles          Set[string] `json:"subtitles,omitempty"`
	Title              Set[string] `json:"title,omitempty"`
	VideoResolution    Set[string] `json:"video_resolution,omitempty"`
	VideoTerm          Set[string] `json:"video_term,omitempty"`
	Volume             Set[string] `json:"volume,omitempty"`
	Year               Set[string] `json:"year,omitempty"`
	EpisodeAlt         Set[string] `json:"episode_alt,omitempty"`
	Date               Set[string] `json:"date,omitempty"`
}
