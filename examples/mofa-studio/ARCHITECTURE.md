# MoFA Studio Architecture Guide

This document describes the modular architecture of MoFA Studio, a desktop application built with the Makepad UI framework. Apps are self-contained widgets that plug into a lightweight shell.

## Project Overview

**MoFA Studio** is an AI-powered desktop application for voice chat and model management, built with Rust and Makepad.

- **Version**: 0.1.0
- **Edition**: Rust 2021
- **License**: Apache-2.0
- **UI Framework**: Makepad (GPU-accelerated, immediate mode)

## Directory Structure

```
mofa-studio/
├── Cargo.toml              # Workspace configuration
├── ARCHITECTURE.md         # This file (English)
├── 架构指南.md              # Architecture guide (Chinese)
├── mofa-widgets/           # Shared reusable widgets (library)
│   ├── src/
│   │   ├── lib.rs          # Module exports and live_design registration
│   │   ├── theme.rs        # Fonts and colors
│   │   ├── participant_panel.rs  # Speaker status cards
│   │   ├── log_panel.rs    # Scrollable log display
│   │   ├── waveform_view.rs     # Audio visualization
│   │   ├── led_gauge.rs    # LED bar indicator
│   │   └── audio_player.rs # Audio playback
│   └── resources/
│       └── fonts/          # Manrope font files
├── mofa-studio-shell/      # Main shell application (binary)
│   ├── src/
│   │   ├── main.rs         # Entry point
│   │   ├── lib.rs          # SharedState definition
│   │   ├── app.rs          # Main App widget (~1,120 lines)
│   │   └── widgets/
│   │       ├── mod.rs
│   │       ├── sidebar.rs  # Navigation sidebar (~550 lines)
│   │       ├── log_panel.rs
│   │       └── participant_panel.rs
│   └── resources/
│       ├── fonts/          # Manrope font files
│       ├── icons/          # SVG icons
│       └── mofa-logo.png   # Application logo
└── apps/
    ├── mofa-fm/            # MoFA FM app (library)
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── screen.rs   # Main screen (~1,360 lines)
    │   │   ├── mofa_hero.rs # Status bar (~660 lines)
    │   │   └── audio.rs    # Audio device management
    │   └── resources/
    └── mofa-settings/      # Settings app (library)
        ├── src/
        │   ├── lib.rs
        │   ├── screen.rs   # Settings screen (~415 lines)
        │   ├── providers_panel.rs  # Provider list (~320 lines)
        │   ├── provider_view.rs    # Provider config (~640 lines)
        │   ├── add_provider_modal.rs  # Add provider dialog
        │   └── data/
        │       ├── mod.rs
        │       ├── providers.rs    # Provider data types
        │       └── preferences.rs  # User preferences
        └── resources/
            └── icons/      # Provider icons
```

## Crate Dependencies

```
mofa-studio-shell (binary)
├── makepad-widgets
├── mofa-widgets
├── mofa-fm (optional, default enabled)
├── mofa-settings (optional, default enabled)
├── dora-node-api
├── cpal (audio)
├── tokio (async runtime)
├── parking_lot (synchronization)
├── serde, serde_json (serialization)
├── dirs (user directories)
├── sysinfo (system metrics)
└── log, ctrlc

mofa-fm (library)
├── makepad-widgets
├── mofa-widgets
├── cpal
├── parking_lot
├── sysinfo
└── log

mofa-settings (library)
├── makepad-widgets
├── mofa-widgets
├── serde, serde_json
├── dirs
├── parking_lot
└── log

mofa-widgets (library)
├── makepad-widgets
├── cpal
├── parking_lot
├── crossbeam-channel
└── log
```

## Architecture Principles

### Core Principle: Black-Box Apps

Apps are self-contained widgets. The shell knows nothing about their internal structure.

**Shell responsibilities:**
- Window chrome (title bar, buttons)
- Navigation (sidebar, tab bar)
- App switching (visibility toggling)
- Widget registration

