use komorebi_server::handlers::clients::anilist_models::AnilistResponse;
use komorebi_server::models::media::{
    ListStatus, MediaFormat, MediaProvider, MediaType, NsfwLevel, PaginatedResponse, ReleaseStatus,
};
use serde_json::json;

#[test]
fn test_parse_anilist_response_json() {
    let sample = json!({
        "data": {
            "Page": {
                "pageInfo": {
                    "total": 1,
                    "perPage": 50,
                    "currentPage": 1,
                    "lastPage": 1,
                    "hasNextPage": false
                },
                "mediaList": [
                    {
                        "id": 123456,
                        "status": "COMPLETED",
                        "score": 8.5,
                        "progress": 24,
                        "progressVolumes": null,
                        "repeat": 1,
                        "notes": "Must watch again!",
                        "updatedAt": 1600000000,
                        "media": {
                            "id": 9999,
                            "idMal": 5555,
                            "type": "ANIME",
                            "format": "TV",
                            "status": "FINISHED",
                            "title": {
                                "romaji": "Shingeki no Kyojin",
                                "english": "Attack on Titan",
                                "native": "進撃の巨人",
                                "userPreferred": "Attack on Titan"
                            },
                            "coverImage": {
                                "extraLarge": "https://s4.anilist.co/file/anilistcdn/media/anime/cover/large/bx9999.jpg",
                                "large": "https://s4.anilist.co/file/anilistcdn/media/anime/cover/medium/bx9999.jpg",
                                "medium": "https://s4.anilist.co/file/anilistcdn/media/anime/cover/small/bx9999.jpg",
                                "color": "#e44320"
                            },
                            "description": "Humanity turned to walls...",
                            "meanScore": 86,
                            "popularity": 500000,
                            "episodes": 25,
                            "duration": 24,
                            "chapters": null,
                            "volumes": null,
                            "genres": ["Action", "Drama", "Fantasy"],
                            "isAdult": false
                        }
                    }
                ]
            }
        }
    });

    let parsed: PaginatedResponse = serde_json::from_value::<AnilistResponse>(sample)
        .expect("failed to parse anilist response")
        .into();

    assert_eq!(parsed.data.len(), 1);
    let entry = &parsed.data[0];

    assert_eq!(entry.media.provider_id, "9999");
    assert_eq!(entry.media.provider, MediaProvider::Anilist);
    assert_eq!(entry.media.media_type, MediaType::Anime);
    assert_eq!(entry.media.format, MediaFormat::Tv);
    assert_eq!(entry.media.release_status, ReleaseStatus::Finished);
    assert_eq!(
        entry.media.title.romanized.as_deref(),
        Some("Shingeki no Kyojin")
    );
    assert_eq!(
        entry.media.title.english.as_deref(),
        Some("Attack on Titan")
    );
    assert_eq!(entry.media.title.native.as_deref(), Some("進撃の巨人"));
    assert_eq!(
        entry.media.title.user_preferred.as_deref(),
        Some("Attack on Titan")
    );
    assert_eq!(
        entry.media.cover.extra_large.as_deref(),
        Some("https://s4.anilist.co/file/anilistcdn/media/anime/cover/large/bx9999.jpg")
    );
    assert_eq!(entry.media.cover.color.as_deref(), Some("#e44320"));
    assert_eq!(entry.media.mean_score, Some(8.6)); // 86 / 10 = 8.6
    assert_eq!(entry.media.duration, Some(1440)); // 24 mins * 60 = 1440 secs
    assert_eq!(entry.media.genres, vec!["Action", "Drama", "Fantasy"]);
    assert_eq!(entry.media.nsfw, NsfwLevel::Safe);

    assert_eq!(entry.list_entry.status, ListStatus::Completed);
    assert_eq!(entry.list_entry.score, Some(8.5));
    assert_eq!(entry.list_entry.progress, Some(24));
    assert_eq!(entry.list_entry.is_repeating, true);
    assert_eq!(entry.list_entry.repeat_count, Some(1));
    assert_eq!(entry.list_entry.notes.as_deref(), Some("Must watch again!"));

    assert_eq!(parsed.paging.has_next, false);
}

#[test]
fn test_parse_anilist_manga_response_json() {
    let sample = json!({
        "data": {
            "Page": {
                "pageInfo": {
                    "total": 1,
                    "perPage": 50,
                    "currentPage": 1,
                    "lastPage": 1,
                    "hasNextPage": false
                },
                "mediaList": [
                    {
                        "id": 654321,
                        "status": "CURRENT",
                        "score": 9.0,
                        "progress": 150,
                        "progressVolumes": 15,
                        "repeat": 0,
                        "notes": "Reading weekly",
                        "updatedAt": 1600000000,
                        "media": {
                            "id": 30013,
                            "idMal": 13,
                            "type": "MANGA",
                            "format": "MANGA",
                            "status": "RELEASING",
                            "title": {
                                "romaji": "One Piece",
                                "english": "One Piece",
                                "native": "ONE PIECE",
                                "userPreferred": "One Piece"
                            },
                            "coverImage": {
                                "extraLarge": "https://s4.anilist.co/file/anilistcdn/media/manga/cover/large/bx30013.jpg",
                                "large": "https://s4.anilist.co/file/anilistcdn/media/manga/cover/medium/bx30013.jpg",
                                "medium": "https://s4.anilist.co/file/anilistcdn/media/manga/cover/small/bx30013.jpg",
                                "color": "#000000"
                            },
                            "description": "Pirate adventures...",
                            "meanScore": 92,
                            "popularity": 800000,
                            "episodes": null,
                            "duration": null,
                            "chapters": 1100,
                            "volumes": 108,
                            "genres": ["Action", "Adventure"],
                            "isAdult": false
                        }
                    }
                ]
            }
        }
    });

    let parsed: PaginatedResponse = serde_json::from_value::<AnilistResponse>(sample)
        .expect("failed to parse anilist manga response")
        .into();

    assert_eq!(parsed.data.len(), 1);
    let entry = &parsed.data[0];

    assert_eq!(entry.media.provider_id, "30013");
    assert_eq!(entry.media.provider, MediaProvider::Anilist);
    assert_eq!(entry.media.media_type, MediaType::Manga);
    assert_eq!(entry.media.format, MediaFormat::Manga);
    assert_eq!(entry.media.release_status, ReleaseStatus::Releasing);
    assert_eq!(entry.media.chapters, Some(1100));
    assert_eq!(entry.media.volumes, Some(108));
    assert_eq!(entry.list_entry.status, ListStatus::Current);
    assert_eq!(entry.list_entry.progress, Some(150));
    assert_eq!(entry.list_entry.progress_volumes, Some(15));
}
