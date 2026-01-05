# MoFA Studio - Architecture Roadmap

**Analysis Date:** 2025-01-04
**Overall Grade:** C+ (68/100)
**Codebase Size:** ~7,845 lines across 27 files

---

## Executive Summary

MoFA Studio demonstrates solid architectural foundations with clear app isolation concepts and good workspace organization. However, the implementation suffers from critical code duplication (8%), monolithic files, and scalability concerns that must be addressed to support growth beyond 2-3 apps.

**Key Metrics:**
- Main Components: 4 crates (1 binary, 3 libraries)
- Apps: 2 (mofa-fm, mofa-settings)
- Shared Widgets: 7 reusable components
- Largest File: app.rs (1,120 lines) - **CRITICAL ISSUE**

---

## Critical Issues (P0 - Must Fix)

### 1. Code Duplication - Grade: D

**Impact:** 626 duplicated lines (8% of codebase)

| File | Locations | Lines Duplicated |
|------|-----------|-------------------|
| `participant_panel.rs` | shell/widgets/ & mofa-widgets/ | 246 × 2 = 492 |
| `log_panel.rs` | shell/widgets/ & mofa-widgets/ | 67 × 2 = 134 |

**Action Items:**
- [ ] Delete `mofa-studio-shell/src/widgets/participant_panel.rs`
- [ ] Delete `mofa-studio-shell/src/widgets/log_panel.rs`
- [ ] Update imports to use `mofa_widgets::` versions
- [ ] Verify functionality after removal

**Timeline:** 1-2 hours

---

### 2. app.rs Monolith - Grade: C

**Impact:** 1,120 lines in single file, handles everything

Current structure:
```rust
// mofa-studio-shell/src/app.rs
- Live design macros (~400 lines)
- Multiple widget definitions (~200 lines)
- Event handling (~200 lines)
- Tab management (~150 lines)
- Animation logic (~100 lines)
- Sidebar management (~70 lines)
```

**Target structure:**
```
mofa-studio-shell/src/app/
├── mod.rs              (50 lines)  - Module exports
├── app.rs              (200 lines) - App struct, core logic
├── dashboard.rs        (200 lines) - Dashboard widget
├── tab_manager.rs      (150 lines) - Tab management
├── navigation.rs       (150 lines) - Navigation logic
├── overlays.rs         (100 lines) - Overlay management
├── animations.rs       (100 lines) - Animation handling
└── ui.rs               (300 lines) - Live design definitions
```

**Action Items:**
- [ ] Create `app/` directory structure
- [ ] Extract dashboard widget to `dashboard.rs`
- [ ] Extract tab logic to `tab_manager.rs`
- [ ] Extract navigation to `navigation.rs`
- [ ] Extract overlays to `overlays.rs`
- [ ] Extract animations to `animations.rs`
- [ ] Move live_design to `ui.rs`
- [ ] Update all imports
- [ ] Test all functionality

**Timeline:** 1-2 weeks

---

### 3. Font Definition Duplication

**Impact:** Same fonts defined in 5+ files

Files with duplicate font definitions:
- `mofa-studio-shell/src/app.rs`
- `mofa-studio-shell/src/widgets/sidebar.rs`
- `mofa-studio-shell/src/widgets/mofa_hero.rs`
- `mofa-widgets/theme.rs` (correct location)
- Plus 5+ more files

**Action Items:**
- [ ] Audit all font definitions: `rg "FONT_REGULAR|FONT_BOLD|FONT_FAMILY" --type rust`
- [ ] Remove all except `mofa-widgets/theme.rs`
- [ ] Update imports: `use mofa_widgets::theme::{FONT_REGULAR, FONT_BOLD}`
- [ ] Verify all widgets still render correctly

**Timeline:** 2-4 hours

---

## Important Issues (P1 - Should Fix)

### 4. Shell Violates Black-Box App Principle

**Current (violates architecture):**
```rust
// mofa-studio-shell/src/app.rs
use mofa_fm::screen::MoFaFMScreen;
use mofa_settings::screen::SettingsScreen;

live_design! {
    fm_page = <MoFaFMScreen> { ... }
    settings_page = <SettingsScreen> { ... }
}
```

