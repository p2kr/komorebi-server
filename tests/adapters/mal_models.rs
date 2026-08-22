use komorebi_server::adapters::mal_models::{
    MalAltTitles, MalGenre, MalItem, MalListStatus, MalNode, MalPaging, MalPicture, MalResponse,
    MalStatus,
};
use komorebi_server::models::media::{
    ListEntry, ListStatus, MediaEntry, MediaFormat, MediaProvider, MediaType, NsfwLevel,
    PaginatedResponse,
};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn make_list_status(status: &str, rewatching: bool, rereading: bool) -> MalListStatus {
    MalListStatus {
        status: Some(status.to_string()),
        score: Some(8.0),
        num_episodes_watched: Some(12.0),
        num_chapters_read: Some(50),
        num_volumes_read: Some(5),
        is_rewatching: Some(rewatching),
        is_rereading: Some(rereading),
        num_times_rewatched: Some(2),
        num_times_reread: Some(1),
        tags: Some(vec!["fav".to_string()]),
        comments: Some("Good show".to_string()),
        updated_at: Some("2024-01-01T00:00:00Z".to_string()),
    }
}

fn make_node(id: i64, title: &str, media_type: &str, status: &str) -> MalNode {
    MalNode {
        id: Some(id),
        title: Some(title.to_string()),
        alternative_titles: Some(MalAltTitles {
            en: Some("English Title".to_string()),
            ja: Some("日本語タイトル".to_string()),
        }),
        main_picture: Some(MalPicture {
            medium: Some("medium.jpg".to_string()),
            large: Some("large.jpg".to_string()),
        }),
        genres: Some(vec![
            MalGenre { id: Some(1), name: Some("Action".to_string()) },
            MalGenre { id: Some(2), name: None }, // name-less genre should be filtered
        ]),
        synopsis: Some("A great story.".to_string()),
        mean: Some(8.5),
        popularity: Some(1000),
        num_episodes: Some(24),
        average_episode_duration: Some(1440),
        num_chapters: None,
        num_volumes: None,
        nsfw: Some("white".to_string()),
        my_list_status: None,
        media_type: Some(media_type.to_string()),
        status: Some(status.to_string()),
    }
}

fn make_item(id: i64, title: &str, media_type: &str, list_status: Option<MalListStatus>) -> MalItem {
    MalItem {
        node: make_node(id, title, media_type, "currently_airing"),
        list_status,
    }
}

// ─── MalStatus::try_from ─────────────────────────────────────────────────────

#[test]
fn mal_status_watching_non_manga() {
    assert_eq!(MalStatus::try_from(("watching", false)), Ok(MalStatus::Watching));
    assert_eq!(MalStatus::try_from(("current", false)), Ok(MalStatus::Watching));
    assert_eq!(MalStatus::try_from(("reading", false)), Ok(MalStatus::Watching));
    assert_eq!(MalStatus::try_from(("repeating", false)), Ok(MalStatus::Watching));
    assert_eq!(MalStatus::try_from(("rewatching", false)), Ok(MalStatus::Watching));
    assert_eq!(MalStatus::try_from(("rereading", false)), Ok(MalStatus::Watching));
}

#[test]
fn mal_status_reading_manga() {
    assert_eq!(MalStatus::try_from(("watching", true)), Ok(MalStatus::Reading));
    assert_eq!(MalStatus::try_from(("current", true)), Ok(MalStatus::Reading));
    assert_eq!(MalStatus::try_from(("reading", true)), Ok(MalStatus::Reading));
    assert_eq!(MalStatus::try_from(("repeating", true)), Ok(MalStatus::Reading));
    assert_eq!(MalStatus::try_from(("rewatching", true)), Ok(MalStatus::Reading));
    assert_eq!(MalStatus::try_from(("rereading", true)), Ok(MalStatus::Reading));
}

#[test]
fn mal_status_plan_to_watch() {
    assert_eq!(MalStatus::try_from(("planning", false)), Ok(MalStatus::PlanToWatch));
    assert_eq!(MalStatus::try_from(("plan_to_watch", false)), Ok(MalStatus::PlanToWatch));
    assert_eq!(MalStatus::try_from(("plan_to_read", false)), Ok(MalStatus::PlanToWatch));
}

