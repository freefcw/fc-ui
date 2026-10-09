use fc_ui::{
    components::input::{Input, InputState},
    components::textarea::Textarea,
    components::textarea_state::TextareaState,
    prelude::{Button, Checkbox, Dialog, Select, SelectOption},
};
use gpui::{
    AppContext, Context, IntoElement, ParentElement as _, Render, Styled as _, TestAppContext,
    VisualTestContext, Window,
};
use serde_json::Value;
use std::{cell::Cell, rc::Rc, time::Duration};

struct ButtonView {
    disabled: bool,
    loading: bool,
}

impl Render for ButtonView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        Button::new("save-settings", "Save settings")
            .disabled(self.disabled)
            .loading(self.loading)
    }
}

struct CheckboxView {
    checked: bool,
    indeterminate: bool,
    disabled: bool,
}

impl Render for CheckboxView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        Checkbox::new("product-updates")
            .label("Product updates")
            .checked(self.checked)
            .indeterminate(self.indeterminate)
            .disabled(self.disabled)
    }
}

struct InputView {
    state: gpui::Entity<InputState>,
    disabled: bool,
    password: bool,
}

struct TextareaView {
    state: gpui::Entity<TextareaState>,
    disabled: bool,
}

struct TextareaLayoutView {
    state: gpui::Entity<TextareaState>,
}

impl Render for TextareaLayoutView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        Textarea::new(&self.state)
            .w(gpui::px(320.0))
            .helper_text("Helper text")
            .show_character_count(true)
    }
}

impl Render for TextareaView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        Textarea::new(&self.state)
            .aria_label("Biography")
            .aria_description("Tell us about yourself")
            .placeholder("Write a short biography")
            .required(true)
            .error(true)
            .disabled(self.disabled)
    }
}

impl Render for InputView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        Input::new(&self.state)
            .aria_label("Email")
            .aria_description("Used for account notifications")
            .placeholder("name@example.com")
            .value("alice@example.com")
            .required(true)
            .error(true)
            .disabled(self.disabled)
            .password(self.password)
    }
}

struct SettingsHost {
    dialog: Option<gpui::Entity<Dialog>>,
}

impl Render for SettingsHost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        self.dialog
            .clone()
            .map(IntoElement::into_any_element)
            .unwrap_or_else(|| gpui::div().into_any_element())
    }
}

fn draw_accessibility_tree(
    cx: &mut TestAppContext,
    build: impl FnOnce(&mut TestAppContext),
) -> Value {
    cx.update(fc_ui::init);
    build(cx);

    let window = cx.windows()[0];
    let mut cx = gpui::VisualTestContext::from_window(window, cx);
    cx.simulate_accessibility_activation();
    cx.update(|window, cx| window.draw(cx).clear());

    let tree = cx
        .update(|window, _| window.debug_a11y_tree_json())
        .expect("accessibility activation should produce a debug tree");
    serde_json::from_str(&tree).expect("debug accessibility tree should be valid JSON")
}

fn current_accessibility_tree(cx: &mut VisualTestContext) -> Value {
    let tree = cx
        .update(|window, _| window.debug_a11y_tree_json())
        .expect("accessibility activation should produce a debug tree");
    serde_json::from_str(&tree).expect("debug accessibility tree should be valid JSON")
}

fn node_with_role<'a>(tree: &'a Value, role: &str) -> &'a Value {
    tree["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["aria"]["role"] == role)
        .unwrap_or_else(|| panic!("missing accessibility node with role {role}"))
}

fn node_with_role_and_label<'a>(tree: &'a Value, role: &str, label: &str) -> &'a Value {
    tree["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["aria"]["role"] == role && node["aria"]["label"] == label)
        .unwrap_or_else(|| panic!("missing accessibility node with role {role} and label {label}"))
}

fn node_id(node: &Value) -> gpui::accesskit::NodeId {
    let id = node["accesskit_id"]
        .as_str()
        .expect("accessibility node should expose its AccessKit id")
        .parse()
        .expect("AccessKit id should be numeric");
    gpui::accesskit::NodeId(id)
}

