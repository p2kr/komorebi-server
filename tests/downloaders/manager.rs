use komorebi_server::{core::is_active_status, models::vault::VaultItemStatus};

#[test]
fn test_is_active_status_includes_processing_and_completed() {
    // Active or pending work on startup
    assert!(is_active_status(&VaultItemStatus::PENDING));
    assert!(is_active_status(&VaultItemStatus::DOWNLOADING));
    assert!(is_active_status(&VaultItemStatus::PAUSED));
    assert!(is_active_status(&VaultItemStatus::COMPLETED));
    assert!(is_active_status(&VaultItemStatus::PROCESSING));

    // Inactive / terminal states excluded from active downloads
    assert!(!is_active_status(&VaultItemStatus::READY));
    assert!(!is_active_status(&VaultItemStatus::FAILED));
    assert!(!is_active_status(&VaultItemStatus::CANCELLED));
}
