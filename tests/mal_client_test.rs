use komorebi_server::handlers::clients::mal_models::MalResponse;
use komorebi_server::models::media::PaginatedResponse;
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
    let item = &parsed.data[0];
    assert_eq!(item.provider_id.as_deref(), Some("30276"));
    assert_eq!(item.title.romanized, "One Punch Man");
    assert_eq!(item.title.english, "One-Punch Man");
    assert_eq!(item.title.native, "ワンパンマン");
    assert_eq!(item.status.status.as_deref(), Some("completed"));
    assert_eq!(item.status.score, Some(9.0));
    assert_eq!(item.genres, vec!["Action", "Comedy"]);
    assert_eq!(parsed.paging_info.has_next, true);
    assert_eq!(
        parsed.paging_info.next.as_deref(),
        Some("https://api.myanimelist.net/v2/users/test/animelist?offset=10")
    );
}
