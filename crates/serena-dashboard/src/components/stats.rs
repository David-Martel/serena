//! Statistics Components
//!
//! Components for displaying tool usage statistics and metrics.

use leptos::*;

/// Tool usage statistics display
#[component]
pub fn ToolStats() -> impl IntoView {
    view! {
        <section class="basic-stats-section">
            <h2>"Tool Usage"</h2>
            <div id="basic-stats-display">
                <div class="loading">"Loading stats..."</div>
            </div>
        </section>
    }
}

/// Configuration display component
#[component]
pub fn ConfigDisplay() -> impl IntoView {
    view! {
        <section class="config-section">
            <h2>"Current Configuration"</h2>
            <div id="config-display">
                <div class="loading">"Loading configuration..."</div>
            </div>
        </section>
    }
}

/// Active executions queue display
#[component]
pub fn ExecutionsQueue() -> impl IntoView {
    view! {
        <section class="executions-section">
            <h2>"Executions Queue"</h2>
            <div id="active-executions-display">
                <div class="loading">"Loading executions..."</div>
            </div>
        </section>
    }
}
