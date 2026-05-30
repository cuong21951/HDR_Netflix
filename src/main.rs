#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod hdr;
mod icon;
mod netflix;
mod startup;
mod tray;

use config::Config;
use std::{
    env,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

type SharedState = Arc<Mutex<AppState>>;

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub config: Config,
    pub netflix_running: bool,
    pub hdr_active_by_app: bool,
    pub last_error: Option<String>,
    pub exiting: bool,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Some(exit_code) = handle_cli(&args) {
        std::process::exit(exit_code);
    }

    let (config, config_path) = match Config::load_or_create() {
        Ok(value) => value,
        Err(err) => {
            eprintln!("Failed to load config: {err}");
            std::process::exit(1);
        }
    };

    let state = Arc::new(Mutex::new(AppState {
        netflix_running: false,
        hdr_active_by_app: false,
        last_error: None,
        exiting: false,
        config,
    }));

    let worker_state = Arc::clone(&state);
    thread::spawn(move || watcher_loop(worker_state));

    if let Err(err) = tray::run(state, config_path) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn handle_cli(args: &[String]) -> Option<i32> {
    match args.first().map(String::as_str) {
        Some("--on") => Some(command_set_hdr(true)),
        Some("--off") => Some(command_set_hdr(false)),
        Some("--status") => Some(command_status()),
        Some("--detect-netflix") => Some(command_detect_netflix(args)),
        Some("--help") | Some("-h") => {
            print_help();
            Some(0)
        }
        Some(_) => {
            print_help();
            Some(2)
        }
        None => None,
    }
}

fn command_set_hdr(enable: bool) -> i32 {
    let config = Config::load_or_create()
        .map(|(config, _)| config)
        .unwrap_or_default();

    match hdr::set_hdr_for_targets(enable, &config.selected_targets) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn command_status() -> i32 {
    let config = Config::load_or_create()
        .map(|(config, _)| config)
        .unwrap_or_default();

    match hdr::get_statuses(&config.selected_targets) {
        Ok(statuses) => {
            for status in statuses {
                println!(
                    "{} name=\"{}\" supported={} enabled={} force_disabled={}",
                    status.target.key,
                    status.target.name,
                    status.supported,
                    status.enabled,
                    status.force_disabled
                );
            }
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn command_detect_netflix(args: &[String]) -> i32 {
    let config = Config::load_or_create()
        .map(|(config, _)| config)
        .unwrap_or_default();
    let detect_window_titles = !args.iter().any(|arg| arg == "--no-window-titles");
    let reasons = netflix::detection_reasons(&config.process_names, detect_window_titles);
    let running = !reasons.is_empty();
    println!("netflix_running={running}");
    for reason in reasons {
        println!("reason={reason}");
    }
    if running { 0 } else { 1 }
}

fn print_help() {
    println!("HDR Netflix");
    println!("  HDRNetflix.exe           Run tray app");
    println!("  HDRNetflix.exe --status  List active display HDR status");
    println!("  HDRNetflix.exe --on      Turn HDR on for configured displays");
    println!("  HDRNetflix.exe --off     Turn HDR off for configured displays");
    println!("  HDRNetflix.exe --detect-netflix [--no-window-titles]");
}

fn watcher_loop(state: SharedState) {
    loop {
        let snapshot = {
            let guard = state.lock().expect("state poisoned");
            if guard.exiting {
                break;
            }
            guard.clone()
        };

        let running = netflix::is_netflix_running(
            &snapshot.config.process_names,
            snapshot.config.detect_window_titles,
        );

        let hdr_state = configured_hdr_state(&snapshot.config.selected_targets);
        let needs_hdr_on = snapshot.config.auto_enabled
            && running
            && (!snapshot.hdr_active_by_app || !hdr_state.all_on);

        if needs_hdr_on {
            apply_hdr_change(&state, true);
        } else if snapshot.config.auto_enabled && running && hdr_state.any_on {
            mark_hdr_active(&state, true, None);
        } else if (!running || !snapshot.config.auto_enabled)
            && snapshot.config.turn_off_when_netflix_closes
            && (snapshot.hdr_active_by_app || hdr_state.any_on)
        {
            apply_hdr_change(&state, false);
        }

        if let Ok(mut guard) = state.lock() {
            guard.netflix_running = running;
        }

        thread::sleep(Duration::from_millis(
            snapshot.config.poll_interval_ms.max(500),
        ));
    }
}

#[derive(Default)]
struct HdrState {
    any_on: bool,
    all_on: bool,
}

fn configured_hdr_state(selected_targets: &[String]) -> HdrState {
    match hdr::get_statuses(selected_targets) {
        Ok(statuses) => {
            let mut supported = statuses
                .iter()
                .filter(|status| status.supported && !status.force_disabled)
                .peekable();
            if supported.peek().is_none() {
                return HdrState::default();
            }

            let states: Vec<bool> = supported.map(|status| status.enabled).collect();
            HdrState {
                any_on: states.iter().any(|enabled| *enabled),
                all_on: states.iter().all(|enabled| *enabled),
            }
        }
        Err(_) => HdrState::default(),
    }
}

fn apply_hdr_change(state: &SharedState, enable: bool) {
    let selected_targets = {
        let guard = state.lock().expect("state poisoned");
        guard.config.selected_targets.clone()
    };

    let result = hdr::set_hdr_for_targets(enable, &selected_targets);
    let last_error = result.err();
    mark_hdr_active(state, enable && last_error.is_none(), last_error);
}

fn mark_hdr_active(state: &SharedState, active: bool, last_error: Option<String>) {
    if let Ok(mut guard) = state.lock() {
        guard.hdr_active_by_app = active;
        guard.last_error = last_error;
    }
}
