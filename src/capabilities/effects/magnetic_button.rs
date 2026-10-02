//! Button whose position subtly shifts toward cursor when nearby.

use gpui::{prelude::FluentBuilder as _, *};

pub struct MagneticButtonState {
    mouse_offset: Point<f32>,
    bounds: Option<Bounds<Pixels>>,
    is_hovering: bool,
}

impl MagneticButtonState {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            mouse_offset: Point::default(),
            bounds: None,
            is_hovering: false,
        }
    }
}

impl Render for MagneticButtonState {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(IntoElement)]
pub struct MagneticButton {
    id: ElementId,
    state: Entity<MagneticButtonState>,
    strength: f32,
    range: Pixels,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl MagneticButton {
    pub fn new(id: impl Into<ElementId>, state: Entity<MagneticButtonState>) -> Self {
        Self {
            id: id.into(),
            state,
            strength: 0.3,
            range: px(100.0),
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    pub fn strength(mut self, strength: f32) -> Self {
        self.strength = strength.clamp(0.0, 1.0);
        self
    }

    pub fn range(mut self, range: Pixels) -> Self {
        self.range = range;
        self
    }
}

impl Styled for MagneticButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for MagneticButton {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for MagneticButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let user_style = self.style;
        let state = self.state.read(cx);
        let strength = self.strength;
        let range = self.range;

        let (final_x, final_y) = if state.is_hovering {
            (
                state.mouse_offset.x * strength,
                state.mouse_offset.y * strength,
            )
        } else {
            (0.0, 0.0)
        };

        let state_bounds = self.state.clone();
        let state_move = self.state.clone();
        let state_hover = self.state.clone();

        div()
            .id(self.id)
            .relative()
            .cursor_pointer()
            .child(
                div()
                    .ml(px(final_x))
                    .mt(px(final_y))
                    .children(self.children),
            )
            .child(
                canvas(
                    move |bounds, _window, cx| {
                        state_bounds.update(cx, |s, _| s.bounds = Some(bounds));
                    },
                    |_bounds, _, _window, _cx| {},
                )
                .absolute()
                .inset_0()
                .size_full(),
            )
            .on_mouse_move(move |event: &MouseMoveEvent, _window, cx| {
                state_move.update(cx, |s, cx| {
                    if let Some(bounds) = s.bounds {
                        let range_f32: f32 = range.into();
                        let center_x: f32 = bounds.center().x.into();
                        let center_y: f32 = bounds.center().y.into();
                        let event_x: f32 = event.position.x.into();
                        let event_y: f32 = event.position.y.into();
                        s.mouse_offset.x = (event_x - center_x).clamp(-range_f32, range_f32);
                        s.mouse_offset.y = (event_y - center_y).clamp(-range_f32, range_f32);
                        cx.notify();
                    }
                });
            })
            .on_hover(move |hovered: &bool, _window, cx| {
                state_hover.update(cx, |s, cx| {
                    s.is_hovering = *hovered;
                    if !*hovered {
                        s.mouse_offset = Point::default();
                    }
                    cx.notify();
                });
            })
            .map(|this: Stateful<Div>| {
                let mut el = this;
                el.style().refine(&user_style);
                el
            })
    }
}
