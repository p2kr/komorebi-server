use komorebi_server::adapters::anilist_models::{
    AniListCoverImage, AniListData, AniListGraphqlError, AniListMedia, AniListMediaListEntry,
    AniListPage, AniListPageInfo, AniListResponse, AniListTitle,
};
use komorebi_server::models::media::{
    ListEntry, ListStatus, MediaEntry, MediaFormat, MediaProvider, MediaType, NsfwLevel,
    PaginatedResponse, ReleaseStatus,
};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn make_full_media(id: i64, media_type: &str, format: &str, status: &str) -> AniListMedia {
    AniListMedia {
        id: Some(id),
        id_mal: Some(id + 1000),
        media_type: Some(media_type.into()),
        format: Some(format.into()),
        status: Some(status.into()),
        title: Some(AniListTitle {
            romaji: Some("Romaji Title".into()),
            english: Some("English Title".into()),
            native: Some("Native Title".into()),
            user_preferred: Some("Preferred Title".into()),
        }),
        cover_image: Some(AniListCoverImage {
            extra_large: Some("xl.jpg".into()),
            large: Some("l.jpg".into()),
            medium: Some("m.jpg".into()),
            color: Some("#ff0000".into()),
        }),
        description: Some("A synopsis".into()),
        mean_score: Some(80.0),
        popularity: Some(5000),
        episodes: Some(12),
        duration: Some(24),
        chapters: Some(100),
        volumes: Some(10),
        genres: Some(vec!["Action".into(), "Drama".into()]),
        is_adult: Some(false),
    }
}

fn make_entry(status: &str, media: Option<AniListMedia>) -> AniListMediaListEntry {
    AniListMediaListEntry {
        id: Some(1),
        status: Some(status.into()),
        score: Some(7.5),
        progress: Some(6),
        progress_volumes: Some(2),
        repeat: Some(0),
        notes: Some("Nice".into()),
        updated_at: Some(1_700_000_000),
        media,
    }
}

// ─── ListEntry conversion ─────────────────────────────────────────────────────

#[test]
fn list_entry_current_status() {
    let entry = make_entry("CURRENT", None);
    let le = ListEntry::from(entry);
    assert_eq!(le.status, ListStatus::Current);
    assert!(!le.is_repeating);
}

#[test]
fn list_entry_planning_status() {
    let entry = make_entry("PLANNING", None);
    let le = ListEntry::from(entry);
    assert_eq!(le.status, ListStatus::Planning);
}

#[test]
fn list_entry_completed_status() {
    let entry = make_entry("COMPLETED", None);
    let le = ListEntry::from(entry);
    assert_eq!(le.status, ListStatus::Completed);
}

#[test]
fn list_entry_dropped_status() {
    let entry = make_entry("DROPPED", None);
    let le = ListEntry::from(entry);
    assert_eq!(le.status, ListStatus::Dropped);
}

#[test]
fn list_entry_paused_status() {
    let entry = make_entry("PAUSED", None);
    let le = ListEntry::from(entry);
    assert_eq!(le.status, ListStatus::Paused);
}

#[test]
fn list_entry_repeating_status() {
    let entry = make_entry("REPEATING", None);
    let le = ListEntry::from(entry);
    assert_eq!(le.status, ListStatus::Repeating);
    assert!(le.is_repeating);
}

#[test]
fn list_entry_unknown_status_falls_back_to_current() {
    let entry = make_entry("GARBAGE", None);
    let le = ListEntry::from(entry);
    assert_eq!(le.status, ListStatus::Current);
}

#[test]
fn list_entry_repeat_count_sets_is_repeating() {
    let mut entry = make_entry("CURRENT", None);
    entry.repeat = Some(3);
    let le = ListEntry::from(entry);
    assert!(le.is_repeating);
    assert_eq!(le.repeat_count, Some(3));
}

#[test]
fn list_entry_score_and_progress_mapped() {
    let entry = make_entry("CURRENT", None);
    let le = ListEntry::from(entry);
    assert_eq!(le.score, Some(7.5_f32));
    assert_eq!(le.progress, Some(6));
    assert_eq!(le.progress_volumes, Some(2));
    assert_eq!(le.notes, Some("Nice".into()));
    assert_eq!(le.updated_at, Some("1700000000".into()));
}

// ─── MediaEntry conversion ────────────────────────────────────────────────────

#[test]
fn media_entry_missing_media_node_is_err() {
    let entry = make_entry("CURRENT", None);
    let result: Result<MediaEntry, ()> = entry.try_into();
    assert!(result.is_err());
}

#[test]
fn media_entry_missing_media_id_is_err() {
    let mut media = make_full_media(1, "ANIME", "TV", "RELEASING");
    media.id = None;
    let entry = make_entry("CURRENT", Some(media));
    let result: Result<MediaEntry, ()> = entry.try_into();
    assert!(result.is_err());
}