#[test]
fn mal_status_plan_to_read() {
    assert_eq!(MalStatus::try_from(("planning", true)), Ok(MalStatus::PlanToRead));
    assert_eq!(MalStatus::try_from(("plan_to_watch", true)), Ok(MalStatus::PlanToRead));
    assert_eq!(MalStatus::try_from(("plan_to_read", true)), Ok(MalStatus::PlanToRead));
}

#[test]
fn mal_status_on_hold() {
    assert_eq!(MalStatus::try_from(("paused", false)), Ok(MalStatus::OnHold));
    assert_eq!(MalStatus::try_from(("on_hold", false)), Ok(MalStatus::OnHold));
    assert_eq!(MalStatus::try_from(("paused", true)), Ok(MalStatus::OnHold));
}

#[test]
fn mal_status_completed_and_dropped() {
    assert_eq!(MalStatus::try_from(("completed", false)), Ok(MalStatus::Completed));
    assert_eq!(MalStatus::try_from(("completed", true)), Ok(MalStatus::Completed));
    assert_eq!(MalStatus::try_from(("dropped", false)), Ok(MalStatus::Dropped));
    assert_eq!(MalStatus::try_from(("dropped", true)), Ok(MalStatus::Dropped));
}

#[test]
fn mal_status_unknown_is_err() {
    assert!(MalStatus::try_from(("garbage", false)).is_err());
    assert!(MalStatus::try_from(("", true)).is_err());
}

#[test]
fn mal_status_case_insensitive() {
    assert_eq!(MalStatus::try_from(("WATCHING", false)), Ok(MalStatus::Watching));
    assert_eq!(MalStatus::try_from(("Completed", false)), Ok(MalStatus::Completed));
}

// ─── ListStatus from MalListStatus ───────────────────────────────────────────

#[test]
fn list_status_from_mal_watching() {
    let ls = make_list_status("watching", false, false);
    let status = ListStatus::from(ls);
    assert_eq!(status, ListStatus::Current);
}

#[test]
fn list_status_from_mal_reading() {
    let ls = make_list_status("reading", false, false);
    let status = ListStatus::from(ls);
    assert_eq!(status, ListStatus::Current);
}

#[test]
fn list_status_from_mal_plan_to_watch() {
    let ls = make_list_status("plan_to_watch", false, false);
    let status = ListStatus::from(ls);
    assert_eq!(status, ListStatus::Planning);
}

#[test]
fn list_status_from_mal_plan_to_read() {
    let ls = make_list_status("plan_to_read", false, false);
    let status = ListStatus::from(ls);
    assert_eq!(status, ListStatus::Planning);
}

#[test]
fn list_status_from_mal_completed() {
    let ls = make_list_status("completed", false, false);
    let status = ListStatus::from(ls);
    assert_eq!(status, ListStatus::Completed);
}

#[test]
fn list_status_from_mal_dropped() {
    let ls = make_list_status("dropped", false, false);
    let status = ListStatus::from(ls);
    assert_eq!(status, ListStatus::Dropped);
}

#[test]
fn list_status_from_mal_on_hold() {
    let ls = make_list_status("on_hold", false, false);
    let status = ListStatus::from(ls);
    assert_eq!(status, ListStatus::Paused);
}

#[test]
fn list_status_from_mal_rewatching_flag() {
    let ls = make_list_status("watching", true, false);
    let status = ListStatus::from(ls);
    assert_eq!(status, ListStatus::Repeating);
}

#[test]
fn list_status_from_mal_rereading_flag() {
    let ls = make_list_status("reading", false, true);
    let status = ListStatus::from(ls);
    assert_eq!(status, ListStatus::Repeating);
}

#[test]
fn list_status_from_mal_unknown_falls_back_to_current() {
    let ls = make_list_status("bogus", false, false);
    let status = ListStatus::from(ls);
    assert_eq!(status, ListStatus::Current);
}

// ─── ListEntry from MalListStatus ────────────────────────────────────────────

