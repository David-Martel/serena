use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use serena::App;

/// Serena - AI-powered coding assistant with LSP and MCP support
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Transport mode: stdio, http, or sse
    #[arg(short, long, default_value = "stdio")]
    transport: String,

    /// HTTP server port (for http/sse transports)
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Project directory to activate
    #[arg(short = 'P', long)]
    project: Option<PathBuf>,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Operating mode (planning, editing, interactive, one-shot)
    #[arg(short, long)]
    mode: Option<String>,

    /// Context to use (desktop-app, agent, ide-assistant)
    #[arg(long)]
    context: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Set up tracing/logging
    setup_tracing(&args)?;

    info!("Starting Serena v{}", env!("CARGO_PKG_VERSION"));
    info!("Transport: {}, Port: {}", args.transport, args.port);

    // Create and initialize application
    let mut app = App::new(args.config, args.project).await?;

    // Apply mode and context if specified
    if let Some(mode) = args.mode {
        app.set_mode(&mode)?;
    }
    if let Some(context) = args.context {
        app.set_context(&context)?;
    }

    // Clone Arc references for cleanup after server consumption
    let lsp_manager = std::sync::Arc::clone(app.lsp_manager());
    let config_service = std::sync::Arc::clone(app.config_service());

    // Create shutdown signal future
    let shutdown_signal = async {
        let ctrl_c = signal::ctrl_c();

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Received Ctrl+C, initiating shutdown...");
            }
            _ = terminate => {
                info!("Received SIGTERM, initiating shutdown...");
            }
        }
    };

    // Run the server based on transport type with graceful shutdown
    let server_result = match args.transport.as_str() {
        "stdio" => {
            info!("Starting MCP server on stdio transport");
            tokio::select! {
                biased;
                result = app.run_stdio() => result,
                _ = shutdown_signal => {
                    info!("Shutdown signal received during stdio operation");
                    Ok(())
                }
            }
        }
        "http" => {
            info!("Starting MCP server on HTTP transport (port {})", args.port);
            tokio::select! {
                biased;
                result = app.run_http(args.port) => result,
                _ = shutdown_signal => {
                    info!("Shutdown signal received during HTTP operation");
                    Ok(())
                }
            }
        }
        "sse" => {
            info!("Starting MCP server on SSE transport (port {})", args.port);
            tokio::select! {
                biased;
                result = app.run_sse(args.port) => result,
                _ = shutdown_signal => {
                    info!("Shutdown signal received during SSE operation");
                    Ok(())
                }
            }
        }
        _ => {
            anyhow::bail!("Invalid transport type: {}", args.transport);
        }
    };

    // Graceful cleanup using cloned references
    info!("Performing graceful cleanup...");

    // Stop all LSP servers
    lsp_manager.stop_all_servers().await;
    info!("LSP servers stopped");

    // Save configuration state
    if let Err(e) = config_service.save() {
        warn!("Failed to save configuration state: {}", e);
    } else {
        info!("Configuration state saved");
    }

    info!("Serena shutdown complete");
    server_result
}

/// Set up tracing subscriber with appropriate log level
fn setup_tracing(args: &Args) -> Result<()> {
    let log_level = if args.verbose {
        "debug"
    } else {
        &args.log_level
    };

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(log_level))
        .unwrap_or_else(|_| {
            warn!("Invalid log level '{}', defaulting to 'info'", log_level);
            EnvFilter::new("info")
        });

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(env_filter)
        .init();

    Ok(())
}