#[test]
fn media_entry_anime_full_conversion() {
    let media = make_full_media(42, "ANIME", "TV", "RELEASING");
    let entry = make_entry("CURRENT", Some(media));
    let me: MediaEntry = entry.try_into().expect("should succeed");
    assert_eq!(me.media.provider_id, "42");
    assert_eq!(me.media.provider, MediaProvider::ANILIST);
    assert_eq!(me.media.media_type, MediaType::Anime);
    assert_eq!(me.media.format, MediaFormat::Tv);
    assert_eq!(me.media.release_status, ReleaseStatus::Releasing);
    assert_eq!(me.media.title.romanized, Some("Romaji Title".into()));
    assert_eq!(me.media.title.english, Some("English Title".into()));
    assert_eq!(me.media.title.native, Some("Native Title".into()));
    assert_eq!(me.media.cover.extra_large, Some("xl.jpg".into()));
    assert_eq!(me.media.cover.color, Some("#ff0000".into()));
    assert_eq!(me.media.synopsis, Some("A synopsis".into()));
    assert_eq!(me.media.episodes, Some(12));
    assert_eq!(me.media.duration, Some(24 * 60)); // converted to seconds
    assert_eq!(me.media.chapters, Some(100));
    assert_eq!(me.media.genres, vec!["Action", "Drama"]);
    assert_eq!(me.media.nsfw, NsfwLevel::Safe);
    // mean_score: 80.0 / 10 = 8.0
    assert!((me.media.mean_score.unwrap() - 8.0).abs() < 0.01);
}

#[test]
fn media_entry_nsfw_adult_flag() {
    let mut media = make_full_media(1, "ANIME", "TV", "FINISHED");
    media.is_adult = Some(true);
    let entry = make_entry("COMPLETED", Some(media));
    let me: MediaEntry = entry.try_into().unwrap();
    assert_eq!(me.media.nsfw, NsfwLevel::Nsfw);
}

#[test]
fn media_entry_no_title_uses_default() {
    let mut media = make_full_media(5, "ANIME", "TV", "FINISHED");
    media.title = None;
    let entry = make_entry("COMPLETED", Some(media));
    let me: MediaEntry = entry.try_into().unwrap();
    assert!(me.media.title.romanized.is_none());
}

#[test]
fn media_entry_no_cover_uses_default() {
    let mut media = make_full_media(5, "ANIME", "TV", "FINISHED");
    media.cover_image = None;
    let entry = make_entry("COMPLETED", Some(media));
    let me: MediaEntry = entry.try_into().unwrap();
    assert!(me.media.cover.extra_large.is_none());
}

// ─── Format parsing (all arms) ────────────────────────────────────────────────

fn format_for(fmt_str: &str) -> MediaFormat {
    let media = make_full_media(1, "ANIME", fmt_str, "FINISHED");
    let entry = make_entry("COMPLETED", Some(media));
    let me: MediaEntry = entry.try_into().unwrap();
    me.media.format
}

#[test]
fn parse_format_all_arms() {
    assert_eq!(format_for("TV"), MediaFormat::Tv);
    assert_eq!(format_for("TV_SHORT"), MediaFormat::TvShort);
    assert_eq!(format_for("MOVIE"), MediaFormat::Movie);
    assert_eq!(format_for("SPECIAL"), MediaFormat::Special);
    assert_eq!(format_for("OVA"), MediaFormat::Ova);
    assert_eq!(format_for("ONA"), MediaFormat::Ona);
    assert_eq!(format_for("MUSIC"), MediaFormat::Music);
    assert_eq!(format_for("MANGA"), MediaFormat::Manga);
    assert_eq!(format_for("NOVEL"), MediaFormat::Novel);
    assert_eq!(format_for("ONE_SHOT"), MediaFormat::OneShot);
    assert_eq!(format_for("UNKNOWN_FORMAT"), MediaFormat::Unknown);
}

// ─── Release status parsing ────────────────────────────────────────────────────

fn release_status_for(status_str: &str) -> ReleaseStatus {
    let media = make_full_media(1, "ANIME", "TV", status_str);
    let entry = make_entry("COMPLETED", Some(media));
    let me: MediaEntry = entry.try_into().unwrap();
    me.media.release_status
}

#[test]
fn parse_release_status_all_arms() {
    assert_eq!(release_status_for("RELEASING"), ReleaseStatus::Releasing);
    assert_eq!(release_status_for("FINISHED"), ReleaseStatus::Finished);
    assert_eq!(
        release_status_for("NOT_YET_RELEASED"),
        ReleaseStatus::NotYetReleased
    );
    assert_eq!(release_status_for("CANCELLED"), ReleaseStatus::Cancelled);
    assert_eq!(release_status_for("HIATUS"), ReleaseStatus::Hiatus);
    assert_eq!(release_status_for("UNKNOWN_STATUS"), ReleaseStatus::Unknown);
}

// ─── Media type parsing ───────────────────────────────────────────────────────

#[test]
fn parse_media_type_anime() {
    let media = make_full_media(1, "ANIME", "TV", "FINISHED");
    let entry = make_entry("COMPLETED", Some(media));
    let me: MediaEntry = entry.try_into().unwrap();
    assert_eq!(me.media.media_type, MediaType::Anime);
}

