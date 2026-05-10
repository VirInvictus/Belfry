//! Belfry — GNOME 50 podcast client.
//!
//! See `spec.md` for the contract.

use anyhow::{Result, bail};
use clap::Parser;
use libadwaita as adw;
use libadwaita::prelude::*;

const APP_ID: &str = "io.github.virinvictus.Belfry";

#[derive(Parser, Debug)]
#[command(name = "belfry", version, about = "GNOME podcast client")]
struct Args {
    /// Open the debug surface (memory watch, IO trace, fixture menu).
    #[arg(long)]
    debug: bool,

    /// Generate a fixture database and exit.
    /// Scales: `small` (100 episodes), `medium` (1K), `large` (10K), `stress` (50K).
    #[arg(long, value_name = "SCALE")]
    fixture: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt::init();

    if let Some(scale_str) = args.fixture.as_deref() {
        return tokio::runtime::Runtime::new()?.block_on(generate_fixture(scale_str));
    }

    if args.debug {
        tracing::info!("debug surface: not yet implemented");
    }

    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);

    let exit = app.run();
    if exit.value() != 0 {
        bail!("application exited with code {}", exit.value());
    }
    Ok(())
}

async fn generate_fixture(scale_str: &str) -> Result<()> {
    use belfry_core::db::fixtures::{FixtureScale, generate};
    use std::str::FromStr;

    let scale = FixtureScale::from_str(scale_str)?;
    let db_path = belfry_core::paths::database_file()?;

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    tracing::info!(
        scale = ?scale,
        path = %db_path.display(),
        "generating fixture"
    );

    let handle = belfry_core::db::spawn_worker(db_path.clone())?;
    let start = std::time::Instant::now();
    generate(&handle, scale).await?;
    let elapsed = start.elapsed();

    tracing::info!(elapsed_ms = elapsed.as_millis() as u64, "fixture complete");
    println!(
        "Generated {scale:?} fixture at {} in {elapsed:?}",
        db_path.display(),
    );

    Ok(())
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Belfry")
        .default_width(1100)
        .default_height(740)
        .build();

    // Phase 0 scaffold: window appears, content panes ship per spec §3.
    let placeholder = adw::StatusPage::builder()
        .icon_name("audio-headphones-symbolic")
        .title("Belfry")
        .description("specification draft — see spec.md")
        .build();

    window.set_content(Some(&placeholder));
    window.present();
}
