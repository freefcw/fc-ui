# Contributing to adabraka-ui

Thank you for your interest in contributing to adabraka-ui! We welcome contributions from the community and are grateful for your support in making this library better.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How Can I Contribute?](#how-can-i-contribute)
  - [Reporting Bugs](#reporting-bugs)
  - [Suggesting Enhancements](#suggesting-enhancements)
  - [Contributing Code](#contributing-code)
- [Development Setup](#development-setup)
- [Pull Request Process](#pull-request-process)
- [Coding Guidelines](#coding-guidelines)
- [Component Development Guidelines](#component-development-guidelines)
- [Documentation Guidelines](#documentation-guidelines)
- [Testing Guidelines](#testing-guidelines)

## Code of Conduct

This project adheres to a code of conduct that all contributors are expected to follow. Please be respectful, inclusive, and considerate in all interactions.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check existing issues to avoid duplicates. When creating a bug report, include as many details as possible:

- **Use a clear and descriptive title**
- **Describe the exact steps to reproduce the problem**
- **Provide specific examples** (code snippets, screenshots)
- **Describe the behavior you observed** and what you expected
- **Include your environment details** (Rust version, GPUI version, OS)

Use our [bug report template](.github/ISSUE_TEMPLATE/bug_report.md) when creating issues.

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion:

- **Use a clear and descriptive title**
- **Provide a detailed description** of the proposed feature
- **Explain why this enhancement would be useful**
- **Provide examples** of how it would be used
- **Consider if this fits the scope** of adabraka-ui

Use our [feature request template](.github/ISSUE_TEMPLATE/feature_request.md) when creating suggestions.

### Contributing Code

We love pull requests! Here's how to contribute:

1. **Fork the repository** and create your branch from `main`
2. **Follow our coding guidelines** (see below)
3. **Add tests** for new functionality
4. **Update documentation** as needed
5. **Ensure all tests pass** and examples compile
6. **Submit your pull request**

## Development Setup

### Prerequisites

- Rust 1.90 or higher
- fc-gpui 0.10 or higher
- just
- Git

### Setup Instructions

1. **Clone your fork:**
   ```bash
   git clone https://github.com/freefcw/fc-ui.git
   cd fc-ui
   ```

2. **Build the project:**
   ```bash
   just build
   ```

3. **Run tests:**
   ```bash
   just test
   ```

4. **Run examples:**
   ```bash
   just run-example demo
   ```

### Project Structure

```
adabraka-ui/
├── src/
│   ├── capabilities/   # Canonical implementations (foundation, layout, controls, scroll, ...)
│   ├── components/     # Compatibility facades
│   ├── theme/          # Theme facade
│   └── lib.rs          # Library entry point
├── examples/           # Example applications
├── docs/               # GitHub Pages site
└── tests/              # Integration tests
```

## Pull Request Process

1. **Create a feature branch:**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following our guidelines

3. **Test thoroughly:**
   - Run `just test`
   - Run `just clippy` for linting
   - Run `just fmt` for formatting
   - Test relevant examples

4. **Commit with clear messages:**
   ```bash
   git commit -m "Add feature: brief description

   Detailed description of what changed and why."
   ```

5. **Push to your fork:**
   ```bash
   git push origin feature/your-feature-name
   ```

6. **Open a Pull Request** using our template

7. **Respond to feedback** from reviewers

### PR Requirements

- ✅ All tests pass
- ✅ Code is formatted with `just fmt`
- ✅ No clippy warnings
- ✅ Documentation is updated
- ✅ Examples are provided for new components
- ✅ Commit messages are clear and descriptive

## Coding Guidelines

### General Principles

- **Follow Rust idioms** and best practices
- **Keep code simple and readable**
- **Prefer composition over inheritance**
- **Use meaningful variable and function names**
- **Write self-documenting code** with comments for complex logic

### Rust Style

- Use `just fmt` with default settings
- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Maximum line length: 100 characters
- Use `rustfmt.toml` configuration if present

### Code Organization

```rust
// 1. Imports (grouped and sorted)
use gpui::*;
use crate::theme::*;

// 2. Type definitions
pub struct MyComponent {
    // fields...
}

// 3. Constructors and builders
impl MyComponent {
    pub fn new() -> Self {
        // ...
    }

    // Builder methods
    pub fn variant(mut self, variant: Variant) -> Self {
        // ...
    }
}

// 4. Trait implementations
impl Render for MyComponent {
    // ...
}

// 5. Helper functions
```

### Naming Conventions

- **Types**: `PascalCase` (e.g., `Button`, `InputState`)
- **Functions/methods**: `snake_case` (e.g., `on_click`, `set_value`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `DEFAULT_SIZE`)
- **Modules**: `snake_case` (e.g., `text_input`, `data_table`)

## Component Development Guidelines

### Component Structure

Every component should follow this pattern:

```rust
use gpui::*;
use crate::theme::*;

/// Brief description of the component
///
/// # Examples
///
/// ```rust
/// Button::new("my-button", "Click me")
///     .variant(ButtonVariant::Default)
///     .on_click(|_, _, _| println!("Clicked!"))
/// ```
pub struct Button {
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    // ... other fields
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    Default,
    Secondary,
    Destructive,
    Outline,
    Ghost,
    Link,
}

impl Button {
    /// Creates a new button with the given ID and label
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::Default,
        }
    }

    /// Sets the button variant
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = use_theme(cx);

        div()
            .child(self.label)
            // ... styling
    }
}
```

### Capability Ownership

Canonical implementations belong under `src/capabilities/`; the public `components`, `display`, `charts`, `navigation`, and `overlays` modules are compatibility facades. Do not add a new implementation directly to a facade.

Choose ownership by the primary user workflow, not by the old directory name:

| User-facing behavior | Canonical capability |
| --- | --- |
| Compose generic stacks, grids, panels, or containers | `layout` |
| Enter, select, validate, or edit a single value | `controls` |
| Navigate an app, command, route, menu, or tree | `navigation` |
| Present tables, charts, timelines, or structured data | `data` |
| Render static, rich, Markdown, or HTML content | `content` |
| Edit a rope-backed document with language behavior | `editor` |
| Work with audio, images, video, canvas, or uploaded media | `media` |
| Show a temporary dialog, popover, sheet, or toast | `overlays` |
| Scroll, resize, split, or virtualize a collection | `scroll` |
| Add animation or decorative visual effects | `motion` or `effects` |

For the complete ownership and exclusion table, read `src/capabilities/mod.rs`. When a component spans two workflows, keep its primary state and public API with the user-facing capability, and depend on the other capability through its canonical module. Do not use a legacy facade as an internal import path.

Production capability sources must use absolute `crate::capabilities::...` imports. Do not use `super::...` there, even for a sibling module: it hides the dependency route and can make a capability-boundary violation hard to spot. `super::...` remains fine inside `#[cfg(test)] mod tests` blocks, which the boundary check excludes.

### Component Checklist

When creating a new component:

- [ ] Choose its canonical capability owner before creating files
- [ ] Follow the builder pattern for configuration
- [ ] Use the theme system for colors and styling
- [ ] Support common variants (size, style, state)
- [ ] Include accessibility features (ARIA, keyboard nav)
- [ ] Add comprehensive documentation
- [ ] Create a dedicated example file
- [ ] Add tests for key functionality
- [ ] Add an owner initializer and preserve the root `init()` ordering when the component registers keybindings
- [ ] Preserve intended public API by updating the relevant facade, `prelude.rs`, and `tests/public_api.rs`
- [ ] Run `just check-boundaries` after changing canonical imports
- [ ] Update the main demo example

### Theming

Always use theme tokens instead of hardcoded colors:

```rust
// ✅ Good
let theme = use_theme(cx);
div().bg(theme.tokens.background)

// ❌ Bad
div().bg(rgb(0xffffff))
```

### Accessibility

- Add ARIA labels where appropriate
- Support keyboard navigation
- Provide focus indicators
- Support disabled states
- Test with screen readers when possible

## Documentation Guidelines

### Component Documentation

```rust
/// A button component for user interactions.
///
/// Buttons support multiple variants, sizes, and states. They follow
/// the shadcn/ui design system with support for icons and custom styling.
///
/// # Examples
///
/// Basic usage:
/// ```rust
/// Button::new("btn-click", "Click me")
///     .on_click(|_, _, _| println!("Clicked!"))
/// ```
///
/// With variants and sizes:
/// ```rust
/// Button::new("btn-primary", "Primary")
///     .variant(ButtonVariant::Default)
///     .size(ButtonSize::Lg)
/// ```
///
/// # Accessibility
///
/// Buttons automatically receive proper ARIA labels and keyboard support.
/// They can be activated with Enter or Space when focused.
pub struct Button {
    // ...
}
```

### Method Documentation

```rust
/// Sets the button variant.
///
/// # Arguments
///
/// * `variant` - The visual variant to apply
///
/// # Examples
///
/// ```rust
/// Button::new("btn-save", "Save").variant(ButtonVariant::Default)
/// ```
pub fn variant(mut self, variant: ButtonVariant) -> Self {
    // ...
}
```

### README Updates

When adding new components, update:

- Component list in the main README
- Appropriate category section
- Examples list if you added a new example

### Cargo Compatibility Contract

Cargo features and example targets are part of the supported compatibility surface. When an intentional change adds or removes either one:

1. Review the `Cargo.toml` change and its required feature combinations.
2. Run `just update-cargo-contract` to refresh `scripts/cargo_contract.json`.
3. Review the snapshot diff, then run `just check-cargo-contract`.

Do not refresh the snapshot to bypass an unexpected contract failure.

## Testing Guidelines

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_variant() {
        let button = Button::new("btn-test", "Test")
            .variant(ButtonVariant::Default);

        assert_eq!(button.variant, ButtonVariant::Default);
    }
}
```

### Integration Tests

Place integration tests in the `tests/` directory:

```rust
// tests/button_tests.rs
use adabraka_ui::components::button::*;

#[test]
fn test_button_builder_pattern() {
    // Test builder pattern
}
```

### GPUI Component Tests

Interactive components (input, textarea, select, ...) have two test layers. Pick the
cheapest one that actually exercises what you changed:

1. **Direct state manipulation** — most tests call the state's methods directly
   (`state.set_value(...)`, `state.word_left(&WordLeft, window, cx)`). No keymap is
   involved, so no `init` is required; only `install_theme` is needed when the code
   path reads theme tokens.
2. **Real keyboard chain** — for changes in key wiring, focus handling, or key
   bindings, drive actual keystrokes through a window:

```rust
#[gpui::test]
fn enter_from_single_line_input_reaches_form_submit() {
    let mut app = TestApp::new();
    app.update(|cx| {
        install_theme(cx, Theme::light());
        // Binds the keystrokes to actions; adabraka_ui::init does this for the
        // whole crate, a single component's init is enough for a focused test.
        crate::capabilities::controls::input::init(cx);
    });
    // ... build state + view, open a window ...
    window.draw();
    window.update(|_, window, cx| state.read(cx).focus_handle(cx).focus(window));
    window.draw();
    let handle: AnyWindowHandle = window.handle().into();
    app.raw_context_mut().simulate_keystrokes(handle, "enter");
    app.flush();
    // assert on effects
}
```

Reference tests: `enter_from_single_line_input_reaches_form_submit`
(`src/capabilities/controls/form.rs`) and
`wire_actions_gives_custom_input_containers_full_keyboard_editing`
(`src/capabilities/controls/input.rs`).

Known pitfalls:

- **Simulated keystrokes require the key bindings to be registered.** The bindings
  are attached by each component's `init` (`input::init`, `textarea::init`, ...).
  Without it, `simulate_keystrokes` dispatches nothing and the test fails silently
  in the form of "nothing happened".
- **Editing keys are filtered on the key context of the focused element.** Input
  and textarea bindings carry `Some("Input")` / `Some("Textarea")`; if the focused
  element does not set `key_context("Input")`, the binding never matches and every
  editing key except plain text insertion is silently dead. Components set this
  themselves; for a custom container around a bare `InputState`/`TextareaState`,
  use `wire_input_actions` / `wire_textarea_actions` (from the prelude) instead of
  hand-rolling the wiring.

A note on multi-form windows: `InputState` entities hold live drafts. When one
settings panel is reused for different data sources (provider switching, etc.),
key the input entities by the form's identity and reuse or rebuild them on
identity change — sharing entities across identities leaks drafts from one form
into the next, while rebuilding unconditionally drops the user's in-progress edits.

### Example Applications

Create a dedicated example for new components:

```rust
// examples/my_component_demo.rs
use adabraka_ui::prelude::*;
use gpui::*;

fn main() {
    Application::new().run(|cx| {
        // ... example implementation
    });
}
```

## Examples Guidelines

### Example Structure

Examples should:

- Demonstrate all major features
- Be well-commented
- Use realistic scenarios
- Follow the same coding style
- Be runnable with `just run-example name`

### Example Categories

- **Component demos**: Showcase a single component
- **Feature demos**: Demonstrate a specific feature (animations, theming)
- **Application demos**: Show components working together (IDE demo, file explorer)

## Commit Message Guidelines

Use clear, descriptive commit messages:

```
Add Button component with variants

- Implement builder pattern for configuration
- Add support for Default, Secondary, Outline, Ghost, Link, Destructive variants
- Include accessibility features (ARIA, keyboard nav)
- Add comprehensive examples and tests

Closes #123
```

Format:
- **First line**: Brief summary (50 chars or less)
- **Body**: Detailed description of changes
- **Footer**: Reference issues, breaking changes

## Questions?

If you have questions:

- Check existing issues and discussions
- Open a new discussion for general questions
- Use issue templates for bugs and feature requests
- Reach out to maintainers for guidance

## Recognition

All contributors will be recognized in our README and release notes. We appreciate every contribution, no matter how small!

## License

By contributing to adabraka-ui, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to adabraka-ui! 🎉
