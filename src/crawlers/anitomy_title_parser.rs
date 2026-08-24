use anitomy::{Element, ElementKind};

use crate::{crawlers::TitleParser, models::crawler::ParsedTitle};

pub struct AnitomyTitleParser;

impl TitleParser for AnitomyTitleParser {
    fn can_parse(_raw_title: &str) -> bool {
        // TODO: check if exe is present
        true
    }

    fn parse(raw_title: &str) -> ParsedTitle {
        let mut parsed_title: ParsedTitle = anitomy::parse(raw_title).into();

        if parsed_title.title.is_empty() {
            parsed_title.title.push(raw_title.into());
        }

        parsed_title
    }
}

impl<'a> From<Vec<Element<'a>>> for ParsedTitle {
    fn from(elements: Vec<Element<'a>>) -> Self {
        let mut title = ParsedTitle::default();

        for el in elements {
            match el.kind() {
                ElementKind::AudioTerm => title.audio_term.push(el.value().into()),
                ElementKind::DeviceCompatibility => title.device.push(el.value().into()),
                ElementKind::Episode => title.episode.push(el.value().into()),
                ElementKind::EpisodeTitle => title.episode_title.push(el.value().into()),
                ElementKind::EpisodeAlt => title.episode_alt.push(el.value().into()),
                ElementKind::FileChecksum => title.file_checksum.push(el.value().into()),
                ElementKind::FileExtension => title.file_extension.push(el.value().into()),
                ElementKind::Language => title.language.push(el.value().into()),
                ElementKind::Other => title.other.push(el.value().into()),
                ElementKind::ReleaseGroup => title.release_group.push(el.value().into()),
                ElementKind::ReleaseInformation => {
                    title.release_information.push(el.value().into())
                }
                ElementKind::ReleaseVersion => title.release_version.push(el.value().into()),
                ElementKind::Season => title.season.push(el.value().into()),
                ElementKind::Source => title.source.push(el.value().into()),
                ElementKind::Subtitles => title.subtitles.push(el.value().into()),
                ElementKind::Title => title.title.push(el.value().into()),
                ElementKind::Type => title.kind.push(el.value().into()),
                ElementKind::VideoResolution => title.video_resolution.push(el.value().into()),
                ElementKind::VideoTerm => title.video_term.push(el.value().into()),
                ElementKind::Volume => title.volume.push(el.value().into()),
                ElementKind::Year => title.year.push(el.value().into()),
                ElementKind::Date => title.date.push(el.value().into()),
            }
        }

        title
    }
}
