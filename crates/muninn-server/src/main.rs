//! `muninn-server` — a standalone HTTP host for the muninn-core API.
//!
//! Used for headless environments (home servers, CI, automation scripts)
//! where the Tauri desktop shell isn't running. The Tauri build embeds the
//! same `muninn_core::api::router` in-process instead.

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use muninn_core::api::{AppState, router};
use muninn_core::vault::Vault;
use tokio::net::TcpListener;

const DEFAULT_PORT: u16 = 9200;
const FALLBACK_SCAN: u16 = 99;

#[derive(Parser)]
#[command(name = "muninn-server", version, about = "HTTP API for a Muninn vault")]
struct Cli {
    /// Vault root directory.
    #[arg(long, env = "MUNINN_VAULT_PATH")]
    vault: PathBuf,

    /// Address to bind on. Defaults to localhost.
    #[arg(long, default_value = "127.0.0.1")]
    bind: IpAddr,

    /// Preferred port. If taken, the server scans up to `port + 99`.
    /// Use `--strict-port` to disable the fallback.
    #[arg(long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// Refuse to start if `--port` is not available.
    #[arg(long)]
    strict_port: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let vault = Vault::open(&cli.vault)
        .with_context(|| format!("failed to open vault at {}", cli.vault.display()))?;
    let state = AppState::new(Arc::new(vault));
    let app = router(state);

    let listener = bind_port(cli.bind, cli.port, cli.strict_port).await?;
    let local = listener
        .local_addr()
        .context("listener has no local address")?;

    tracing::info!(addr = %local, vault = %cli.vault.display(), "muninn-server listening");
    println!("muninn-server listening on http://{}", local);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    Ok(())
}

/// Bind `port` on `bind`; if in use and `strict` is false, try the next 99
/// ports in turn. Returns the bound `TcpListener` with whatever port the OS
/// accepted. On total exhaustion, bail with the original `EADDRINUSE`.
async fn bind_port(bind: IpAddr, port: u16, strict: bool) -> Result<TcpListener> {
    let first_addr = SocketAddr::new(bind, port);
    let first_err = match TcpListener::bind(first_addr).await {
        Ok(listener) => return Ok(listener),
        Err(e) => e,
    };

    if strict {
        return Err(anyhow::anyhow!(
            "port {port} on {bind} is unavailable ({first_err}); --strict-port is set"
        ));
    }

    tracing::warn!(
        port,
        error = %first_err,
        "preferred port in use; scanning for a fallback"
    );

    for offset in 1..=FALLBACK_SCAN {
        let candidate = match port.checked_add(offset) {
            Some(p) => p,
            None => break,
        };
        let addr = SocketAddr::new(bind, candidate);
        if let Ok(listener) = TcpListener::bind(addr).await {
            tracing::info!(port = candidate, "bound fallback port");
            return Ok(listener);
        }
    }

    Err(anyhow::anyhow!(
        "no port in {port}..={end} available on {bind} (initial error: {first_err})",
        end = port.saturating_add(FALLBACK_SCAN)
    ))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install ctrl-c handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("ctrl-c received, shutting down"),
        _ = terminate => tracing::info!("SIGTERM received, shutting down"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn fallback_picks_next_free_port() {
        let bind = IpAddr::V4(Ipv4Addr::LOCALHOST);
        // Hold the first port so `bind_port` has to fall back.
        let blocker = TcpListener::bind(SocketAddr::new(bind, 0)).await.unwrap();
        let blocked_port = blocker.local_addr().unwrap().port();

        let listener = bind_port(bind, blocked_port, false).await.unwrap();
        let chosen = listener.local_addr().unwrap().port();
        assert_ne!(chosen, blocked_port, "fallback must pick a different port");
        assert!(
            chosen > blocked_port && chosen <= blocked_port.saturating_add(FALLBACK_SCAN),
            "chose {chosen}, expected in {blocked_port}..={}",
            blocked_port.saturating_add(FALLBACK_SCAN)
        );
    }

    #[tokio::test]
    async fn strict_port_refuses_fallback() {
        let bind = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let blocker = TcpListener::bind(SocketAddr::new(bind, 0)).await.unwrap();
        let blocked_port = blocker.local_addr().unwrap().port();

        let err = bind_port(bind, blocked_port, true).await.unwrap_err();
        assert!(
            err.to_string().contains("--strict-port"),
            "unexpected error: {err}"
        );
    }
}
