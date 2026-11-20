// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: changed the logger tests to wait for the previous logger before reusing its path; renamed the Warp and Zap names to ZapNG.

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use zap_ng_ui::r#async::executor::Background;

use super::LogManager;

fn temp_path(name: &str) -> PathBuf {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "simple-logger-tests-{name}-{}-{id}",
        std::process::id()
    ))
}
fn cleanup_log_path(log_path: &Path) {
    let _ = std::fs::remove_file(log_path);
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }
}

#[test]
fn register_resolved_path_reuses_stale_entries_after_drop() {
    let mut manager = LogManager::new();
    let executor = Arc::new(Background::default());
    let log_path = temp_path("re-register").join("server.log");

    let logger = manager
        .register_resolved_path(log_path.clone(), executor.clone())
        .expect("initial registration should succeed");

    drop(logger);

    let logger = manager
        .register_resolved_path(log_path.clone(), executor)
        .expect("stale entry should be reclaimed after the logger is dropped");
    drop(logger);
    cleanup_log_path(&log_path);
}

#[test]
fn register_resolved_path_rejects_duplicate_active_loggers() {
    let mut manager = LogManager::new();
    let executor = Arc::new(Background::default());
    let log_path = temp_path("collision").join("server.log");

    let logger = manager
        .register_resolved_path(log_path.clone(), executor.clone())
        .expect("initial registration should succeed");
    assert!(
        manager
            .register_resolved_path(log_path.clone(), executor)
            .is_err(),
        "live logger should block duplicate registration"
    );

    drop(logger);
    cleanup_log_path(&log_path);
}

#[test]
fn register_reclaims_closed_logger() {
    let mut manager = LogManager::new();
    let executor = Arc::new(Background::default());
    let log_path = temp_path("close-reclaim").join("server.log");

    let logger = manager
        .register_resolved_path(log_path.clone(), executor.clone())
        .expect("initial registration should succeed");

    // Close the channel without dropping the logger — the Arc<LogFileWriter> is still alive.
    logger.close();

    let new_logger = manager
        .register_resolved_path(log_path.clone(), executor)
        .expect("closed logger should be reclaimed even when Arc is still alive");

    drop(logger);
    drop(new_logger);
    cleanup_log_path(&log_path);
}

/// A logger that is closed with lines still queued keeps writing them after its
/// path has been reclaimed. The logger that takes the path over must not open
/// the file until then: otherwise the old lines land in the new logger's file,
/// at the old logger's file offset.
#[test]
fn reclaiming_a_path_waits_for_the_previous_logger_to_finish_writing() {
    let mut manager = LogManager::new();
    let executor = Arc::new(Background::default());
    let log_path = temp_path("reclaim-while-flushing").join("server.log");

    let old_logger = manager
        .register_resolved_path(log_path.clone(), executor.clone())
        .expect("initial registration should succeed");
    for i in 0..2000 {
        old_logger.log(format!("old line {i}"));
    }
    old_logger.close();

    let new_logger = manager
        // Keep the executor: dropping it cancels the logging tasks awaited below.
        .register_resolved_path(log_path.clone(), executor.clone())
        .expect("closed logger should be reclaimed");
    new_logger.log("new line".to_string());
    new_logger.close();

    let writer = Arc::try_unwrap(new_logger.writer)
        .unwrap_or_else(|_| panic!("logger should have no other owners"));
    futures::executor::block_on(writer._logging_task).expect("logging task should finish");

    let contents = std::fs::read_to_string(&log_path).expect("read log file");
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "only the new logger's line should be in the file, got {} lines",
        lines.len()
    );
    assert!(
        lines[0].ends_with(" | new line"),
        "unexpected line: {:?}",
        lines[0]
    );

    drop(old_logger);
    cleanup_log_path(&log_path);
}
