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
            parsed_title.title.insert(raw_title.into());
        }

        parsed_title
    }
}

impl<'a> From<Vec<Element<'a>>> for ParsedTitle {
    fn from(elements: Vec<Element<'a>>) -> Self {
        let mut title = ParsedTitle::default();

        for el in elements {
            match el.kind() {
                ElementKind::AudioTerm => title.audio_term.insert(el.value().into()),
                ElementKind::DeviceCompatibility => title.device.insert(el.value().into()),
                ElementKind::Episode => title.episode.insert(el.value().into()),
                ElementKind::EpisodeTitle => title.episode_title.insert(el.value().into()),
                ElementKind::EpisodeAlt => title.episode_alt.insert(el.value().into()),
                ElementKind::FileChecksum => title.file_checksum.insert(el.value().into()),
                ElementKind::FileExtension => title.file_extension.insert(el.value().into()),
                ElementKind::Language => title.language.insert(el.value().into()),
                ElementKind::Other => title.other.insert(el.value().into()),
                ElementKind::ReleaseGroup => title.release_group.insert(el.value().into()),
                ElementKind::ReleaseInformation => {
                    title.release_information.insert(el.value().into())
                }
                ElementKind::ReleaseVersion => title.release_version.insert(el.value().into()),
                ElementKind::Season => title.season.insert(el.value().into()),
                ElementKind::Source => title.source.insert(el.value().into()),
                ElementKind::Subtitles => title.subtitles.insert(el.value().into()),
                ElementKind::Title => title.title.insert(el.value().into()),
                ElementKind::Type => title.kind.insert(el.value().into()),
                ElementKind::VideoResolution => title.video_resolution.insert(el.value().into()),
                ElementKind::VideoTerm => title.video_term.insert(el.value().into()),
                ElementKind::Volume => title.volume.insert(el.value().into()),
                ElementKind::Year => title.year.insert(el.value().into()),
                ElementKind::Date => title.date.insert(el.value().into()),
            };
        }

        title
    }
}