**Shell must NOT:**
- Know about app-internal widgets
- Handle app-specific events
- Store app-specific state

**App responsibilities:**
- All internal UI layout
- All internal events
- All internal state
- Own resource files

### Minimal Coupling (4 Points Only)

#### 1. Import Statement
```rust
// mofa-studio-shell/src/app.rs
use mofa_fm::screen::MoFaFMScreen;
use mofa_settings::screen::SettingsScreen;
```

#### 2. Widget Registration (Order Matters!)
```rust
impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        mofa_widgets::live_design(cx);           // Shared first
        mofa_studio_shell::widgets::sidebar::live_design(cx);
        mofa_studio_shell::widgets::log_panel::live_design(cx);
        mofa_studio_shell::widgets::participant_panel::live_design(cx);
        mofa_fm::live_design(cx);                // Then apps
        mofa_settings::live_design(cx);
    }
}
```

#### 3. Widget Instantiation
```rust
live_design! {
    content = <View> {
        flow: Overlay
        fm_page = <MoFaFMScreen> {
            width: Fill, height: Fill
            visible: true
        }
        settings_page = <SettingsScreen> {
            width: Fill, height: Fill
            visible: false
        }
    }
}
```

#### 4. Visibility Toggling
```rust
// Navigation via apply_over
self.ui.view(ids!(content.fm_page)).apply_over(cx, live!{ visible: true });
self.ui.view(ids!(content.settings_page)).apply_over(cx, live!{ visible: false });
self.ui.redraw(cx);
```

## Widget Hierarchy

```
Window (1400x900)
├── Dashboard (base layer)
│   ├── Header
│   │   ├── Hamburger Button (21x21)
│   │   ├── Logo (40x40)
│   │   ├── Title "MoFA Studio"
│   │   └── User Profile Container
│   └── Content Area
│       └── Main Content (Overlay)
│           ├── fm_page (MoFaFMScreen)
│           │   ├── MofaHero (status bar)
│           │   │   ├── Action Section (Start/Stop)
│           │   │   ├── Connection Section
│           │   │   ├── Buffer Section
│           │   │   ├── CPU Section
│           │   │   └── Memory Section
│           │   ├── Participant Container
│           │   │   ├── Student 1 Panel
│           │   │   ├── Student 2 Panel
│           │   │   └── Tutor Panel
│           │   ├── Chat Container
│           │   └── Audio Control Panel
│           ├── app_page (placeholder)
│           └── settings_page (SettingsScreen)
│               ├── ProvidersPanel (left)
│               ├── VerticalDivider
│               ├── ProviderView (right)
│               └── AddProviderModal (overlay)
├── Tab Overlay (modal layer)
│   ├── Tab Bar
│   └── Tab Content
├── Sidebar Menu Overlay (slide animation)
│   └── Sidebar
│       ├── MoFA FM Tab
│       ├── App List (1-20)
│       │   ├── Apps 1-4 (always visible)
│       │   ├── Pinned App (for Show More selection)
│       │   ├── Show More Button
│       │   └── More Apps Section (5-20, collapsible)
│       └── Settings Tab
├── User Menu Overlay
└── Sidebar Trigger Overlay (28x28)
```

## State Management

### Shell State (app.rs)
```rust
pub struct App {
    #[live] ui: WidgetRef,

    // Menu states
    #[rust] user_menu_open: bool,
    #[rust] sidebar_menu_open: bool,

    // Tab system
    #[rust] open_tabs: Vec<String>,      // "profile", "settings"
    #[rust] active_tab: Option<String>,

    // Responsive layout
    #[rust] last_window_size: DVec2,

    // Sidebar animation
    #[rust] sidebar_animating: bool,
    #[rust] sidebar_animation_start: f64,
    #[rust] sidebar_slide_in: bool,
}
```

