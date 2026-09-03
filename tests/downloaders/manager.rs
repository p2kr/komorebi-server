use komorebi_server::{downloaders::manager::DownloadManager, models::vault::VaultItemStatus};

#[test]
fn test_is_active_status_includes_processing_and_completed() {
    // Active or pending work on startup
    assert!(DownloadManager::is_active_status(&VaultItemStatus::PENDING));
    assert!(DownloadManager::is_active_status(
        &VaultItemStatus::DOWNLOADING
    ));
    assert!(DownloadManager::is_active_status(&VaultItemStatus::PAUSED));
    assert!(DownloadManager::is_active_status(
        &VaultItemStatus::COMPLETED
    ));
    assert!(DownloadManager::is_active_status(
        &VaultItemStatus::PROCESSING
    ));

    // Inactive / terminal states excluded from active downloads
    assert!(!DownloadManager::is_active_status(&VaultItemStatus::READY));
    assert!(!DownloadManager::is_active_status(&VaultItemStatus::FAILED));
    assert!(!DownloadManager::is_active_status(
        &VaultItemStatus::CANCELLED
    ));
}
