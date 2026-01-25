//! Serena Dashboard - WASM Frontend
//!
//! A modern Rust/WASM dashboard for Serena using Leptos framework.
//! Replaces the vanilla JS dashboard with type-safe Rust components.

use leptos::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

mod api;
mod components;

pub use api::*;
pub use components::*;

/// Heartbeat response from the Serena backend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeartbeatResponse {
    pub status: String,
    pub agent: String,
    #[serde(default)]
    pub version: Option<String>,
}

/// Runtime state for the backend indicator
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeState {
    Connecting,
    Rust { version: String },
    Python,
    Error,
}

impl RuntimeState {
    pub fn icon(&self) -> &'static str {
        match self {
            RuntimeState::Connecting => "\u{23F3}",          // ⏳
            RuntimeState::Rust { .. } => "\u{2699}\u{FE0F}", // ⚙️
            RuntimeState::Python => "\u{1F40D}",             // 🐍
            RuntimeState::Error => "\u{26A0}\u{FE0F}",       // ⚠️
        }
    }

    pub fn text(&self) -> String {
        match self {
            RuntimeState::Connecting => "Connecting...".to_string(),
            RuntimeState::Rust { version } => format!("Rust v{}", version),
            RuntimeState::Python => "Python".to_string(),
            RuntimeState::Error => "Offline".to_string(),
        }
    }

    pub fn css_class(&self) -> &'static str {
        match self {
            RuntimeState::Connecting => "runtime-badge--connecting",
            RuntimeState::Rust { .. } => "runtime-badge--rust",
            RuntimeState::Python => "runtime-badge--python",
            RuntimeState::Error => "runtime-badge--error",
        }
    }
}

/// Main dashboard application component
#[component]
pub fn App() -> impl IntoView {
    view! {
        <div id="frame">
            <Header />
            <main class="main">
                <OverviewPage />
            </main>
        </div>
    }
}

/// Header component with logo, runtime badge, and navigation
#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="header">
            <div class="header-left">
                <div class="logo-container">
                    <img id="serena-logo" src="serena-logs.png" alt="Serena" />
                </div>
            </div>

            <RuntimeBadge />

            <nav class="header-nav">
                <ThemeToggle />
                <MenuButton />
            </nav>
        </header>
    }
}

/// Runtime badge component - shows Rust/Python backend status
#[component]
pub fn RuntimeBadge() -> impl IntoView {
    let (runtime_state, set_runtime_state) = create_signal(RuntimeState::Connecting);

    // Detect runtime on component mount
    create_effect(move |_| {
        spawn_local(async move {
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
                }
                Err(_) => {
                    set_runtime_state.set(RuntimeState::Error);
                }
            }
        });
    });

    view! {
        <div
            id="runtime-badge"
            class=move || format!("runtime-badge {}", runtime_state.get().css_class())
        >
            <span class="runtime-badge__icon">{move || runtime_state.get().icon()}</span>
            <span class="runtime-badge__text">{move || runtime_state.get().text()}</span>
        </div>
    }
}

/// Theme toggle button component
#[component]
pub fn ThemeToggle() -> impl IntoView {
    let (is_dark, set_is_dark) = create_signal(false);

    let toggle_theme = move |_| {
        set_is_dark.update(|dark| *dark = !*dark);
        // Update document theme attribute
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(body) = document.body() {
                    let theme = if is_dark.get() { "dark" } else { "light" };
                    let _ = body.set_attribute("data-theme", theme);
                }
            }
        }
    };

    view! {
        <button id="theme-toggle" class="theme-toggle" on:click=toggle_theme>
            <span id="theme-icon">
                {move || if is_dark.get() { "\u{2600}\u{FE0F}" } else { "\u{1F319}" }}
            </span>
            <span id="theme-text">{move || if is_dark.get() { "Light" } else { "Dark" }}</span>
        </button>
    }
}

/// Menu button component
#[component]
pub fn MenuButton() -> impl IntoView {
    let (menu_open, set_menu_open) = create_signal(false);

    view! {
        <div class="menu-container">
            <button
                id="menu-toggle"
                class="menu-button"
                on:click=move |_| set_menu_open.update(|open| *open = !*open)
            >
                <span>"\u{2630}"</span>
                <span>"Menu"</span>
            </button>
            <div
                id="menu-dropdown"
                class="menu-dropdown"
                style:display=move || if menu_open.get() { "block" } else { "none" }
            >
                <a href="#" data-page="overview" class="active">"Overview"</a>
                <a href="#" data-page="logs">"Logs"</a>
                <a href="#" data-page="stats">"Advanced Stats"</a>
                <hr />
                <a href="#" id="menu-shutdown">"Shutdown Server"</a>
            </div>
        </div>
    }
}

/// Overview page placeholder
#[component]
pub fn OverviewPage() -> impl IntoView {
    view! {
        <div id="page-overview" class="page-view">
            <div class="overview-container">
                <section class="config-section">
                    <h2>"Current Configuration"</h2>
                    <div id="config-display">
                        <div class="loading">"Loading configuration..."</div>
                    </div>
                </section>
            </div>
        </div>
    }
}

/// WASM entry point
#[wasm_bindgen(start)]
pub fn main() {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();

    // Initialize logging
    let _ = console_log::init_with_level(log::Level::Debug);

    log::info!("Serena Dashboard WASM initializing...");

    // Mount the app
    mount_to_body(|| view! { <App /> });

    log::info!("Serena Dashboard WASM mounted successfully");
}
