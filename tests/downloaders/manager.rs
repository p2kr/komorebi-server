use komorebi_server::{core::is_active_status, models::vault::VaultStatus};

#[test]
fn test_is_active_status_includes_processing_and_completed() {
    // Active or pending work on startup
    assert!(is_active_status(&VaultStatus::PENDING));
    assert!(is_active_status(&VaultStatus::DOWNLOADING));
    assert!(is_active_status(&VaultStatus::PAUSED));
    assert!(is_active_status(&VaultStatus::COMPLETED));
    assert!(is_active_status(&VaultStatus::PROCESSING));

    // Inactive / terminal states excluded from active downloads
    assert!(!is_active_status(&VaultStatus::READY));
    assert!(!is_active_status(&VaultStatus::FAILED));
    assert!(!is_active_status(&VaultStatus::CANCELLED));
}
