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
	var dto dto.ParsedTitle

	dto.Season = ToStringSet(p.AnimeSeason)
	dto.Title = ToStringSet(p.AnimeTitle)
	dto.Kind = ToStringSet(p.AnimeType)
	dto.Year = ToStringSet(p.AnimeYear)
	dto.AudioTerm = ToStringSet(p.AudioTerm)
	dto.Device = ToStringSet(p.DeviceCompatibility)
	dto.Episode = ToStringSet(p.EpisodeNumber)
	dto.EpisodeAlt = ToStringSet(p.EpisodeNumberAlt)
	dto.EpisodeTitle = ToStringSet(p.EpisodeTitle)
	dto.FileChecksum = ToStringSet(p.FileChecksum)
	dto.FileExtension = ToStringSet(p.FileExtension)
	dto.Language = ToStringSet(p.Language)
	dto.Other = ToStringSet(p.Other)
	dto.ReleaseGroup = ToStringSet(p.ReleaseGroup)
	dto.ReleaseInformation = ToStringSet(p.ReleaseInformation)
	dto.ReleaseVersion = ToStringSet(p.ReleaseVersion)
	dto.Source = ToStringSet(p.Source)
	dto.Subtitles = ToStringSet(p.Subtitles)
	dto.VideoResolution = ToStringSet(p.VideoResolution)
	dto.VideoTerm = ToStringSet(p.VideoTerm)
	dto.Volume = ToStringSet(p.VolumeNumber)
	dto.Unknown = ToStringSet(p.Unknown)

	return dto
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
