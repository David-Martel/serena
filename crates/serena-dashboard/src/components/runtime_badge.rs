//! Runtime Badge Component
//!
//! Displays the current backend runtime (Rust or Python) with version info.
//! Features:
//! - Automatic detection via /heartbeat endpoint
//! - Animated transitions between states
//! - Rust orange (#F74C00) accent for Rust backend
//! - Python blue (#3776AB) accent for Python backend

use crate::{api, RuntimeState};
use leptos::*;

/// Standalone runtime badge that can be used anywhere
#[component]
pub fn RuntimeBadgeStandalone() -> impl IntoView {
    let (runtime_state, set_runtime_state) = create_signal(RuntimeState::Connecting);
    let (is_loaded, set_is_loaded) = create_signal(false);

    // Detect runtime on mount
    create_effect(move |_| {
        spawn_local(async move {
            // Small delay to show connecting state
            gloo_timers::future::TimeoutFuture::new(500).await;

            match api::fetch_heartbeat().await {
                Ok(response) => {
                    let state = if response.agent == "serena-rust" {
                        RuntimeState::Rust {
                            version: response.version.unwrap_or_else(|| "0.1.0".to_string()),
                        }
                    } else if response.agent == "serena-python" || response.status == "ok" {
                        RuntimeState::Python
                    } else {
                        RuntimeState::Error
                    };
                    set_runtime_state.set(state);
                    set_is_loaded.set(true);
                }
                Err(e) => {
                    log::error!("Heartbeat failed: {:?}", e);
                    set_runtime_state.set(RuntimeState::Error);
                    set_is_loaded.set(true);
                }
            }
        });
    });

    // CSS classes based on state
    let badge_class = move || {
        let base = "runtime-badge";
        let state_class = runtime_state.get().css_class();
        let loaded_class = if is_loaded.get() {
            "runtime-badge--loaded"
        } else {
            ""
        };
        format!("{} {} {}", base, state_class, loaded_class)
    };

    view! {
        <div id="runtime-badge" class=badge_class>
            <span class="runtime-badge__icon">{move || runtime_state.get().icon()}</span>
            <span class="runtime-badge__text">{move || runtime_state.get().text()}</span>
        </div>
    }
}

/// Inline CSS for the runtime badge (can be included in WASM bundle)
pub const RUNTIME_BADGE_CSS: &str = r#"
/* Runtime Badge - Industrial Precision Style */
.runtime-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-radius: 20px;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', 'Consolas', monospace;
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.02em;
    border: 1.5px solid transparent;
    background: var(--bg-secondary, #ffffff);
    color: var(--text-secondary, #333333);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    user-select: none;
    flex-shrink: 0;
}

.runtime-badge__icon {
    font-size: 14px;
    line-height: 1;
}

.runtime-badge__text {
    white-space: nowrap;
}

/* Connecting State */
.runtime-badge--connecting {
    border-color: var(--border-color, #ddd);
    animation: badge-pulse 2s ease-in-out infinite;
}

@keyframes badge-pulse {
    0%, 100% { opacity: 0.6; }
    50% { opacity: 1; }
}

/* Rust Active State - Orange accent */
.runtime-badge--rust {
    border-color: #F74C00;
    background: linear-gradient(135deg, rgba(247, 76, 0, 0.08) 0%, rgba(247, 76, 0, 0.03) 100%);
    color: var(--text-primary, #000000);
    animation: badge-connected 0.5s ease-out;
}

.runtime-badge--rust .runtime-badge__icon {
    color: #F74C00;
}

@keyframes badge-connected {
    0% { transform: scale(0.9); opacity: 0; }
    60% { transform: scale(1.03); }
    100% { transform: scale(1); opacity: 1; }
}

/* Python Fallback State - Blue accent */
.runtime-badge--python {
    border-color: #3776AB;
    background: linear-gradient(135deg, rgba(55, 118, 171, 0.08) 0%, rgba(55, 118, 171, 0.03) 100%);
    color: var(--text-primary, #000000);
}

.runtime-badge--python .runtime-badge__icon {
    color: #3776AB;
}

/* Error State - Red accent */
.runtime-badge--error {
    border-color: #DC3545;
    background: linear-gradient(135deg, rgba(220, 53, 69, 0.08) 0%, rgba(220, 53, 69, 0.03) 100%);
    color: var(--text-secondary, #666666);
}

.runtime-badge--error .runtime-badge__icon {
    color: #DC3545;
}

/* Hover effect */
.runtime-badge:hover {
    transform: translateY(-1px);
    box-shadow: 0 3px 10px rgba(0, 0, 0, 0.1);
}

/* Dark theme */
[data-theme="dark"] .runtime-badge--rust {
    background: linear-gradient(135deg, rgba(247, 76, 0, 0.15) 0%, rgba(247, 76, 0, 0.06) 100%);
    box-shadow: 0 0 12px rgba(247, 76, 0, 0.15);
}

[data-theme="dark"] .runtime-badge--python {
    background: linear-gradient(135deg, rgba(55, 118, 171, 0.15) 0%, rgba(55, 118, 171, 0.06) 100%);
    box-shadow: 0 0 12px rgba(55, 118, 171, 0.15);
}
"#;