fn descendant_has_role(tree: &Value, parent: &Value, role: &str) -> bool {
    let Some(children) = parent["children"].as_array() else {
        return false;
    };

    children.iter().any(|child_id| {
        let child = tree["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["id"] == *child_id)
            .expect("accessibility child id should resolve to a node");
        child["aria"]["role"] == role || descendant_has_role(tree, child, role)
    })
}

fn perform_accessibility_action(
    cx: &mut VisualTestContext,
    node: &Value,
    action: gpui::accesskit::Action,
) {
    cx.simulate_accessibility_action(gpui::accesskit::ActionRequest {
        action,
        target_tree: gpui::accesskit::TreeId::ROOT,
        target_node: node_id(node),
        data: None,
    });
    cx.update(|window, cx| window.draw(cx).clear());
}

#[gpui::test]
fn button_exposes_its_role_and_visible_label(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, _| ButtonView {
            disabled: false,
            loading: false,
        });
    });
    let button = node_with_role(&tree, "Button");

    assert_eq!(button["aria"]["label"], "Save settings");
}

#[gpui::test]
fn disabled_button_reports_its_disabled_state(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, _| ButtonView {
            disabled: true,
            loading: false,
        });
    });
    let button = node_with_role(&tree, "Button");

    assert_eq!(button["aria"]["disabled"], true);
}

#[gpui::test]
fn loading_button_reports_that_it_is_not_available(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, _| ButtonView {
            disabled: false,
            loading: true,
        });
    });
    let button = node_with_role(&tree, "Button");

    assert_eq!(button["aria"]["disabled"], true);
}

#[gpui::test]
fn checkbox_exposes_its_label_and_checked_state(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, _| CheckboxView {
            checked: true,
            indeterminate: false,
            disabled: false,
        });
    });
    let checkbox = node_with_role(&tree, "CheckBox");

    assert_eq!(checkbox["aria"]["label"], "Product updates");
    assert_eq!(checkbox["aria"]["toggled"], "True");
}

#[gpui::test]
fn indeterminate_checkbox_reports_mixed_state(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, _| CheckboxView {
            checked: false,
            indeterminate: true,
            disabled: false,
        });
    });
    let checkbox = node_with_role(&tree, "CheckBox");

    assert_eq!(checkbox["aria"]["toggled"], "Mixed");
}

#[gpui::test]
fn select_exposes_its_label_value_and_collapsed_state(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, cx| {
            Select::new(cx)
                .options(vec![
                    SelectOption::new("light", "Light"),
                    SelectOption::new("dark", "Dark"),
                ])
                .selected_index(Some(1))
                .aria_label("Theme")
        });
    });
    let select = node_with_role(&tree, "ComboBox");

    assert_eq!(select["aria"]["label"], "Theme");
    assert_eq!(select["aria"]["value"], "Dark");
    assert_eq!(select["aria"]["expanded"], false);
}

#[gpui::test]
fn unselected_select_exposes_placeholder_without_claiming_a_value(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, cx| {
            Select::new(cx)
                .options(vec![SelectOption::new("dark", "Dark")])
                .placeholder("Choose a theme")
                .aria_label("Theme")
        });
    });
    let select = node_with_role(&tree, "ComboBox");

    assert_eq!(select["aria"]["placeholder"], "Choose a theme");
    assert!(select["aria"].get("value").is_none());
}

#[gpui::test]
fn select_expand_action_exposes_options_and_focuses_the_combobox(cx: &mut TestAppContext) {
    cx.update(fc_ui::init);
    let (_, cx) = cx.add_window_view(|_, cx| {
        Select::new(cx)
            .options(vec![
                SelectOption::new("light", "Light"),
                SelectOption::new("dark", "Dark"),
            ])
            .selected_index(Some(0))
            .aria_label("Theme")
    });
    cx.simulate_accessibility_activation();
    cx.update(|window, cx| window.draw(cx).clear());

    let tree = current_accessibility_tree(cx);
    let select_id = node_id(node_with_role(&tree, "ComboBox"));
    cx.simulate_accessibility_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Expand,
        target_tree: gpui::accesskit::TreeId::ROOT,
        target_node: select_id,
        data: None,
    });
    cx.update(|window, cx| window.draw(cx).clear());

    let tree = current_accessibility_tree(cx);
    let select = node_with_role(&tree, "ComboBox");
    assert_eq!(select["aria"]["expanded"], true);
    assert_eq!(tree["focus"], select["id"]);
    node_with_role(&tree, "ListBox");
    node_with_role_and_label(&tree, "ListBoxOption", "Light");
    node_with_role_and_label(&tree, "ListBoxOption", "Dark");
}