**Target (plugin-based):**
```rust
// Define app trait
pub trait App {
    fn name(&self) -> &str;
    fn icon(&self) -> &str;
    fn create_widget(&self, cx: &mut Cx) -> WidgetRef;
    fn on_activate(&mut self, cx: &mut Cx);
    fn on_deactivate(&mut self, cx: &mut Cx);
}

// App registry
pub struct AppRegistry {
    apps: HashMap<String, Box<dyn App>>,
}
```

**Action Items:**
- [ ] Design `App` trait in shared location
- [ ] Create `AppRegistry` in shell
- [ ] Implement trait for mofa-fm
- [ ] Implement trait for mofa-settings
- [ ] Update shell to use registry
- [ ] Remove direct app imports from shell

**Timeline:** 3-4 weeks

---

### 5. Hardcoded App Limits

**Current Problem:**
```rust
// Hardcoded 20 app slots
app1_btn = <SidebarMenuButton> { text: "App 1" }
app2_btn = <SidebarMenuButton> { text: "App 2" }
...
app20_btn = <SidebarMenuButton> { text: "App 20" }
```

**Target (dynamic apps):**
```rust
// Data-driven app list
struct AppState {
    apps: Vec<AppEntry>,
    visible_count: usize,
    show_more: bool,
}

struct AppEntry {
    id: String,
    name: String,
    icon: String,
}
```

**Action Items:**
- [ ] Create `AppState` struct to hold app list
- [ ] Create `AppEntry` data model
- [ ] Implement dynamic button generation
- [ ] Update sidebar to render from data
- [ ] Remove hardcoded button definitions
- [ ] Test with varying numbers of apps

**Timeline:** 2-3 weeks

---

### 6. Centralized State Management ⚠️ REVISED

**Status:** Architecture analysis completed - traditional Redux-style stores are **NOT feasible** in Makepad.

**Finding:** Makepad uses component-owned state model (similar to Flutter, not React). Centralized stores with `Arc<Mutex<T>>` conflict with Makepad's runtime borrow checker.

**Solution:** Shell coordinator pattern instead of global store.

**Updated Target (shell coordinator):**
```rust
// Shell owns all shared state
impl App {
    #[rust]
    app_state: AppState,  // Single source of truth
}

pub struct AppState {
    pub dark_mode: bool,
    pub providers: Vec<Provider>,
    pub active_dataflow: bool,
}

// Shell broadcasts changes via WidgetRef methods
impl App {
    fn notify_dark_mode_change(&mut self, cx: &mut Cx) {
        self.ui.mo_fa_fmscreen(ids!(fm_page))
            .on_dark_mode_change(cx, self.app_state.dark_mode);
        self.ui.settings_screen(ids!(settings_page))
            .on_dark_mode_change(cx, self.app_state.dark_mode);
    }
}
```

**Revised Action Items:**
- [x] ~~Design state store architecture~~ → Use shell coordinator
- [x] ~~Implement `Store` type~~ → Not compatible with Makepad
- [x] ~~Define action types~~ → Use WidgetRef methods instead
- [ ] Add `AppState` struct to `App`
- [ ] Add `StateDependencies` to `MofaApp` trait
- [ ] Implement state change notifications
- [x] ~~Add middleware pattern~~ → Not needed with coordinator

**Timeline:** 1-2 weeks (down from 4-6 weeks)

**See Also:** [STATE_MANAGEMENT_ANALYSIS.md](./STATE_MANAGEMENT_ANALYSIS.md) - Complete architecture analysis with code examples

---

## Nice to Have (P2)

### 7. Widget Library

**Missing Components:**
- Button variants (primary, secondary, icon, text)
- Input components (text, number, password, select)
- Layout widgets (stack, grid, card, list)
- Data widgets (table, tree, chart)

**Action Items:**
- [ ] Design component API
- [ ] Implement button variants
- [ ] Implement input components
- [ ] Create layout widgets
- [ ] Add component documentation
- [ ] Create component showcase

**Timeline:** 6-8 weeks

---

### 8. Testing Infrastructure

**Current:** No tests at all (Grade: F)

**Target:**
```rust
// Widget unit tests
#[cfg(test)]
mod tests {
    #[test]
    fn test_sidebar_selection() {
        let mut sidebar = Sidebar::new(cx);
        sidebar.handle_click(cx, "app1");
        assert_eq!(sidebar.selection(), Some("app1"));
    }
}

// Visual regression tests
#[test]
    fn visual_regression_sidebar() {
        let screenshot = widget.render(cx);
        assert_visual_match!(screenshot, "baseline/sidebar.png");
    }
}
```

