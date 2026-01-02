//! Conference Dashboard - Entry point
//!
//! Usage:
//!   cargo run -- --dataflow dataflow.yml          # Start dataflow and connect as dashboard
//!   cargo run -- -d dataflow.yml                  # Short form
//!   DORA_NODE_ID=dashboard cargo run              # Connect to existing dataflow
//!   cargo run                                     # Demo mode (no Dora connection)

use conference_dashboard::audio_player::create_audio_player;
use conference_dashboard::SharedStateRef;
use sysinfo::System;
use std::process::Command;

/// Parse command line arguments
struct Args {
    dataflow_path: Option<String>,
    node_name: String,
    sample_rate: u32,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut dataflow_path: Option<String> = None;
    let mut node_name = String::from("dashboard");
    let mut sample_rate: u32 = 32000;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dataflow" | "-d" => {
                if i + 1 < args.len() {
                    dataflow_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("[dashboard] ERROR: --dataflow requires a path");
                    i += 1;
                }
            }
            "--name" | "-n" => {
                if i + 1 < args.len() {
                    node_name = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("[dashboard] ERROR: --name requires a value");
                    i += 1;
                }
            }
            "--sample-rate" | "-s" => {
                if i + 1 < args.len() {
                    sample_rate = args[i + 1].parse().unwrap_or(32000);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("Conference Dashboard - Visual UI for Dora conference system");
                println!();
                println!("USAGE:");
                println!("    conference-dashboard [OPTIONS]");
                println!();
                println!("OPTIONS:");
                println!("    -d, --dataflow <PATH>     Path to dataflow YAML file to start");
                println!("    -n, --name <NAME>         Node name to connect as (default: dashboard)");
                println!("    -s, --sample-rate <RATE>  Audio sample rate in Hz (default: 32000)");
                println!("    -h, --help                Print help information");
                println!();
                println!("ENVIRONMENT VARIABLES:");
                println!("    DORA_NODE_ID              Node name (if --name not provided)");
                println!("    SAMPLE_RATE               Audio sample rate (if --sample-rate not provided)");
                println!("    DATAFLOW_PATH             Dataflow file (if --dataflow not provided)");
                println!();
                println!("EXAMPLES:");
                println!("    conference-dashboard --dataflow dataflow-study.yml");
                println!("    conference-dashboard -d dataflow.yml -n my-dashboard");
                println!("    DORA_NODE_ID=dashboard conference-dashboard  # Connect to existing dataflow");
                std::process::exit(0);
            }
            arg => {
                // Check if it looks like a dataflow file (positional argument)
                if arg.ends_with(".yml") || arg.ends_with(".yaml") {
                    dataflow_path = Some(arg.to_string());
                }
                i += 1;
            }
        }
    }

    // Fallback to environment variables
    if dataflow_path.is_none() {
        dataflow_path = std::env::var("DATAFLOW_PATH").ok();
    }
    if let Ok(name) = std::env::var("DORA_NODE_ID") {
        node_name = name;
    } else if let Ok(name) = std::env::var("DORA_NODE_NAME") {
        node_name = name;
    }
    if let Ok(rate) = std::env::var("SAMPLE_RATE") {
        sample_rate = rate.parse().unwrap_or(32000);
    }

    Args {
        dataflow_path,
        node_name,
        sample_rate,
    }
}

/// Ensure dora daemon is running
fn ensure_dora_daemon() -> Result<(), String> {
    log::info!("Checking if dora daemon is running...");

    let list_output = Command::new("dora")
        .arg("list")
        .output();

    let daemon_running = match &list_output {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            !stderr.contains("Could not connect") && !stderr.contains("Connection refused")
        }
        Err(_) => false,
    };

    if !daemon_running {
        log::info!("Dora daemon not running, starting with 'dora up'...");
        let up_output = Command::new("dora")
            .arg("up")
            .output()
            .map_err(|e| format!("Failed to execute dora up: {}", e))?;

        if !up_output.status.success() {
            let stderr = String::from_utf8_lossy(&up_output.stderr);
            if !stderr.contains("already") {
                return Err(format!("Failed to start dora daemon: {}", stderr));
            }
        }
        log::info!("Dora daemon started");
        // Give daemon time to initialize
        std::thread::sleep(std::time::Duration::from_secs(1));
    } else {
        log::info!("Dora daemon is running");
    }

    Ok(())
}

/// Check if a dataflow is already running
fn is_dataflow_running() -> bool {
    let list_output = Command::new("dora")
        .arg("list")
        .output();

    match list_output {
        Ok(output) => {
            let list_str = String::from_utf8_lossy(&output.stdout);
            list_str.lines()
                .skip(1)
                .any(|line| !line.trim().is_empty() && line.contains("Running"))
        }
        Err(_) => false,
    }
}

