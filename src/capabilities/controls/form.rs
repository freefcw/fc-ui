use crate::capabilities::controls::input::InputEvent;
use crate::capabilities::controls::input_state::InputState;
use crate::capabilities::controls::textarea_state::TextareaState;
use crate::capabilities::controls::validation::ValidationError;
use crate::capabilities::layout::VStack;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use std::collections::{HashMap, HashSet};

/// A field registered with a [`Form`] for validation orchestration.
///
/// Single-line (`InputState`) and multi-line (`TextareaState`) inputs share
/// the same validation machinery, so the form validates either kind without
/// knowing which one it holds.
#[derive(Clone)]
pub enum FormField {
    Input(Entity<InputState>),
    Textarea(Entity<TextareaState>),
}

impl From<Entity<InputState>> for FormField {
    fn from(state: Entity<InputState>) -> Self {
        Self::Input(state)
    }
}

impl From<Entity<TextareaState>> for FormField {
    fn from(state: Entity<TextareaState>) -> Self {
        Self::Textarea(state)
    }
}

impl FormField {
    /// Validate the underlying field state, recording its error/success
    /// message on the state itself.
    pub fn validate(&self, cx: &mut App) -> Result<(), ValidationError> {
        match self {
            Self::Input(entity) => entity.update(cx, |state, cx| state.validate(cx)),
            Self::Textarea(entity) => entity.update(cx, |state, cx| state.validate(cx)),
        }
    }

    fn clear_validation(&self, cx: &mut App) {
        match self {
            Self::Input(entity) => entity.update(cx, |state, cx| {
                state.validation_error = None;
                state.success_message = None;
                cx.notify();
            }),
            Self::Textarea(entity) => entity.update(cx, |state, cx| {
                state.validation_error = None;
                state.success_message = None;
                cx.notify();
            }),
        }
    }
}

pub struct FormState {
    fields: Vec<(SharedString, FormField)>,
    errors: HashMap<SharedString, Vec<String>>,
    dirty: HashSet<SharedString>,
    submitted: bool,
    field_subscriptions: Vec<Subscription>,
}

impl FormState {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|_cx| Self {
            fields: Vec::new(),
            errors: HashMap::new(),
            dirty: HashSet::new(),
            submitted: false,
            field_subscriptions: Vec::new(),
        })
    }

    /// Register a field for validation and Enter-driven form submission.
    ///
    /// Accepts both `Entity<InputState>` and `Entity<TextareaState>`.
    pub fn register_field(&mut self, name: impl Into<SharedString>, field: impl Into<FormField>) {
        let name = name.into();
        self.fields.push((name, field.into()));
    }

    fn replace_field_subscriptions(&mut self, subscriptions: Vec<Subscription>) {
        self.field_subscriptions = subscriptions;
    }

    pub fn validate_all(&mut self, cx: &mut Context<Self>) -> bool {
        self.errors.clear();
        self.submitted = true;

        let fields: Vec<_> = self.fields.clone();
        let mut all_valid = true;

        for (name, field) in fields {
            let result = field.validate(cx);

            if let Err(error) = result {
                all_valid = false;
                self.errors
                    .entry(name)
                    .or_default()
                    .push(error.message.to_string());
            }
        }

        cx.notify();
        all_valid
    }

    pub fn field_errors(&self, name: &str) -> Option<&Vec<String>> {
        self.errors.get(name)
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    pub fn mark_dirty(&mut self, name: impl Into<SharedString>) {
        self.dirty.insert(name.into());
    }

    pub fn reset(&mut self, cx: &mut Context<Self>) {
        let fields = self.fields.clone();
        for (_, field) in fields {
            field.clear_validation(cx);
        }
        self.errors.clear();
        self.dirty.clear();
        self.submitted = false;
        cx.notify();
    }

    pub fn is_submitted(&self) -> bool {
        self.submitted
    }
}

