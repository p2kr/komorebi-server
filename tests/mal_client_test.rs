use komorebi_server::adapters::mal_models::{MalResponse, MalStatus};
use komorebi_server::models::media::{ListStatus, PaginatedResponse};
use serde_json::json;

#[test]
fn test_parse_response_json() {
    let sample = json!({
        "data": [
            {
                "node": {
                    "id": 30276,
                    "title": "One Punch Man",
                    "main_picture": {
                        "medium": "https://cdn.myanimelist.net/images/anime/12/76049.jpg",
                        "large": "https://cdn.myanimelist.net/images/anime/12/76049l.jpg"
                    },
                    "alternative_titles": {
                        "synonyms": ["OPM"],
                        "en": "One-Punch Man",
                        "ja": "ワンパンマン"
                    },
                    "synopsis": "The story of Saitama...",
                    "mean": 8.5,
                    "rank": 100,
                    "popularity": 5,
                    "nsfw": "white",
                    "genres": [
                        { "id": 1, "name": "Action" },
                        { "id": 4, "name": "Comedy" }
                    ],
                    "num_episodes": 12,
                    "average_episode_duration": 1440
                },
                "list_status": {
                    "status": "completed",
                    "score": 9,
                    "num_episodes_watched": 12,
                    "is_rewatching": false,
                    "tags": ["favorite"],
                    "comments": "Great animation"
                }
            }
        ],
        "paging": {
            "next": "https://api.myanimelist.net/v2/users/test/animelist?offset=10"
        }
    });

    let parsed: PaginatedResponse = serde_json::from_value::<MalResponse>(sample)
        .expect("failed to parse")
        .into();

    assert_eq!(parsed.data.len(), 1);
    let entry = &parsed.data[0];
    assert_eq!(entry.media.provider_id, "30276");
    assert_eq!(
        entry.media.title.romanized.as_deref(),
        Some("One Punch Man")
    );
    assert_eq!(entry.media.title.english.as_deref(), Some("One-Punch Man"));
    assert_eq!(entry.media.title.native.as_deref(), Some("ワンパンマン"));
    assert_eq!(entry.list_entry.status, ListStatus::Completed);
    assert_eq!(entry.list_entry.score, Some(9.0));
    assert_eq!(entry.media.genres, vec!["Action", "Comedy"]);
    assert!(parsed.paging.has_next);
    assert_eq!(
        parsed.paging.next_cursor.as_deref(),
        Some("https://api.myanimelist.net/v2/users/test/animelist?offset=10")
    );
}

#[test]
fn test_parse_mal_manga_response_json() {
    let sample = json!({
        "data": [
            {
                "node": {
                    "id": 13,
                    "title": "One Piece",
                    "main_picture": {
                        "medium": "https://cdn.myanimelist.net/images/manga/2/253146.jpg",
                        "large": "https://cdn.myanimelist.net/images/manga/2/253146l.jpg"
                    },
                    "alternative_titles": {
                        "synonyms": [],
                        "en": "One Piece",
                        "ja": "ONE PIECE"
                    },
                    "synopsis": "Gol D. Roger was known as the Pirate King...",
                    "mean": 9.25,
                    "popularity": 1,
                    "media_type": "manga",
                    "num_chapters": 1100,
                    "num_volumes": 108,
                    "genres": [
                        { "id": 1, "name": "Action" },
                        { "id": 2, "name": "Adventure" }
                    ]
                },
                "list_status": {
                    "status": "reading",
                    "score": 10,
                    "num_chapters_read": 1050,
                    "num_volumes_read": 100,
                    "is_rereading": false
                }
            }
        ]
    });

    let parsed: PaginatedResponse = serde_json::from_value::<MalResponse>(sample)
        .expect("failed to parse mal manga response")
        .into();

    assert_eq!(parsed.data.len(), 1);
    let entry = &parsed.data[0];
    assert_eq!(entry.media.provider_id, "13");
    assert_eq!(
        entry.media.media_type,
        komorebi_server::models::media::MediaType::Manga
    );
    assert_eq!(
        entry.media.format,
        komorebi_server::models::media::MediaFormat::Manga
    );
    assert_eq!(entry.media.chapters, Some(1100));
    assert_eq!(entry.media.volumes, Some(108));
    assert_eq!(entry.list_entry.status, ListStatus::Current);
    assert_eq!(entry.list_entry.progress, Some(1050));
    assert_eq!(entry.list_entry.progress_volumes, Some(100));
}

#[test]
fn test_mal_status_conversion() {
    assert_eq!(
        MalStatus::try_from(("watching", false)),
        Ok(MalStatus::Watching)
    );
    assert_eq!(
        MalStatus::try_from(("current", true)),
        Ok(MalStatus::Reading)
    );
    assert_eq!(
        MalStatus::try_from(("planning", false)),
        Ok(MalStatus::PlanToWatch)
    );
    assert_eq!(
        MalStatus::try_from(("planning", true)),
        Ok(MalStatus::PlanToRead)
    );
    assert_eq!(
        MalStatus::try_from(("paused", false)),
        Ok(MalStatus::OnHold)
    );
    assert_eq!(
        MalStatus::try_from(("completed", false)),
        Ok(MalStatus::Completed)
    );
    assert_eq!(
        MalStatus::try_from(("dropped", false)),
        Ok(MalStatus::Dropped)
    );
    assert!(MalStatus::try_from(("invalid_status", false)).is_err());

    // Serde serialization test
    assert_eq!(
        serde_json::to_string(&MalStatus::PlanToWatch).unwrap(),
        "\"plan_to_watch\""
    );
    assert_eq!(
        serde_json::to_string(&MalStatus::Reading).unwrap(),
        "\"reading\""
    );
}
