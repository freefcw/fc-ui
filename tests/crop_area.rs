use fc_ui::components::crop_area::{CropArea, CropAreaState};
use gpui::{
    point, px, AppContext, Context, Entity, IntoElement, MouseButton, ParentElement, Render,
    Styled, TestAppContext, Window,
};

struct CropHost {
    state: Entity<CropAreaState>,
}

impl Render for CropHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui::div().p(px(20.)).child(
            CropArea::new("crop-regression", self.state.clone())
                .w(px(200.))
                .h(px(200.)),
        )
    }
}

#[gpui::test]
fn dragging_the_default_selection_uses_the_rendered_bounds(cx: &mut TestAppContext) {
    let state = cx.update(|cx| cx.new(CropAreaState::new));
    let state_for_view = state.clone();
    let (_, cx) = cx.add_window_view(move |_, _| CropHost {
        state: state_for_view,
    });
    cx.update(|window, cx| window.draw(cx).clear());

    cx.simulate_mouse_down(
        point(px(100.), px(100.)),
        MouseButton::Left,
        Default::default(),
    );
    cx.simulate_mouse_move(
        point(px(110.), px(110.)),
        MouseButton::Left,
        Default::default(),
    );
    cx.simulate_mouse_up(
        point(px(110.), px(110.)),
        MouseButton::Left,
        Default::default(),
    );

    cx.update(|_, cx| {
        let selection = state.read(cx).get_selection();
        assert!((selection.origin.x - 0.15).abs() < 0.001);
        assert!((selection.origin.y - 0.15).abs() < 0.001);
        assert_eq!(selection.size.width, 0.8);
        assert_eq!(selection.size.height, 0.8);
    });
}