#[derive(IntoElement)]
pub struct Form {
    state: Entity<FormState>,
    on_submit: Option<Box<dyn Fn(&mut Window, &mut App)>>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Form {
    pub fn new(state: Entity<FormState>) -> Self {
        Self {
            state,
            on_submit: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    /// Register the callback invoked when a registered single-line input
    /// receives Enter and all registered fields validate successfully.
    pub fn on_submit(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_submit = Some(Box::new(handler));
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.children
            .extend(children.into_iter().map(|c| c.into_any_element()));
        self
    }
}

impl Styled for Form {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Form {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.clone();
        let on_submit = self.on_submit;
        let user_style = self.style;

        let handler = on_submit.map(std::rc::Rc::new);
        let subscriptions = if let Some(handler) = handler {
            // Enter-driven submission only applies to single-line inputs; in a
            // textarea Enter inserts a newline.
            let fields: Vec<_> = state
                .read(cx)
                .fields
                .iter()
                .filter_map(|(_, field)| match field {
                    FormField::Input(entity) => Some(entity.clone()),
                    FormField::Textarea(_) => None,
                })
                .collect();
            fields
                .into_iter()
                .map(|field| {
                    let form = state.clone();
                    let handler = handler.clone();
                    window.subscribe(&field, cx, move |_, event: &InputEvent, window, cx| {
                        if !matches!(event, InputEvent::Enter) {
                            return;
                        }

                        let is_valid =
                            form.update(cx, |form_state, cx| form_state.validate_all(cx));
                        if is_valid {
                            (handler)(window, cx);
                        }
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        state.update(cx, |form, _| {
            form.replace_field_subscriptions(subscriptions)
        });

        VStack::new()
            .w_full()
            .gap(px(16.0))
            .children(self.children)
            .map(|this| {
                let mut vstack = this;
                vstack.style().refine(&user_style);
                vstack
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::controls::{input::Input, textarea::Textarea};
    use crate::capabilities::foundation::theme::{install_theme, Theme};
    use std::cell::Cell;
    use std::rc::Rc;

    struct FormTestView {
        form: Entity<FormState>,
        input: Entity<InputState>,
        submits: Rc<Cell<usize>>,
        enters: Rc<Cell<usize>>,
    }

    impl Render for FormTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let submits = self.submits.clone();
            let enters = self.enters.clone();
            Form::new(self.form.clone())
                .child(Input::new(&self.input).on_enter(move |_, _| {
                    enters.set(enters.get() + 1);
                }))
                .on_submit(move |_, _| submits.set(submits.get() + 1))
        }
    }

    #[gpui::test]
    fn enter_from_single_line_input_reaches_form_submit() {
        let mut app = TestApp::new();
        app.update(|cx| {
            install_theme(cx, Theme::light());
            crate::capabilities::controls::input::init(cx);
        });

        let submits = Rc::new(Cell::new(0));
        let submits_for_view = submits.clone();
        let enters = Rc::new(Cell::new(0));
        let enters_for_view = enters.clone();
        let input = app.update(|cx| cx.new(InputState::new));
        let form = app.update(|cx| {
            let form = cx.new(|_| FormState {
                fields: Vec::new(),
                errors: HashMap::new(),
                dirty: HashSet::new(),
                submitted: false,
                field_subscriptions: Vec::new(),
            });
            form.update(cx, |form, _| form.register_field("value", input.clone()));
            form
        });

        let mut window = app.open_window(|_, _| FormTestView {
            form,
            input: input.clone(),
            submits: submits_for_view,
            enters: enters_for_view,
        });
        window.draw();
        window.update(|_, window, cx| input.read(cx).focus_handle(cx).focus(window));
        window.draw();
        let handle: AnyWindowHandle = window.handle().into();
        app.raw_context_mut().simulate_keystrokes(handle, "enter");
        app.flush();

        assert_eq!(submits.get(), 1);
        assert_eq!(enters.get(), 1);
    }
    #[gpui::test]
    fn validate_all_covers_textarea_fields() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));

        let input = app.update(|cx| cx.new(InputState::new));
        let textarea = app.update(|cx| {
            cx.new(|cx| {
                crate::capabilities::controls::textarea_state::TextareaState::new(cx)
                    .min_length(3)
                    .value("ab")
            })
        });
        let form = app.update(|cx| {
            let form = FormState::new(cx);
            form.update(cx, |form, _| {
                form.register_field("name", input.clone());
                form.register_field("bio", textarea.clone());
            });
            form
        });

        app.update(|cx| {
            let valid = form.update(cx, |form, cx| form.validate_all(cx));
            assert!(!valid);
            assert_eq!(
                form.read(cx).field_errors("bio").unwrap()[0].as_str(),
                "Must be at least 3 characters"
            );
            // The optional empty input skips constraint checks entirely.
            assert!(form.read(cx).field_errors("name").is_none());
        });

        app.update(|cx| {
            textarea.update(cx, |state, _| state.content = "abcd".into());
            let valid = form.update(cx, |form, cx| form.validate_all(cx));
            assert!(valid);
            assert!(form.read(cx).is_valid());
        });
    }

    #[gpui::test]
    fn reset_clears_form_and_field_validation_state() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));

        let input = app.update(|cx| cx.new(|cx| InputState::new(cx).required(true)));
        let textarea = app.update(|cx| cx.new(|cx| TextareaState::new(cx).required(true)));
        let form = app.update(|cx| {
            let form = FormState::new(cx);
            form.update(cx, |form, _| {
                form.register_field("name", input.clone());
                form.register_field("bio", textarea.clone());
                form.mark_dirty("name");
            });
            form
        });

        app.update(|cx| {
            let valid = form.update(cx, |form, cx| form.validate_all(cx));
            assert!(!valid);
            assert!(input.read(cx).validation_error.is_some());
            assert!(textarea.read(cx).validation_error.is_some());
            assert!(form.read(cx).is_submitted());
            assert!(form.read(cx).is_dirty());
        });

        app.update(|cx| form.update(cx, |form, cx| form.reset(cx)));

        app.update(|cx| {
            assert!(form.read(cx).is_valid());
            assert!(!form.read(cx).is_submitted());
            assert!(!form.read(cx).is_dirty());
            assert!(input.read(cx).validation_error.is_none());
            assert!(input.read(cx).success_message.is_none());
            assert!(textarea.read(cx).validation_error.is_none());
            assert!(textarea.read(cx).success_message.is_none());
        });
    }

    struct TextareaFormTestView {
        form: Entity<FormState>,
        textarea: Entity<TextareaState>,
        submits: Rc<Cell<usize>>,
        enters: Rc<Cell<usize>>,
    }

    impl Render for TextareaFormTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let submits = self.submits.clone();
            let enters = self.enters.clone();
            Form::new(self.form.clone())
                .child(Textarea::new(&self.textarea).on_enter(move |_, _| {
                    enters.set(enters.get() + 1);
                }))
                .on_submit(move |_, _| submits.set(submits.get() + 1))
        }
    }

    #[gpui::test]
    fn enter_in_textarea_inserts_newline_without_submitting_form() {
        let mut app = TestApp::new();
        app.update(|cx| {
            install_theme(cx, Theme::light());
            crate::capabilities::controls::textarea::init(cx);
        });

        let textarea = app.update(|cx| cx.new(TextareaState::new));
        let form = app.update(|cx| {
            let form = FormState::new(cx);
            form.update(cx, |form, _| form.register_field("bio", textarea.clone()));
            form
        });
        let submits = Rc::new(Cell::new(0));
        let enters = Rc::new(Cell::new(0));
        let mut window = app.open_window(|_, _| TextareaFormTestView {
            form,
            textarea: textarea.clone(),
            submits: submits.clone(),
            enters: enters.clone(),
        });

        window.draw();
        window.update(|_, window, cx| textarea.read(cx).focus_handle(cx).focus(window));
        window.draw();
        let handle: AnyWindowHandle = window.handle().into();
        app.raw_context_mut().simulate_keystrokes(handle, "enter");
        app.flush();

        app.update(|cx| assert_eq!(textarea.read(cx).content(), "\n"));
        assert_eq!(enters.get(), 1);
        assert_eq!(submits.get(), 0);
    }
}