#[test]
fn list_entry_from_mal_all_fields() {
    let ls = make_list_status("watching", false, false);
    let le = ListEntry::from(ls);
    assert_eq!(le.score, Some(8.0_f32));
    // num_episodes_watched takes priority for progress
    assert_eq!(le.progress, Some(12));
    assert_eq!(le.progress_volumes, Some(5));
    assert!(!le.is_repeating);
    // num_times_rewatched takes priority
    assert_eq!(le.repeat_count, Some(2));
    assert_eq!(le.tags, vec!["fav".to_string()]);
    assert_eq!(le.notes, Some("Good show".to_string()));
    assert_eq!(le.updated_at, Some("2024-01-01T00:00:00Z".to_string()));
}

#[test]
fn list_entry_from_mal_repeating() {
    let ls = make_list_status("watching", true, false);
    let le = ListEntry::from(ls);
    assert!(le.is_repeating);
    assert_eq!(le.status, ListStatus::Repeating);
}

#[test]
fn list_entry_chapters_read_used_when_no_episodes() {
    let mut ls = make_list_status("reading", false, false);
    ls.num_episodes_watched = None;
    let le = ListEntry::from(ls);
    assert_eq!(le.progress, Some(50)); // falls back to num_chapters_read
}

// ─── MediaEntry from MalItem ──────────────────────────────────────────────────

#[test]
fn media_entry_missing_title_is_err() {
    let mut item = make_item(1, "Title", "tv", None);
    item.node.title = None;
    let result: Result<MediaEntry, ()> = item.try_into();
    assert!(result.is_err());
}

#[test]
fn media_entry_missing_id_is_err() {
    let mut item = make_item(1, "Title", "tv", None);
    item.node.id = None;
    let result: Result<MediaEntry, ()> = item.try_into();
    assert!(result.is_err());
}

#[test]
fn media_entry_anime_full_conversion() {
    let ls = make_list_status("watching", false, false);
    let item = make_item(42, "My Anime", "tv", Some(ls));
    let me: MediaEntry = item.try_into().expect("should succeed");
    assert_eq!(me.media.provider_id, "42");
    assert_eq!(me.media.provider, MediaProvider::MAL);
    assert_eq!(me.media.media_type, MediaType::Anime);
    assert_eq!(me.media.format, MediaFormat::Tv);
    assert_eq!(me.media.title.romanized, Some("My Anime".to_string()));
    assert_eq!(me.media.title.user_preferred, Some("My Anime".to_string()));
    assert_eq!(me.media.title.english, Some("English Title".to_string()));
    assert_eq!(me.media.title.native, Some("日本語タイトル".to_string()));
    assert_eq!(me.media.cover.medium, Some("medium.jpg".to_string()));
    assert_eq!(me.media.cover.large, Some("large.jpg".to_string()));
    // extra_large falls back to large when large exists
    assert_eq!(me.media.cover.extra_large, Some("large.jpg".to_string()));
    assert_eq!(me.media.nsfw, NsfwLevel::Safe);
    // Only the genre with a name is included
    assert_eq!(me.media.genres, vec!["Action".to_string()]);
    assert_eq!(me.media.episodes, Some(24));
    assert_eq!(me.media.duration, Some(1440)); // stored as-is in seconds
}

#[test]
fn media_entry_uses_node_my_list_status_when_no_item_list_status() {
    let mut item = make_item(1, "Anime", "tv", None);
    item.node.my_list_status = Some(make_list_status("completed", false, false));
    let me: MediaEntry = item.try_into().unwrap();
    assert_eq!(me.list_entry.status, ListStatus::Completed);
}

#[test]
fn media_entry_item_list_status_takes_priority() {
    let item_ls = make_list_status("dropped", false, false);
    let mut item = make_item(1, "Anime", "tv", Some(item_ls));
    item.node.my_list_status = Some(make_list_status("watching", false, false));
    let me: MediaEntry = item.try_into().unwrap();
    // item.list_status wins
    assert_eq!(me.list_entry.status, ListStatus::Dropped);
}