#[gpui::test]
fn select_option_action_updates_the_value_and_collapses_the_list(cx: &mut TestAppContext) {
    cx.update(fc_ui::init);
    let (_, cx) = cx.add_window_view(|_, cx| {
        Select::new(cx)
            .options(vec![
                SelectOption::new("light", "Light"),
                SelectOption::new("dark", "Dark"),
            ])
            .selected_index(Some(0))
            .aria_label("Theme")
    });
    cx.simulate_accessibility_activation();
    cx.update(|window, cx| window.draw(cx).clear());

    let tree = current_accessibility_tree(cx);
    let select_id = node_id(node_with_role(&tree, "ComboBox"));
    cx.simulate_accessibility_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Expand,
        target_tree: gpui::accesskit::TreeId::ROOT,
        target_node: select_id,
        data: None,
    });
    cx.update(|window, cx| window.draw(cx).clear());

    let tree = current_accessibility_tree(cx);
    let dark_option_id = node_id(node_with_role_and_label(&tree, "ListBoxOption", "Dark"));
    cx.simulate_accessibility_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_tree: gpui::accesskit::TreeId::ROOT,
        target_node: dark_option_id,
        data: None,
    });
    cx.update(|window, cx| window.draw(cx).clear());

    let tree = current_accessibility_tree(cx);
    let select = node_with_role(&tree, "ComboBox");
    assert_eq!(select["aria"]["value"], "Dark");
    assert_eq!(select["aria"]["expanded"], false);
    assert_eq!(tree["focus"], select["id"]);
}

#[gpui::test]
fn select_collapse_action_closes_the_dropdown(cx: &mut TestAppContext) {
    cx.update(fc_ui::init);
    let (_, cx) = cx.add_window_view(|_, cx| {
        Select::new(cx)
            .options(vec![SelectOption::new("dark", "Dark")])
            .selected_index(Some(0))
            .aria_label("Theme")
    });
    cx.simulate_accessibility_activation();
    cx.update(|window, cx| window.draw(cx).clear());

    let tree = current_accessibility_tree(cx);
    let select_id = node_id(node_with_role(&tree, "ComboBox"));
    cx.simulate_accessibility_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Expand,
        target_tree: gpui::accesskit::TreeId::ROOT,
        target_node: select_id,
        data: None,
    });
    cx.update(|window, cx| window.draw(cx).clear());
    cx.simulate_accessibility_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Collapse,
        target_tree: gpui::accesskit::TreeId::ROOT,
        target_node: select_id,
        data: None,
    });
    cx.update(|window, cx| window.draw(cx).clear());

    let tree = current_accessibility_tree(cx);
    let select = node_with_role(&tree, "ComboBox");
    assert_eq!(select["aria"]["expanded"], false);
}

#[gpui::test]
fn input_exposes_its_form_field_semantics(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, cx| InputView {
            state: cx.new(InputState::new),
            disabled: false,
            password: false,
        });
    });
    let input = node_with_role(&tree, "TextInput");

    assert_eq!(input["aria"]["label"], "Email");
    assert_eq!(
        input["aria"]["description"],
        "Used for account notifications"
    );
    assert_eq!(input["aria"]["value"], "alice@example.com");
    assert_eq!(input["aria"]["placeholder"], "name@example.com");
    assert_eq!(input["aria"]["required"], true);
    assert_eq!(input["aria"]["invalid"], "True");
}

#[gpui::test]
fn disabled_input_reports_its_disabled_state(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, cx| InputView {
            state: cx.new(InputState::new),
            disabled: true,
            password: false,
        });
    });
    let input = node_with_role(&tree, "TextInput");

    assert_eq!(input["aria"]["disabled"], true);
}

#[gpui::test]
fn password_input_does_not_expose_its_raw_value(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, cx| InputView {
            state: cx.new(InputState::new),
            disabled: false,
            password: true,
        });
    });
    let input = node_with_role(&tree, "PasswordInput");

    assert_ne!(input["aria"]["value"], "alice@example.com");
    assert!(!input["aria"]["value"].as_str().unwrap().is_empty());
}

#[gpui::test]
fn textarea_exposes_its_multiline_form_field_semantics(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, cx| TextareaView {
            state: cx.new(|cx| TextareaState::new(cx).value("hello")),
            disabled: false,
        });
    });
    let textarea = node_with_role(&tree, "MultilineTextInput");

    assert_eq!(textarea["aria"]["label"], "Biography");
    assert_eq!(textarea["aria"]["description"], "Tell us about yourself");
    assert_eq!(textarea["aria"]["value"], "hello");
    assert_eq!(textarea["aria"]["placeholder"], "Write a short biography");
    assert_eq!(textarea["aria"]["required"], true);
    assert_eq!(textarea["aria"]["invalid"], "True");
}