**Action Items:**
- [ ] Set up test framework
- [ ] Write widget unit tests
- [ ] Create visual regression system
- [ ] Add integration tests
- [ ] Set up CI/CD testing

**Timeline:** Ongoing, start in 2-3 weeks

---

### 9. Documentation

**Current:** Good architecture doc, no API docs

**Target:**
- API documentation for all public types
- Architecture decision records (ADRs)
- Widget usage examples
- Design system documentation
- Contributing guidelines

**Action Items:**
- [ ] Add rustdoc comments
- [ ] Create ADR template
- [ ] Write widget examples
- [ ] Document design system
- [ ] Create contributing guide

**Timeline:** 3-4 weeks

---

## Detailed Grades by Category

| Category | Grade | Score | Notes |
|----------|-------|-------|-------|
| **Project Structure** | B+ | 85/100 | Good organization, needs core crate |
| **Code Organization** | B | 80/100 | Apps are good, shell needs refactoring |
| **Dependencies** | C+ | 70/100 | Too much coupling, violations |
| **Code Duplication** | D | 50/100 | **8% duplication** |
| **Scalability** | C- | 60/100 | Won't scale beyond 5 apps |
| **Widget Reusability** | B- | 75/100 | Good foundation, needs library |
| **State Management** | C+ | 70/100 | Fragmented, needs centralization |
| **Live Design Patterns** | B | 80/100 | Good usage, large blocks |
| **Testing** | F | 0/100 | No tests at all |
| **Documentation** | C | 60/100 | Good arch doc, no API docs |

**Overall: C+ (68/100)**

---

## Scalability Analysis

### Current State
- **2 apps:** Manageable
- **5 apps:** Becomes difficult (app.rs complexity)
- **10+ apps:** Will break without refactoring

### Bottlenecks
1. Hardcoded app slots (exactly 20)
2. Manual event handling in app.rs
3. No app lifecycle management
4. Direct widget instantiation in shell

### With Improvements
After implementing P0-P1 items:
- **10 apps:** Fully supported
- **20+ apps:** Scalable with data-driven approach
- **Dynamic apps:** Possible with plugin system

---

## Architecture Principles to Follow

### 1. Black-Box Apps
✅ **Principle:** Shell must NOT know about app-internal widgets

❌ **Current Violation:**
```rust
use mofa_fm::screen::MoFaFMScreen;
fm_page = <MoFaFMScreen> { ... }
```

✅ **Target:**
```rust
let app = registry.get_app("mofa-fm");
let widget = app.create_widget(cx);
```

### 2. Single Responsibility
Each module should have one reason to change:
- `app.rs` → Application lifecycle
- `dashboard.rs` → Main UI layout
- `tab_manager.rs` → Tab state
- `navigation.rs` → Navigation logic

### 3. DRY (Don't Repeat Yourself)
- Zero tolerance for duplicate code
- Widget library for reusable components
- Centralized theme and styling

### 4. Data-Driven UI
- UI generated from data, not hardcoded
- Dynamic app registration
- Configurable layouts

---

## Migration Path

### Phase 1: Quick Wins (Week 1-2)
1. Remove duplicate widgets
2. Consolidate font definitions
3. Add basic documentation

**Success Criteria:**
- [ ] 0 duplicate files
- [ ] Single source of truth for fonts
- [ ] README updated

### Phase 2: Refactoring (Week 3-6)
1. Break up app.rs monolith
2. Implement App trait
3. Create AppRegistry

**Success Criteria:**
- [ ] No file > 500 lines
- [ ] Apps register dynamically
- [ ] Shell doesn't import app types

### Phase 3: State Management (Week 3-4)
1. Add `AppState` struct to shell
2. Implement shell coordinator pattern
3. Add state dependencies to apps
4. Implement state change notifications

**Success Criteria:**
- [x] Single source of truth for state (shell-owned)
- [ ] Automatic UI updates (via WidgetRef methods)
- [x] State persists across restarts (file-based)

### Phase 4: Scale Up (Week 13-20)
1. Widget library
2. Testing infrastructure
3. Documentation
4. Performance optimization

**Success Criteria:**
- [ ] 10+ reusable widgets
- [ ] 80%+ test coverage
- [ ] Complete API docs
- [ ] No performance regressions

---

## Strengths to Preserve