#[test]
fn media_entry_no_list_status_uses_default() {
    let item = make_item(1, "Anime", "tv", None);
    let me: MediaEntry = item.try_into().unwrap();
    assert_eq!(me.list_entry.status, ListStatus::Current); // default
}

// ─── Format parsing (all MAL arms) ───────────────────────────────────────────

fn format_for(media_type_str: &str) -> MediaFormat {
    let item = make_item(1, "Title", media_type_str, None);
    let me: MediaEntry = item.try_into().unwrap();
    me.media.format
}

#[test]
fn parse_mal_format_all_arms() {
    assert_eq!(format_for("tv"), MediaFormat::Tv);
    assert_eq!(format_for("ova"), MediaFormat::Ova);
    assert_eq!(format_for("movie"), MediaFormat::Movie);
    assert_eq!(format_for("special"), MediaFormat::Special);
    assert_eq!(format_for("ona"), MediaFormat::Ona);
    assert_eq!(format_for("music"), MediaFormat::Music);
    assert_eq!(format_for("manga"), MediaFormat::Manga);
    assert_eq!(format_for("novel"), MediaFormat::Novel);
    assert_eq!(format_for("one_shot"), MediaFormat::OneShot);
    assert_eq!(format_for("doujinshi"), MediaFormat::Doujinshi);
    assert_eq!(format_for("manhwa"), MediaFormat::Manhwa);
    assert_eq!(format_for("manhua"), MediaFormat::Manhua);
    assert_eq!(format_for("oel"), MediaFormat::Oel);
    assert_eq!(format_for("unknown_type"), MediaFormat::Unknown);
}

// ─── NSFW parsing ────────────────────────────────────────────────────────────

fn nsfw_level_for(nsfw_str: Option<&str>) -> NsfwLevel {
    let mut item = make_item(1, "Title", "tv", None);
    item.node.nsfw = nsfw_str.map(|s| s.to_string());
    let me: MediaEntry = item.try_into().unwrap();
    me.media.nsfw
}

#[test]
fn parse_mal_nsfw_all_arms() {
    assert_eq!(nsfw_level_for(Some("white")), NsfwLevel::Safe);
    assert_eq!(nsfw_level_for(Some("gray")), NsfwLevel::Gray);
    assert_eq!(nsfw_level_for(Some("black")), NsfwLevel::Nsfw);
    assert_eq!(nsfw_level_for(Some("unknown")), NsfwLevel::Safe); // fallback
    assert_eq!(nsfw_level_for(None), NsfwLevel::Safe);
}

// ─── Release status parsing ──────────────────────────────────────────────────

fn release_status_for(status_str: &str) -> komorebi_server::models::media::ReleaseStatus {
    let mut item = make_item(1, "Title", "tv", None);
    item.node.status = Some(status_str.to_string());
    let me: MediaEntry = item.try_into().unwrap();
    me.media.release_status
}

#[test]
fn parse_mal_release_status_all_arms() {
    use komorebi_server::models::media::ReleaseStatus;
    assert_eq!(release_status_for("currently_airing"), ReleaseStatus::Releasing);
    assert_eq!(release_status_for("currently_publishing"), ReleaseStatus::Releasing);
    assert_eq!(release_status_for("finished_airing"), ReleaseStatus::Finished);
    assert_eq!(release_status_for("finished"), ReleaseStatus::Finished);
    assert_eq!(release_status_for("not_yet_aired"), ReleaseStatus::NotYetReleased);
    assert_eq!(release_status_for("not_yet_published"), ReleaseStatus::NotYetReleased);
    assert_eq!(release_status_for("unknown"), ReleaseStatus::Unknown);
}

// ─── Media type inference ─────────────────────────────────────────────────────

#[test]
fn media_type_manga_formats_give_manga_type() {
    for fmt in &["manga", "novel", "one_shot", "doujinshi", "manhwa", "manhua", "oel"] {
        let item = make_item(1, "Title", fmt, None);
        let me: MediaEntry = item.try_into().unwrap();
        assert_eq!(me.media.media_type, MediaType::Manga, "expected Manga for format {fmt}");
    }
}