#[gpui::test]
fn disabled_textarea_reports_its_disabled_state(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, cx| TextareaView {
            state: cx.new(TextareaState::new),
            disabled: true,
        });
    });
    let textarea = node_with_role(&tree, "MultilineTextInput");

    assert_eq!(textarea["aria"]["disabled"], true);
}

#[gpui::test]
fn textarea_footer_preserves_component_width(cx: &mut TestAppContext) {
    cx.update(fc_ui::init);
    let (_, cx) = cx.add_window_view(|_, cx| TextareaLayoutView {
        state: cx.new(TextareaState::new),
    });

    cx.update(|window, cx| window.draw(cx).clear());
    let root = cx
        .debug_bounds("textarea-root")
        .expect("textarea root should expose its bounds");
    let field = cx
        .debug_bounds("textarea-field")
        .expect("textarea field should expose its bounds");

    assert!(root.size.width > gpui::px(0.0));
    assert_eq!(root.size.width, field.size.width);
    assert!(root.size.height > field.size.height);
}

#[gpui::test]
fn settings_dialog_supports_a_complete_assistive_action_flow(cx: &mut TestAppContext) {
    cx.update(fc_ui::init);
    let updates_enabled = Rc::new(Cell::new(false));
    let saved = Rc::new(Cell::new(false));
    let closed = Rc::new(Cell::new(false));
    let (host, cx) = cx.add_window_view(|_, _| SettingsHost { dialog: None });
    cx.simulate_accessibility_activation();
    cx.update(|window, cx| window.draw(cx).clear());

    host.update(cx, {
        let updates_enabled = updates_enabled.clone();
        let saved = saved.clone();
        let closed = closed.clone();
        move |host, cx| {
            let select = cx.new(|cx| {
                Select::new(cx)
                    .options(vec![
                        SelectOption::new("light", "Light"),
                        SelectOption::new("dark", "Dark"),
                    ])
                    .selected_index(Some(0))
                    .aria_label("Theme")
            });
            let input_state = cx.new(InputState::new);
            host.dialog = Some(cx.new(move |cx| {
                Dialog::new(cx)
                    .title("Account settings")
                    .description("Manage account preferences")
                    .child({
                        let input_state = input_state.clone();
                        let select = select.clone();
                        let updates_enabled = updates_enabled.clone();
                        let saved = saved.clone();
                        move |_, _| {
                            let updates_enabled = updates_enabled.clone();
                            let saved = saved.clone();
                            gpui::div()
                                .flex()
                                .flex_col()
                                .gap(gpui::px(16.0))
                                .child(
                                    Input::new(&input_state)
                                        .aria_label("Email")
                                        .value("alice@example.com"),
                                )
                                .child(
                                    Checkbox::new("product-updates")
                                        .label("Product updates")
                                        .on_click(move |checked, _, _| {
                                            updates_enabled.set(*checked)
                                        }),
                                )
                                .child(select.clone())
                                .child(
                                    Button::new("save-settings", "Save settings")
                                        .on_click(move |_, _, _| saved.set(true)),
                                )
                        }
                    })
                    .on_close(move |_, _| closed.set(true))
            }));
            cx.notify();
        }
    });
    cx.update(|window, cx| window.draw(cx).clear());

    let tree = current_accessibility_tree(cx);
    let dialog = node_with_role(&tree, "Dialog");
    assert!(descendant_has_role(&tree, dialog, "TextInput"));
    assert!(descendant_has_role(&tree, dialog, "CheckBox"));
    assert!(descendant_has_role(&tree, dialog, "ComboBox"));
    assert!(descendant_has_role(&tree, dialog, "Button"));

    let checkbox = node_with_role_and_label(&tree, "CheckBox", "Product updates").clone();
    perform_accessibility_action(cx, &checkbox, gpui::accesskit::Action::Click);
    assert!(updates_enabled.get());

    let tree = current_accessibility_tree(cx);
    let select = node_with_role_and_label(&tree, "ComboBox", "Theme").clone();
    perform_accessibility_action(cx, &select, gpui::accesskit::Action::Expand);
    let tree = current_accessibility_tree(cx);
    let dark_option = node_with_role_and_label(&tree, "ListBoxOption", "Dark").clone();
    perform_accessibility_action(cx, &dark_option, gpui::accesskit::Action::Click);
    let tree = current_accessibility_tree(cx);
    let select = node_with_role_and_label(&tree, "ComboBox", "Theme");
    assert_eq!(select["aria"]["value"], "Dark");
    assert_eq!(select["aria"]["expanded"], false);

    let save = node_with_role_and_label(&tree, "Button", "Save settings").clone();
    perform_accessibility_action(cx, &save, gpui::accesskit::Action::Click);
    assert!(saved.get());

    let tree = current_accessibility_tree(cx);
    let close = node_with_role_and_label(&tree, "Button", "Close dialog").clone();
    perform_accessibility_action(cx, &close, gpui::accesskit::Action::Click);
    cx.executor().advance_clock(Duration::from_millis(250));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear());
    assert!(closed.get());
}