✅ **Workspace dependency management**
```toml
[workspace.dependencies]
makepad-widgets = { git = "...", rev = "..." }
```

✅ **Optional app features**
```toml
[features]
default = ["mofa-fm", "mofa-settings"]
mofa-fm = ["dep:mofa-fm"]
```

✅ **Theme centralization** (mofa-widgets/theme.rs)
```rust
pub FONT_REGULAR = { ... }
pub DARK_BG = #f5f7fa
```

✅ **Data model isolation** (mofa-settings/src/data/)
```rust
pub struct Provider { ... }
pub struct Preferences { ... }
```

✅ **Clean audio management** (mofa-fm/src/audio.rs)
```rust
pub struct AudioManager { ... }
```

✅ **Consistent live_design patterns**
```rust
live_design! {
    pub MyWidget = {{MyWidget}} { ... }
}
```

---

## Risk Assessment

### High Risk Items
1. **app.rs Refactoring** - Could introduce regressions
   - Mitigation: Comprehensive testing, incremental changes

2. **App Trait Implementation** - Breaking change for existing apps
   - Mitigation: Feature flags, backward compatibility

3. **State Management** - Complex, could introduce bugs
   - Mitigation: Prototype first, gradual migration

### Medium Risk Items
1. **Widget Library** - Time-consuming
   - Mitigation: Start with most-needed components

2. **Testing Infrastructure** - Requires tooling
   - Mitigation: Use existing Rust testing frameworks

---

## Success Metrics

### Code Quality
- [ ] Maximum file size: 500 lines
- [ ] Code duplication: < 1%
- [ ] Test coverage: > 70%
- [ ] Documentation: > 80%

### Architecture
- [ ] Shell doesn't import app types
- [ ] Apps register dynamically
- [ ] Single source of truth for state
- [ ] Zero duplicate widgets

### Scalability
- [ ] Supports 20+ apps
- [ ] Adds new app < 1 day
- [ ] Plugin loading possible
- [ ] No performance regression

---

## Recommended Tooling

### For Refactoring
- **ripgrep (rg)**: Code search
- **fd**: Fast file search
- **tokei**: Code metrics

### For Quality
- **cargo-outdated**: Dependency updates
- **cargo-audit**: Security vulnerabilities
- **cargo-clippy**: Linting

### For Documentation
- **cargo doc**: API docs
- **mdbook**: Documentation site

### For Testing
- **cargo nextest**: Test runner
- **criterion**: Benchmarks

---

## Next Steps

### This Week
1. **Remove duplicate widgets** (2 hours)
2. **Consolidate fonts** (2 hours)
3. **Create this roadmap** (done)

### Next 2 Weeks
1. **Break up app.rs** into modules
2. **Add basic testing infrastructure**
3. **Update README** with architecture

### Next Month
1. **Implement App trait**
2. **Create AppRegistry**
3. **Design state store**

---

## Conclusion

MoFA Studio has a solid foundation but requires focused refactoring to scale beyond 2-3 apps. The P0 issues (code duplication, monolithic app.rs, font duplication) are quick wins that will immediately improve code quality. The P1 issues (app plugin system, state management) are essential for long-term scalability.

**Recommended Priority:**
1. P0 (Must Fix) → Complete in 2 weeks
2. P1 (Should Fix) → Complete in 2 months
3. P2 (Nice to Have) → Complete in 6 months

With these improvements, MoFA Studio will have a robust, scalable architecture supporting 20+ apps while maintaining code quality and developer productivity.

---

---

## Cross-References

This roadmap is part of a multi-document analysis:

| Document | Focus | Best For |
|----------|-------|----------|
| **roadmap-claude.md** | Architectural evidence | Understanding WHY problems exist |
| **roadmap-m2.md** | Tactical bug fixes | Quick wins and immediate fixes |
| **roadmap-glm.md** (this) | Strategic planning | Long-term roadmap with grades |
| **STATE_MANAGEMENT_ANALYSIS.md** | State management deep dive | Makepad architecture & patterns |

See also:
- [CHECKLIST.md](./CHECKLIST.md) - Master consolidated checklist
- [APP_DEVELOPMENT_GUIDE.md](./APP_DEVELOPMENT_GUIDE.md) - Contributor guide

---

**Generated:** 2025-01-04
**Analyzer:** Claude (Architectural Review Agent)
**Method:** Static code analysis, dependency graph analysis, pattern recognition