#[test]
fn media_type_inferred_from_chapters() {
    let mut item = make_item(1, "Title", "unknown_type", None);
    item.node.num_chapters = Some(10);
    let me: MediaEntry = item.try_into().unwrap();
    assert_eq!(me.media.media_type, MediaType::Manga);
}

#[test]
fn media_type_inferred_from_volumes() {
    let mut item = make_item(1, "Title", "unknown_type", None);
    item.node.num_volumes = Some(5);
    let me: MediaEntry = item.try_into().unwrap();
    assert_eq!(me.media.media_type, MediaType::Manga);
}

#[test]
fn media_type_falls_back_to_anime_for_tv() {
    let item = make_item(1, "Title", "tv", None);
    let me: MediaEntry = item.try_into().unwrap();
    assert_eq!(me.media.media_type, MediaType::Anime);
}

// ─── PaginatedResponse from MalResponse ──────────────────────────────────────

#[test]
fn mal_paginated_response_with_paging() {
    let item = make_item(1, "Title", "tv", None);
    let res = MalResponse {
        data: vec![item],
        paging: Some(MalPaging {
            next: Some("https://api.mal.net/next".to_string()),
            previous: Some("https://api.mal.net/prev".to_string()),
        }),
    };
    let paginated: PaginatedResponse = res.into();
    assert_eq!(paginated.data.len(), 1);
    assert!(paginated.paging.has_next);
    assert_eq!(paginated.paging.next_cursor, Some("https://api.mal.net/next".to_string()));
    assert_eq!(paginated.paging.prev_cursor, Some("https://api.mal.net/prev".to_string()));
}

#[test]
fn mal_paginated_response_no_paging() {
    let item = make_item(1, "Title", "tv", None);
    let res = MalResponse {
        data: vec![item],
        paging: None,
    };
    let paginated: PaginatedResponse = res.into();
    assert!(!paginated.paging.has_next);
    assert!(paginated.paging.next_cursor.is_none());
    assert!(paginated.paging.prev_cursor.is_none());
}

#[test]
fn mal_paginated_response_no_next_means_no_has_next() {
    let item = make_item(1, "Title", "tv", None);
    let res = MalResponse {
        data: vec![item],
        paging: Some(MalPaging { next: None, previous: Some("prev".to_string()) }),
    };
    let paginated: PaginatedResponse = res.into();
    assert!(!paginated.paging.has_next);
}

#[test]
fn mal_paginated_response_invalid_items_filtered() {
    // Item with no title → TryFrom fails → silently dropped
    let mut bad_item = make_item(1, "Title", "tv", None);
    bad_item.node.title = None;
    let good_item = make_item(2, "Good", "tv", None);
    let res = MalResponse {
        data: vec![bad_item, good_item],
        paging: None,
    };
    let paginated: PaginatedResponse = res.into();
    assert_eq!(paginated.data.len(), 1);
}

#[test]
fn mal_paginated_response_empty_data() {
    let res = MalResponse { data: vec![], paging: None };
    let paginated: PaginatedResponse = res.into();
    assert!(paginated.data.is_empty());
    assert!(!paginated.paging.has_next);
}

// ─── No alternative titles ────────────────────────────────────────────────────

#[test]
fn media_entry_no_alt_titles_gives_none() {
    let mut item = make_item(1, "Title", "tv", None);
    item.node.alternative_titles = None;
    let me: MediaEntry = item.try_into().unwrap();
    assert!(me.media.title.english.is_none());
    assert!(me.media.title.native.is_none());
}

#[test]
fn media_entry_no_picture_gives_none_cover() {
    let mut item = make_item(1, "Title", "tv", None);
    item.node.main_picture = None;
    let me: MediaEntry = item.try_into().unwrap();
    assert!(me.media.cover.medium.is_none());
    assert!(me.media.cover.large.is_none());
    assert!(me.media.cover.extra_large.is_none());
}

#[test]
fn media_entry_no_genres_gives_empty_vec() {
    let mut item = make_item(1, "Title", "tv", None);
    item.node.genres = None;
    let me: MediaEntry = item.try_into().unwrap();
    assert!(me.media.genres.is_empty());
}
