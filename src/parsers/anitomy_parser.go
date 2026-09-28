package parsers

import (
	"strings"

	"komorebi-server/src/dto"

	mapset "github.com/deckarep/golang-set/v3"
	"github.com/nssteinbrenner/anitogo"
)

type anitomyParser struct{}

func (p *anitomyParser) CanParse(content string) bool {
	return true
}

func (p *anitomyParser) Parse(content string) dto.ParsedTitle {
	res := anitogo.Parse(content, anitogo.DefaultOptions)

	return ToParsedTitle(res)
}

func ToParsedTitle(p *anitogo.Elements) dto.ParsedTitle {
	var parsedTitle dto.ParsedTitle

	parsedTitle.Season = ToStringSet(p.AnimeSeason)
	parsedTitle.Title = ToStringSet(p.AnimeTitle)
	parsedTitle.Kind = ToStringSet(p.AnimeType)
	parsedTitle.Year = ToStringSet(p.AnimeYear)
	parsedTitle.AudioTerm = ToStringSet(p.AudioTerm)
	parsedTitle.Device = ToStringSet(p.DeviceCompatibility)
	parsedTitle.Episode = ToStringSet(p.EpisodeNumber)
	parsedTitle.EpisodeAlt = ToStringSet(p.EpisodeNumberAlt)
	parsedTitle.EpisodeTitle = ToStringSet(p.EpisodeTitle)
	parsedTitle.FileChecksum = ToStringSet(p.FileChecksum)
	parsedTitle.FileExtension = ToStringSet(p.FileExtension)
	parsedTitle.Language = ToStringSet(p.Language)
	parsedTitle.Other = ToStringSet(p.Other)
	parsedTitle.ReleaseGroup = ToStringSet(p.ReleaseGroup)
	parsedTitle.ReleaseInformation = ToStringSet(p.ReleaseInformation)
	parsedTitle.ReleaseVersion = ToStringSet(p.ReleaseVersion)
	parsedTitle.Source = ToStringSet(p.Source)
	parsedTitle.Subtitles = ToStringSet(p.Subtitles)
	parsedTitle.VideoResolution = ToStringSet(p.VideoResolution)
	parsedTitle.VideoTerm = ToStringSet(p.VideoTerm)
	parsedTitle.Volume = ToStringSet(p.VolumeNumber)
	parsedTitle.Unknown = ToStringSet(p.Unknown)

	return parsedTitle
}

func ToStringSet(value any) mapset.Set[string] {
	x, ok := value.([]string)
	if !ok {
		y, ok := value.(string)
		if !ok || strings.TrimSpace(y) == "" {
			return nil
		}
		return mapset.NewSet(y)
	}

	if len(x) == 0 || (len(x) == 1 && strings.TrimSpace(x[0]) == "") {
		return nil
	}

	return mapset.NewSet(x...)
}