### Sidebar State (sidebar.rs)
```rust
pub struct Sidebar {
    #[deref] view: View,
    #[rust] more_apps_visible: bool,
    #[rust] selection: Option<SidebarSelection>,
    #[rust] pinned_app_name: Option<String>,
}

pub enum SidebarSelection {
    MofaFM,
    App(usize),  // 1-20
    Settings,
}
```

### Settings State (screen.rs)
```rust
pub struct SettingsScreen {
    #[deref] view: View,
    #[rust] preferences: Option<Preferences>,
    #[rust] selected_provider_id: Option<ProviderId>,
}
```

### Shared State (lib.rs)
```rust
pub struct SharedState {
    pub buffer_fill: f64,
    pub is_connected: bool,
    pub cpu_usage: f32,
    pub memory_usage: f32,
}

pub type SharedStateRef = Arc<Mutex<SharedState>>;
```

## Animation System

### Sidebar Slide Animation
```rust
// Animation parameters
const ANIMATION_DURATION: f64 = 0.2;  // 200ms
const SIDEBAR_WIDTH: f64 = 180.0;

// Ease-out cubic easing
let eased = 1.0 - (1.0 - progress).powi(3);

// Position calculation
let x = if slide_in {
    -SIDEBAR_WIDTH * (1.0 - eased)  // -180 -> 0
} else {
    -SIDEBAR_WIDTH * eased           // 0 -> -180
};

// Apply via abs_pos
self.ui.view(ids!(sidebar_menu_overlay)).apply_over(cx, live!{
    abs_pos: (dvec2(x, 52.0))
});
```

## Theme System

### Fonts (Multi-language Support)
```rust
// All fonts support: Latin, Chinese (LXGWWenKai), Emoji (NotoColorEmoji)
FONT_REGULAR    // Normal text
FONT_MEDIUM     // Slightly bolder
FONT_SEMIBOLD   // Section headers
FONT_BOLD       // Titles
```

### Color Palette
```rust
// Background colors
DARK_BG = #f5f7fa        // Page background (light gray)
PANEL_BG = #ffffff       // Card/panel background (white)

// Accent colors
ACCENT_BLUE = #3b82f6    // Primary action
ACCENT_GREEN = #10b981   // Success/active

// Text colors
TEXT_PRIMARY = #1f2937   // Main text (dark gray)
TEXT_SECONDARY = #6b7280 // Muted text (medium gray)

// UI colors
Border: #e2e8f0, #e5e7eb
Hover: #f1f5f9
Selected: #dbeafe (light blue)
Status: Green #22c55e, Yellow #f59e0b, Red #ef4444
```

## Data Models

### Provider Configuration
```rust
pub enum ProviderType {
    OpenAi,
    DeepSeek,
    AlibabaCloud,
    Custom,
}

pub enum ProviderConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

pub struct Provider {
    pub id: ProviderId,
    pub name: String,
    pub url: String,
    pub api_key: Option<String>,
    pub provider_type: ProviderType,
    pub enabled: bool,
    pub models: Vec<String>,
    pub is_custom: bool,
    pub connection_status: ProviderConnectionStatus,
}
```

### Audio Device Info
```rust
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
}
```

## Creating a New App

### Step 1: Create Crate Structure
```
apps/my-app/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   └── screen.rs
└── resources/
    └── icons/
```

### Step 2: Define Cargo.toml
```toml
[package]
name = "my-app"
version.workspace = true
edition.workspace = true

[lib]
path = "src/lib.rs"

[dependencies]
makepad-widgets.workspace = true
mofa-widgets = { path = "../../mofa-widgets" }
```

### Step 3: Create lib.rs
```rust
mod screen;
pub use screen::*;

use makepad_widgets::*;

pub fn live_design(cx: &mut Cx) {
    screen::live_design(cx);
}
```

### Step 4: Create screen.rs
```rust
use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    use mofa_widgets::theme::*;

    pub MyAppScreen = {{MyAppScreen}} {
        width: Fill, height: Fill
        flow: Down
        show_bg: true
        draw_bg: { color: (DARK_BG) }

        // Your UI here
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct MyAppScreen {
    #[deref]
    view: View,

    #[rust]
    my_state: bool,
}

impl Widget for MyAppScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        // Handle events
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}
```