#[gpui::test]
fn dialog_on_close_runs_once_across_later_redraws(cx: &mut TestAppContext) {
    cx.update(fc_ui::init);
    let close_count = Rc::new(Cell::new(0));
    let (host, cx) = cx.add_window_view(|_, _| SettingsHost { dialog: None });
    cx.simulate_accessibility_activation();
    cx.update(|window, cx| window.draw(cx).clear());

    host.update(cx, {
        let close_count = close_count.clone();
        move |host, cx| {
            host.dialog = Some(cx.new(move |cx| {
                Dialog::new(cx)
                    .title("Account settings")
                    .on_close(move |_, _| close_count.set(close_count.get() + 1))
            }));
            cx.notify();
        }
    });
    cx.update(|window, cx| window.draw(cx).clear());

    let tree = current_accessibility_tree(cx);
    let close = node_with_role_and_label(&tree, "Button", "Close dialog").clone();
    perform_accessibility_action(cx, &close, gpui::accesskit::Action::Click);
    cx.executor().advance_clock(Duration::from_millis(250));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear());
    cx.update(|window, cx| window.draw(cx).clear());
    cx.update(|window, cx| window.draw(cx).clear());

    assert_eq!(close_count.get(), 1);
}

#[gpui::test]
fn dialog_child_and_footer_survive_repeated_redraws(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, cx| {
            let input_state = cx.new(InputState::new);
            Dialog::new(cx)
                .title("Account settings")
                .child({
                    let input_state = input_state.clone();
                    move |_, _| Input::new(&input_state).aria_label("Email")
                })
                .footer(|_, _| Button::new("save-settings", "Save settings"))
        });
    });

    let dialog = node_with_role(&tree, "Dialog");
    assert!(descendant_has_role(&tree, dialog, "TextInput"));
    assert!(descendant_has_role(&tree, dialog, "Button"));
    node_with_role_and_label(&tree, "TextInput", "Email");
    node_with_role_and_label(&tree, "Button", "Save settings");
}

#[gpui::test]
fn dialog_exposes_its_title_description_and_modal_state(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, cx| {
            Dialog::new(cx)
                .title("Account settings")
                .description("Manage account preferences")
                .show_close_button(false)
        });
    });
    let dialog = node_with_role(&tree, "Dialog");

    assert_eq!(dialog["aria"]["label"], "Account settings");
    assert_eq!(dialog["aria"]["description"], "Manage account preferences");
    assert_eq!(dialog["aria"]["modal"], true);
}

#[gpui::test]
fn dialog_close_button_has_a_readable_label(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, cx| Dialog::new(cx).title("Account settings"));
    });
    let close_button =
        tree["nodes"].as_array().unwrap().iter().find(|node| {
            node["aria"]["role"] == "Button" && node["aria"]["label"] == "Close dialog"
        });

    assert!(
        close_button.is_some(),
        "dialog close button needs a readable label"
    );
}

#[gpui::test]
fn disabled_select_reports_its_disabled_state(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, cx| {
            Select::new(cx)
                .options(vec![SelectOption::new("dark", "Dark")])
                .selected_index(Some(0))
                .aria_label("Theme")
                .disabled(true)
        });
    });
    let select = node_with_role(&tree, "ComboBox");

    assert_eq!(select["aria"]["disabled"], true);
}

#[gpui::test]
fn disabled_checkbox_reports_its_disabled_state(cx: &mut TestAppContext) {
    let tree = draw_accessibility_tree(cx, |cx| {
        cx.add_window(|_, _| CheckboxView {
            checked: false,
            indeterminate: false,
            disabled: true,
        });
    });
    let checkbox = node_with_role(&tree, "CheckBox");

    assert_eq!(checkbox["aria"]["disabled"], true);
}