#[test]
fn parse_media_type_manga() {
    let media = make_full_media(1, "MANGA", "MANGA", "FINISHED");
    let entry = make_entry("COMPLETED", Some(media));
    let me: MediaEntry = entry.try_into().unwrap();
    assert_eq!(me.media.media_type, MediaType::Manga);
}

#[test]
fn parse_media_type_unknown_falls_back_to_anime() {
    let mut media = make_full_media(1, "WEIRD", "TV", "FINISHED");
    // clear episodes/chapters so no secondary inference
    media.episodes = None;
    media.chapters = None;
    let entry = make_entry("COMPLETED", Some(media));
    let me: MediaEntry = entry.try_into().unwrap();
    assert_eq!(me.media.media_type, MediaType::Anime);
}

// ─── PaginatedResponse conversion ────────────────────────────────────────────

fn make_page_info(current: i32, has_next: bool) -> AniListPageInfo {
    AniListPageInfo {
        total: Some(100),
        per_page: Some(10),
        current_page: Some(current),
        last_page: Some(10),
        has_next_page: Some(has_next),
    }
}

fn make_anilist_response(
    entries: Vec<AniListMediaListEntry>,
    page_info: Option<AniListPageInfo>,
) -> AniListResponse {
    AniListResponse {
        data: Some(AniListData {
            page: Some(AniListPage {
                page_info,
                media_list: Some(entries),
            }),
        }),
        errors: None,
    }
}

#[test]
fn paginated_response_has_next_page() {
    let media = make_full_media(1, "ANIME", "TV", "FINISHED");
    let entry = make_entry("COMPLETED", Some(media));
    let res = make_anilist_response(vec![entry], Some(make_page_info(1, true)));
    let paginated: PaginatedResponse = res.into();
    assert!(paginated.paging.has_next);
    assert_eq!(paginated.paging.next_cursor, Some("2".into()));
    assert!(paginated.paging.prev_cursor.is_none()); // page 1, no prev
    assert_eq!(paginated.data.len(), 1);
}

#[test]
fn paginated_response_has_prev_page() {
    let media = make_full_media(2, "ANIME", "TV", "FINISHED");
    let entry = make_entry("COMPLETED", Some(media));
    let res = make_anilist_response(vec![entry], Some(make_page_info(3, false)));
    let paginated: PaginatedResponse = res.into();
    assert!(!paginated.paging.has_next);
    assert!(paginated.paging.next_cursor.is_none());
    assert_eq!(paginated.paging.prev_cursor, Some("2".into()));
}

#[test]
fn paginated_response_first_page_no_prev() {
    let res = make_anilist_response(vec![], Some(make_page_info(1, false)));
    let paginated: PaginatedResponse = res.into();
    assert!(paginated.paging.prev_cursor.is_none());
    assert!(paginated.paging.next_cursor.is_none());
}

#[test]
fn paginated_response_no_data_node() {
    let res = AniListResponse {
        data: None,
        errors: None,
    };
    let paginated: PaginatedResponse = res.into();
    assert!(paginated.data.is_empty());
    assert!(!paginated.paging.has_next);
}

#[test]
fn paginated_response_no_page_info() {
    let media = make_full_media(3, "ANIME", "TV", "FINISHED");
    let entry = make_entry("COMPLETED", Some(media));
    let res = make_anilist_response(vec![entry], None);
    let paginated: PaginatedResponse = res.into();
    assert!(!paginated.paging.has_next);
    assert!(paginated.paging.next_cursor.is_none());
}

#[test]
fn paginated_response_invalid_entries_filtered() {
    // Entry with no media should be silently dropped
    let valid_media = make_full_media(10, "ANIME", "TV", "FINISHED");
    let valid_entry = make_entry("CURRENT", Some(valid_media));
    let invalid_entry = make_entry("CURRENT", None); // no media → TryFrom fails
    let res = make_anilist_response(
        vec![valid_entry, invalid_entry],
        Some(make_page_info(1, false)),
    );
    let paginated: PaginatedResponse = res.into();
    assert_eq!(paginated.data.len(), 1);
}

#[test]
fn paginated_response_with_graphql_errors() {
    // Errors field is present but AniListResponse → PaginatedResponse ignores it
    // (the error check is in the HTTP layer, not in the From impl)
    let res = AniListResponse {
        data: Some(AniListData { page: None }),
        errors: Some(vec![AniListGraphqlError {
            message: "Not found".into(),
            status: Some(404),
        }]),
    };
    let paginated: PaginatedResponse = res.into();
    assert!(paginated.data.is_empty());
}

#[test]
fn paginated_response_page_info_no_has_next_flag() {
    let mut pi = make_page_info(2, false);
    pi.has_next_page = None; // missing field defaults to false
    let res = make_anilist_response(vec![], Some(pi));
    let paginated: PaginatedResponse = res.into();
    assert!(!paginated.paging.has_next);
    assert!(paginated.paging.next_cursor.is_none());
}
