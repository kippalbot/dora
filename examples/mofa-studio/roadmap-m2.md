# MoFA Studio Roadmap M2 - Architecture Improvements

## Executive Summary

This document outlines architectural improvements for MoFA Studio based on code review findings. The focus is on addressing technical debt, improving maintainability, and establishing consistent patterns across the codebase.

---

## Critical Issues (High Priority)

### 1. Code Duplication in Sidebar

**Location:** `apps/mofa-sidebar/src/sidebar.rs` (~700 lines)

**Issue:** The sidebar contains extensive duplicated code patterns across multiple action handlers. Similar patterns exist in `apps/mofa-fm/src/screen.rs` and `apps/mofa-settings/src/screen.rs`.

**Impact:**
- Maintenance burden increased O(n) with each new action
- Inconsistent behavior between modules
- High risk of bugs when modifying shared patterns

**Example Pattern (repeated ~15 times):**
```rust
match event.hits(cx, widget.area()) {
    Hit::FingerUp(_) => {
        // Nearly identical boilerplate for visibility toggling
    }
    _ => {}
}
```

**Solution:**
```rust
// Extract to a helper trait or macro
trait ClickHandler {
    fn handle_click(&mut self, cx: &mut Cx, action: Action);
}

impl ClickHandler for Sidebar {
    fn handle_click(&mut self, cx: &mut Cx, action: Action) {
        match action {
            Action::ToggleView(view_id) => {
                self.views.iter_mut().for_each(|(id, v)| {
                    v.set_visible(cx, id == view_id);
                });
            }
        }
    }
}
```

### 2. Timer Cleanup Pattern

**Location:** `apps/mofa-fm/src/screen.rs`

**Issue:**
```rust
#[rust]
aec_timer: Timer,  // Timer started but never stopped!
```

**Problem:** The timer is stored in `Cx`'s internal registry, NOT in the widget. The widget only holds the timer ID (u64). A `Drop` impl won't work because `Cx` is not available in Drop context.

**Impact:** Timers continue firing after widget is hidden/destroyed:
- Unnecessary CPU usage
- Potential panic on timer callback with dead widget state
- Timer registry accumulation

**Solution - Shell-Managed Cleanup:**
The widget provides a cleanup method, and the parent shell calls it when the widget becomes hidden:

```rust
// In MoFaFMScreen (widget):
pub fn stop_timers(&self, cx: &mut Cx) {
    if let Some(mut inner) = self.borrow_mut() {
        cx.stop_timer(inner.audio_timer);
        cx.stop_timer(inner.aec_timer);
    }
}

// In app.rs (shell) - called when FM page becomes hidden:
self.ui.mo_fa_fmscreen(ids!(body...fm_page)).stop_timers(cx);
```

**Implementation:** ✅ COMPLETE
- `stop_timers()` implemented in `apps/mofa-fm/src/screen.rs:1349`
- Called in `mofa-studio-shell/src/app.rs` at 3 locations:
  - Line 753: When closing tab overlay to show settings
  - Line 793: When switching to settings tab
  - Line 994: When overlay becomes visible (covers FM page)

### 3. Inconsistent Naming Conventions

**Issue:** Mixed naming patterns across codebase:

| Pattern | Example | Count |
|---------|---------|-------|
| snake_case | `aec_enabled`, `api_key` | Majority |
| camelCase | `audioPanel`, `startView` | ~20% |
| kebab-case | `add-provider-button` | ~5% |

**Solution:** Establish and enforce naming standards:
- **Rust:** `snake_case` for all identifiers
- **Live IDs:** `snake_case` with underscores
- **UI Labels:** `Title Case` for display text only

---

## Medium Priority Issues

### 4. Debug Println! Statements in Production Code

**Locations:**
- `apps/mofa-sidebar/src/sidebar.rs` - Multiple debug logs
- `apps/mofa-fm/src/screen.rs` - Console output in event handlers
- `apps/mofa-settings/src/screen.rs` - Action debugging

**Impact:**
- Console spam in production
- Performance overhead (string formatting)
- Information leakage in production environments

**Solution:**
```rust
// Use conditional compilation
#[cfg(debug_assertions)]
macro_rules! debug_log {
    ($($arg:tt)*) => { println!($($arg)*) };
}

#[cfg(not(debug_assertions))]
macro_rules! debug_log {
    ($($arg:tt)*) => { };
}
```

### 5. Mixed State Management: Event + Polling Hybrid

**Issue:** State is managed through both event-driven callbacks and polling timers:

```rust
// Event-driven (good)
fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
    // React to specific events
}

// Polling (expensive)
fn update_aec_blink(&mut self, cx: &mut Cx) {
    self.aec_blink_counter += 1;  // Runs every 50ms
}
```

**Impact:**
- Inconsistent state updates (race conditions possible)
- Wasted CPU cycles on idle components
- Harder to debug state transitions

**Solution:** Prefer event-driven where possible. For animations:
```rust
// Use animation callbacks instead of timers
cx.animate::<AecBlinkAnimation>(widget_id, duration, easing);
```

### 6. Tight Coupling Between Widget and View Hierarchy

**Issue:** Widgets directly reference deeply nested IDs:

