use leptos::*;
use leptos_router::*;

#[component]
pub fn Sidebar() -> impl IntoView {
    view! {
        <nav class="sidebar">
            <div class="sidebar-header">
                <h2 class="sidebar-title">
                    <span class="sidebar-icon" data-sw-icon="🗡️">
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M12 3 L5 6 V11 C5 15.5 8 19.5 12 21 C16 19.5 19 15.5 19 11 V6 Z"/>
                            <path d="M9 11.5 L11 13.5 L15 9"/>
                        </svg>
                    </span>
                    " Watchtower"
                </h2>
                <span class="sidebar-version">"v2 • Rust"</span>
            </div>
            <ul class="sidebar-nav">
                <li><A href="/" class="nav-link" exact=true>
                    <span class="nav-icon" data-sw-icon="🌑">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                            <rect x="3.5" y="3.5" width="7" height="7" rx="1.2"/>
                            <rect x="13.5" y="3.5" width="7" height="7" rx="1.2"/>
                            <rect x="3.5" y="13.5" width="7" height="7" rx="1.2"/>
                            <rect x="13.5" y="13.5" width="7" height="7" rx="1.2"/>
                        </svg>
                    </span>
                    " Dashboard"
                </A></li>
                <li><A href="/scan" class="nav-link">
                    <span class="nav-icon" data-sw-icon="⚡">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                            <circle cx="10.5" cy="10.5" r="6.5"/>
                            <line x1="20" y1="20" x2="15.3" y2="15.3"/>
                        </svg>
                    </span>
                    " New Scan"
                </A></li>
                <li><A href="/scans" class="nav-link">
                    <span class="nav-icon" data-sw-icon="📜">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                            <circle cx="12" cy="12" r="8.5"/>
                            <polyline points="12 7 12 12 16 14"/>
                        </svg>
                    </span>
                    " Scan History"
                </A></li>
                <li><A href="/schedules" class="nav-link">
                    <span class="nav-icon" data-sw-icon="🌀">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                            <circle cx="12" cy="12" r="8.5"/>
                            <path d="M12 7 A5 5 0 1 1 7 12"/>
                            <polyline points="7 8 7 12 11 12"/>
                        </svg>
                    </span>
                    " Schedules"
                </A></li>
                <li><A href="/tools" class="nav-link">
                    <span class="nav-icon" data-sw-icon="🚀">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                            <rect x="3" y="4" width="18" height="16" rx="2"/>
                            <polyline points="7 9 10 12 7 15"/>
                            <line x1="12" y1="15" x2="16" y2="15"/>
                        </svg>
                    </span>
                    " Tools"
                </A></li>
                <li><A href="/reports" class="nav-link">
                    <span class="nav-icon" data-sw-icon="📡">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/>
                            <polyline points="14 3 14 8 19 8"/>
                            <line x1="9" y1="13" x2="15" y2="13"/>
                            <line x1="9" y1="17" x2="15" y2="17"/>
                        </svg>
                    </span>
                    " Reports"
                </A></li>
                <li><A href="/settings" class="nav-link">
                    <span class="nav-icon" data-sw-icon="🔮">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                            <line x1="4" y1="6" x2="20" y2="6"/><circle cx="14" cy="6" r="2"/>
                            <line x1="4" y1="12" x2="20" y2="12"/><circle cx="8" cy="12" r="2"/>
                            <line x1="4" y1="18" x2="20" y2="18"/><circle cx="16" cy="18" r="2"/>
                        </svg>
                    </span>
                    " Settings"
                </A></li>
            </ul>
            <div class="sidebar-footer">
                <p class="sidebar-quote">"\"I find your lack of security disturbing.\""</p>
            </div>
        </nav>
    }
}