### Step 5: Register in Shell

Add to `mofa-studio-shell/Cargo.toml`:
```toml
[features]
default = ["mofa-fm", "mofa-settings", "my-app"]
my-app = ["dep:my-app"]

[dependencies]
my-app = { path = "../apps/my-app", optional = true }
```

Add to `mofa-studio-shell/src/app.rs`:
```rust
use my_app::screen::MyAppScreen;

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        // ... existing ...
        my_app::live_design(cx);
    }
}
```

Add to live_design:
```rust
content = <View> {
    flow: Overlay
    fm_page = <MoFaFMScreen> { ... }
    my_app_page = <MyAppScreen> {
        width: Fill, height: Fill
        visible: false
    }
    settings_page = <SettingsScreen> { ... }
}
```

### Step 6: Add Navigation

Add sidebar button in `widgets/sidebar.rs`:
```rust
my_app_tab = <SidebarMenuButton> {
    text: "My App"
    draw_icon: {
        svg_file: dep("crate://self/resources/icons/my-app.svg")
    }
}
```

Add click handler in app.rs:
```rust
if self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.my_app_tab)).clicked(&actions) {
    self.sidebar_menu_open = false;
    self.start_sidebar_slide_out(cx);
    // Toggle visibility...
}
```

## Event Handling Patterns

### Hover Events (Important!)
Hover events (`FingerHoverIn`/`FingerHoverOut`) must be handled **before** the early return for `Event::Actions`:

```rust
fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
    self.view.handle_event(cx, event, scope);

    // Handle hover BEFORE extracting actions
    for item in &items {
        match event.hits(cx, item.area()) {
            Hit::FingerHoverIn(_) => { /* hover effect */ }
            Hit::FingerHoverOut(_) => { /* reset */ }
            _ => {}
        }
    }

    // THEN extract actions
    let actions = match event {
        Event::Actions(actions) => actions.as_slice(),
        _ => return,  // Early return AFTER hover handling
    };

    // Handle clicks
}
```

### Button Clicks
```rust
if self.view.button(ids!(my_button)).clicked(actions) {
    // Handle click
}
```

### View Finger Events
```rust
if self.view.view(ids!(my_view)).finger_up(actions).is_some() {
    // Handle finger up on view
}
```

## Best Practices

1. **Keep apps self-contained**: All UI, state, and events within the app
2. **Use shared widgets for consistency**: Theme, common patterns
3. **Minimize shell coupling**: Only the 4 required points
4. **Register in order**: Dependencies before dependents
5. **Use `apply_over` for visibility**: More reliable than `set_visible()`
6. **Handle hover before early return**: Check event.hits() before extracting actions
7. **Restore selection state**: When sidebar reopens, call `restore_selection_state()`

## Troubleshooting

### Widget Not Rendering
- Check `live_design(cx)` is called in correct order
- Verify import paths in live_design macro
- Ensure `visible: true` is set

### Hover Not Working
- Ensure hover handling is before the `Event::Actions` early return
- Use `Hit::FingerHoverIn` / `Hit::FingerHoverOut`, not `MouseHoverIn`

### Events Not Firing
- Ensure `handle_event` calls `self.view.handle_event(...)`
- Verify button IDs match between live_design and code

### Animation Not Smooth
- Call `self.ui.redraw(cx)` at end of animation update
- Check `sidebar_animating` flag is being checked on every event

## Statistics

- **Total Crates**: 5 (1 binary, 4 libraries)
- **Total Lines**: ~6,000 lines of Rust
- **Apps**: 2 (mofa-fm, mofa-settings)
- **Shared Widgets**: 7 reusable components
- **Default Window**: 1400x900 pixels
- **Sidebar Width**: 180 pixels