```rust
self.view.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page.mofa_hero.action_section.start_view))
```

**Impact:**
- Refactoring breaking changes (moving a widget breaks 5+ files)
- Poor separation of concerns
- Testing becomes difficult

**Solution:** Use widget composition patterns:
```rust
// Instead of deep nesting, pass parent reference
MofaHero::new(parent: &dyn HeroParent) {
    parent.on_start(|| self.handle_start());
    parent.on_stop(|| self.handle_stop());
}
```

---

## Quick Wins (Can Implement Immediately)

### 1. Remove All Debug Logs

**Action:** Search and remove `println!` statements in:
- `apps/mofa-sidebar/src/sidebar.rs`
- `apps/mofa-fm/src/screen.rs`
- `apps/mofa-settings/src/screen.rs`
- `mofa-studio-shell/src/app.rs`

**Estimated Time:** 30 minutes

### 2. Delete Duplicate Files

**Action:** Remove identified duplicates:
```bash
# Delete duplicate participant panel
rm apps/mofa-fm/src/participant_panel.rs

# Verify no imports reference deleted file
grep -r "participant_panel" apps/mofa-fm/src/
```

**Estimated Time:** 10 minutes

### 3. Apply Theme Constants

**Action:** Replace hardcoded colors with theme constants:

```rust
// Before
draw_bg: { color: #dbeafe }

// After (using existing theme)
draw_bg: { color: THEME.color.surface.highlight }
```

**Estimated Time:** 2 hours (across all files)

---

## Low Priority Enhancements

### 7. Dynamic Provider Generation

**Current:** Hardcoded provider list in `providers_panel.rs`

```rust
openai_item = <ProviderItemBg> { ... }
deepseek_item = <ProviderItemBg> { ... }
alibaba_item = <ProviderItemBg> { ... }
```

**Problem:** Adding new provider requires code change

**Solution:**
```rust
// Dynamic generation from preferences
impl ProvidersPanel {
    fn rebuild_provider_list(&mut self, cx: &mut Cx, providers: &[Provider]) {
        self.list_container = View::new();
        for provider in providers {
            let item = ProviderItemBg::new()
                .with_provider(provider)
                .build(cx);
            self.list_container.add_child(cx, item);
        }
    }
}
```

### 8. Unit Tests for Widget Behavior

**Current State:** No unit tests for widget behavior

**Target:** 70% coverage on core logic

**Key Test Cases:**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_provider_selection_highlight() {
        let panel = ProvidersPanel::new();
        panel.select_provider("openai");
        assert_eq!(panel.selected_provider(), Some("openai"));
    }
}
```

---

## Implementation Order

### Phase 1: Cleanup (Week 1)
1. [ ] Remove all debug println! statements
2. [ ] Delete duplicate participant_panel.rs
3. [ ] Standardize naming conventions (snake_case)
4. [ ] Fix hover effect on provider buttons (not working)

### Phase 2: Architecture (Week 2)
5. [ ] Extract ClickHandler trait for sidebar actions
6. [x] Implement proper Timer cleanup (Shell-managed pattern) - DONE
7. [ ] Add conditional debug logging macro
8. [ ] Refactor deep nesting in app.rs

### Phase 3: Polish (Week 3)
9. [ ] Add theme constant usage
10. [ ] Dynamic provider generation
11. [ ] Unit tests for core logic
12. [ ] Documentation for widget patterns

---

## Files Modified During Review

| File | Issues Found | Status |
|------|--------------|--------|
| `apps/mofa-sidebar/src/sidebar.rs` | Duplication, debug logs | Needs refactor |
| `apps/mofa-fm/src/screen.rs` | Timer cleanup | ✅ FIXED |
| `apps/mofa-fm/src/mofa_hero.rs` | No issues | OK |
| `apps/mofa-settings/src/providers_panel.rs` | Hover not working | In progress |
| `apps/mofa-settings/src/screen.rs` | Debug logs | Cleaned |
| `mofa-studio-shell/src/app.rs` | Timer calls added | ✅ FIXED |

---

## Success Metrics

After completing M2 improvements:

| Metric | Current | Target |
|--------|---------|--------|
| Code duplication | ~30% | <5% |
| Debug statements | 15+ | 0 |
| Hover effects working | 0/1 | 1/1 |
| Naming consistency | 75% | 100% |
| Timer cleanup | ✅ Implemented | Automatic |
| Unit test coverage | 0% | 70% |

---

## Open Questions

1. **Theme Access:** Is `THEME` constant accessible in all widgets, or do we need a theme provider?
2. **Widget Communication:** Should we use `cx.widget_action` or direct references for parent-child communication?

---

---

## Cross-References

This roadmap is part of a three-document analysis:

| Document | Focus | Best For |
|----------|-------|----------|
| **roadmap-claude.md** | Architectural evidence | Understanding WHY problems exist |
| **roadmap-m2.md** (this) | Tactical bug fixes | Quick wins and immediate fixes |
| **roadmap-glm.md** | Strategic planning | Long-term roadmap with grades |

See also: [CHECKLIST.md](./CHECKLIST.md) - Master consolidated checklist

---

*Generated: 2026-01-04*
*Reviewer: Claude Code*
*Branch: cloud-model-mcp*