/// Start a dataflow
fn start_dataflow(path: &str) -> Result<(), String> {
    log::info!("Starting dataflow: {}", path);

    let output = Command::new("dora")
        .arg("start")
        .arg(path)
        .arg("--detach")
        .output()
        .map_err(|e| format!("Failed to execute dora start: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to start dataflow: {}", stderr));
    }

    log::info!("Dataflow started successfully");

    // Wait for nodes to initialize
    log::info!("Waiting for nodes to initialize (5s)...");
    std::thread::sleep(std::time::Duration::from_secs(5));

    Ok(())
}

/// Stop all running dataflows
fn stop_all_dataflows() {
    log::info!("Stopping running dataflows...");

    let list_output = Command::new("dora")
        .arg("list")
        .output();

    if let Ok(output) = list_output {
        let list_str = String::from_utf8_lossy(&output.stdout);

        for line in list_str.lines().skip(1) {
            if line.contains("Running") {
                if let Some(uuid) = line.split_whitespace().next() {
                    log::info!("Stopping dataflow: {}", uuid);
                    let _ = Command::new("dora")
                        .arg("stop")
                        .arg(uuid)
                        .output();
                }
            }
        }
    }
}

/// Start system monitor thread that updates CPU and memory usage
fn start_system_monitor(shared_state: SharedStateRef) {
    std::thread::spawn(move || {
        let mut sys = System::new_all();

        loop {
            // Refresh CPU and memory info
            sys.refresh_cpu_usage();
            sys.refresh_memory();

            // Calculate CPU usage (average across all cores)
            let cpu_usage = sys.cpus().iter()
                .map(|cpu| cpu.cpu_usage())
                .sum::<f32>() / sys.cpus().len().max(1) as f32;

            // Calculate memory usage
            let total_memory = sys.total_memory() as f64;
            let used_memory = sys.used_memory() as f64;
            let memory_usage = if total_memory > 0.0 {
                (used_memory / total_memory * 100.0) as f32
            } else {
                0.0
            };

            // Update shared state
            {
                let mut state = shared_state.lock();
                state.cpu_usage = cpu_usage;
                state.memory_usage = memory_usage;
                state.total_memory_gb = (total_memory / 1024.0 / 1024.0 / 1024.0) as f32;
                state.used_memory_gb = (used_memory / 1024.0 / 1024.0 / 1024.0) as f32;
            }

            // Update every 1 second
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Starting Conference Dashboard");

    // Parse command line arguments
    let args = parse_args();

    log::info!("Audio sample rate: {} Hz", args.sample_rate);
    log::info!("Node name: {}", args.node_name);

    // If dataflow path is provided, manage the dataflow lifecycle
    let dataflow_started = if let Some(ref dataflow_path) = args.dataflow_path {
        log::info!("Dataflow path: {}", dataflow_path);

        // Ensure dora daemon is running
        if let Err(e) = ensure_dora_daemon() {
            log::error!("{}", e);
            return;
        }

        // Check if dataflow is already running
        if !is_dataflow_running() {
            if let Err(e) = start_dataflow(dataflow_path) {
                log::error!("{}", e);
                return;
            }
            true
        } else {
            log::info!("Dataflow already running, connecting to it...");
            false
        }
    } else {
        log::info!("No dataflow specified, will connect to existing or run in demo mode");
        false
    };

    // Set the node name environment variable for dora_bridge
    std::env::set_var("DORA_NODE_ID", &args.node_name);

    // Create audio player
    let audio_player = match create_audio_player(args.sample_rate) {
        Ok(player) => {
            log::info!("Audio player initialized successfully");
            Some(player)
        }
        Err(e) => {
            log::warn!("Failed to create audio player: {}. Running without audio.", e);
            None
        }
    };

    // Create shared state
    let shared_state = conference_dashboard::create_shared_state();

    // Start system monitor in background thread
    start_system_monitor(shared_state.clone());

    // Start microphone input level monitor in background thread
    conference_dashboard::start_mic_monitor(shared_state.clone());

    // Start Dora bridge in background thread
    let state_clone = shared_state.clone();
    let player_clone = audio_player.clone();
    std::thread::spawn(move || {
        if let Err(e) = conference_dashboard::dora_bridge::run_dora_bridge(state_clone, player_clone) {
            log::error!("Dora bridge error: {}", e);
        }
    });

    // Store shared state for app to access
    conference_dashboard::app::set_shared_state(shared_state);

    // Set up Ctrl+C handler to stop dataflow on exit
    let dataflow_started_clone = dataflow_started;
    ctrlc::set_handler(move || {
        log::info!("Ctrl+C received, shutting down...");
        if dataflow_started_clone {
            stop_all_dataflows();
        }
        std::process::exit(0);
    }).expect("Error setting Ctrl+C handler");

    // Start Makepad UI (blocks until window closes)
    conference_dashboard::app::app_main();

    // Stop dataflow on normal exit if we started it
    if dataflow_started {
        stop_all_dataflows();
    }
}