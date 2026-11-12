use crate::app::state::{App, UpdatePhase};
use crate::app::updater::{self, AfterExit, Progress};
use crossterm::event::{KeyCode, KeyEvent};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

/// Returns true when update state changes.
pub fn poll(app: &mut App) -> bool {
    let mut changed = false;
    if let Ok(Some(update)) = app.update_rx.try_recv() {
        app.update = Some(update);
        app.update_phase = UpdatePhase::Offer;
        app.show_update_popup = true;
        changed = true;
    }
    while let Some(rx) = &app.update_progress_rx {
        let event = match rx.try_recv() {
            Ok(event) => event,
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                Progress::Finished(Err("the update stopped unexpectedly".into()))
            }
        };
        changed = true;
        match event {
            Progress::Downloading { received, total } => {
                app.update_phase = UpdatePhase::Downloading { received, total };
            }
            Progress::Finished(result) => {
                app.update_progress_rx = None;
                finish(app, result);
            }
        }
    }
    changed
}

fn finish(app: &mut App, result: Result<AfterExit, String>) {
    let cancelled = app.update_cancel.load(Ordering::Relaxed);
    match result {
        Ok(AfterExit::Restart(exe)) => {
            app.update_phase = UpdatePhase::Installed(exe);
            // Cancelled too late: the swap already happened.
            if cancelled {
                updated_message(app);
            }
        }
        Ok(after) if !cancelled => {
            app.after_exit = Some(after);
            app.quit();
        }
        Ok(_) => app.update_phase = UpdatePhase::Offer,
        Err(_) if cancelled => app.update_phase = UpdatePhase::Offer,
        Err(e) => app.update_phase = UpdatePhase::Failed(e),
    }
}

pub fn handle_update_popup(app: &mut App, key: KeyEvent) {
    let Some(update) = app.update.clone() else {
        app.show_update_popup = false;
        return;
    };
    match &app.update_phase {
        UpdatePhase::Downloading { .. } => {
            if key.code == KeyCode::Esc {
                app.update_cancel.store(true, Ordering::Relaxed);
                app.show_update_popup = false;
                app.set_status_message("Update cancelled.", Some(chrono::Duration::seconds(5)));
            }
        }
        UpdatePhase::Installed(exe) => {
            if matches!(key.code, KeyCode::Char('r' | 'R')) {
                app.after_exit = Some(AfterExit::Restart(exe.clone()));
                app.quit();
            } else {
                updated_message(app);
            }
        }
        UpdatePhase::Offer | UpdatePhase::Failed(_) => match key.code {
            KeyCode::Char('u' | 'U')
                if matches!(app.update_phase, UpdatePhase::Offer) && update.action.in_app() =>
            {
                start_download(app, update.action);
            }
            KeyCode::Char('u' | 'U')
                if matches!(app.update_phase, UpdatePhase::Offer)
                    && update.action.command().is_some() =>
            {
                app.after_exit = Some(AfterExit::Upgrade {
                    action: update.action,
                    version: update.version,
                });
                app.quit();
            }
            KeyCode::Enter | KeyCode::Char('o' | 'O') => open_releases_page(app),
            _ => app.show_update_popup = false,
        },
    }
}

fn updated_message(app: &mut App) {
    app.show_update_popup = false;
    let version = app.update.as_ref().map_or("", |u| u.version.as_str());
    app.set_status_message(
        format!("Updated to {version}. Restart Budget Tracker to use it."),
        Some(chrono::Duration::seconds(5)),
    );
}

fn start_download(app: &mut App, action: updater::Action) {
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    app.update_cancel = Arc::clone(&cancel);
    app.update_progress_rx = Some(rx);
    app.update_phase = UpdatePhase::Downloading {
        received: 0,
        total: action.download_size(),
    };
    thread::spawn(move || {
        let progress_tx = tx.clone();
        let mut last_percent = None;
        let result = updater::install(&action, &cancel, |received, total| {
            let percent = received * 100 / total.max(1);
            if last_percent != Some(percent) {
                last_percent = Some(percent);
                let _ = progress_tx.send(Progress::Downloading { received, total });
            }
        });
        let _ = tx.send(Progress::Finished(result));
    });
}

fn open_releases_page(app: &mut App) {
    app.show_update_popup = false;
    if crate::app::util::open_url(updater::RELEASES_PAGE) {
        app.quit();
    } else {
        app.set_status_message(
            format!("Could not open browser. Visit {}", updater::RELEASES_PAGE),
            Some(chrono::Duration::seconds(5)),
        );
    }
}
