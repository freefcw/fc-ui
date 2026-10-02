//! Reusable layout containers and composition primitives.

use gpui::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Justify {
    Start,
    Center,
    End,
    Between,
    Around,
    Evenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowDirection {
    Horizontal,
    Vertical,
}

pub struct VStack {
    base: Div,
    spacing: Option<Pixels>,
    align: Option<Align>,
}

impl Default for VStack {
    fn default() -> Self {
        Self::new()
    }
}

impl VStack {
    pub fn new() -> Self {
        Self {
            base: div().flex().flex_col(),
            spacing: None,
            align: None,
        }
    }

    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = Some(spacing.into());
        self
    }

    pub fn gap(self, gap: impl Into<Pixels>) -> Self {
        self.spacing(gap)
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }

    pub fn fill(mut self) -> Self {
        self.base = self.base.size_full();
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.base = self.base.w_full();
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.base = self.base.h_full();
        self
    }

    pub fn grow(mut self) -> Self {
        self.base = self.base.flex_1();
        self
    }

    pub fn padding(mut self, padding: impl Into<Pixels>) -> Self {
        self.base = self.base.p(padding.into());
        self
    }

    pub fn items_center(mut self) -> Self {
        self.align = Some(Align::Center);
        self
    }
}

impl ParentElement for VStack {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl Styled for VStack {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for VStack {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for VStack {}

impl IntoElement for VStack {
    type Element = Div;

    fn into_element(mut self) -> Self::Element {
        if let Some(spacing) = self.spacing {
            self.base = self.base.gap(spacing);
        }

        if let Some(align) = self.align {
            self.base = match align {
                Align::Start => self.base.items_start(),
                Align::Center => self.base.items_center(),
                Align::End => self.base.items_end(),
                Align::Stretch => self.base,
            };
        }

        self.base
    }
}

pub struct HStack {
    base: Div,
    spacing: Option<Pixels>,
    align: Option<Align>,
    justify: Option<Justify>,
}

impl Default for HStack {
    fn default() -> Self {
        Self::new()
    }
}

impl HStack {
    pub fn new() -> Self {
        Self {
            base: div().flex().flex_row(),
            spacing: None,
            align: None,
            justify: None,
        }
    }

    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = Some(spacing.into());
        self
    }

    pub fn gap(self, gap: impl Into<Pixels>) -> Self {
        self.spacing(gap)
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }

    pub fn justify(mut self, justify: Justify) -> Self {
        self.justify = Some(justify);
        self
    }

    pub fn fill(mut self) -> Self {
        self.base = self.base.size_full();
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.base = self.base.w_full();
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.base = self.base.h_full();
        self
    }

    pub fn grow(mut self) -> Self {
        self.base = self.base.flex_1();
        self
    }

    pub fn padding(mut self, padding: impl Into<Pixels>) -> Self {
        self.base = self.base.p(padding.into());
        self
    }

    pub fn items_center(mut self) -> Self {
        self.align = Some(Align::Center);
        self
    }

    pub fn space_between(mut self) -> Self {
        self.justify = Some(Justify::Between);
        self
    }
}

impl ParentElement for HStack {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl Styled for HStack {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for HStack {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for HStack {}

impl IntoElement for HStack {
    type Element = Div;

    fn into_element(mut self) -> Self::Element {
        if let Some(spacing) = self.spacing {
            self.base = self.base.gap(spacing);
        }

        if let Some(align) = self.align {
            self.base = match align {
                Align::Start => self.base.items_start(),
                Align::Center => self.base.items_center(),
                Align::End => self.base.items_end(),
                Align::Stretch => self.base,
            };
        }

        if let Some(justify) = self.justify {
            self.base = match justify {
                Justify::Start => self.base.justify_start(),
                Justify::Center => self.base.justify_center(),
                Justify::End => self.base.justify_end(),
                Justify::Between => self.base.justify_between(),
                Justify::Around => self.base.justify_around(),
                Justify::Evenly => {
                    // Styled has no justify_evenly() helper, but taffy supports it
                    let mut base = self.base;
                    base.style().justify_content = Some(JustifyContent::SpaceEvenly);
                    base
                }
            };
        }

        self.base
    }
}

pub struct Flow {
    base: Div,
    direction: FlowDirection,
    spacing: Option<Pixels>,
    align: Option<Align>,
}

impl Default for Flow {
    fn default() -> Self {
        Self::new()
    }
}

impl Flow {
    pub fn new() -> Self {
        Self {
            base: div().flex().flex_wrap(),
            direction: FlowDirection::Horizontal,
            spacing: None,
            align: None,
        }
    }

    pub fn direction(mut self, direction: FlowDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = Some(spacing.into());
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }
}

impl ParentElement for Flow {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl Styled for Flow {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Flow {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Flow {}

impl IntoElement for Flow {
    type Element = Div;

    fn into_element(mut self) -> Self::Element {
        self.base = match self.direction {
            FlowDirection::Horizontal => self.base.flex_row(),
            FlowDirection::Vertical => self.base.flex_col(),
        };

        if let Some(spacing) = self.spacing {
            self.base = self.base.gap(spacing);
        }

        if let Some(align) = self.align {
            self.base = match align {
                Align::Start => self.base.items_start(),
                Align::Center => self.base.items_center(),
                Align::End => self.base.items_end(),
                Align::Stretch => self.base,
            };
        }

        self.base
    }
}

pub struct Grid {
    base: Div,
    columns: usize,
    gap: Option<Pixels>,
    grid_children: Vec<AnyElement>,
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}

impl Grid {
    pub fn new() -> Self {
        Self {
            base: div().flex().flex_col(),
            columns: 1,
            gap: None,
            grid_children: vec![],
        }
    }

    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    pub fn gap(mut self, gap: impl Into<Pixels>) -> Self {
        self.gap = Some(gap.into());
        self
    }
}

impl ParentElement for Grid {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.grid_children.extend(elements);
    }
}

impl Styled for Grid {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Grid {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Grid {}

impl IntoElement for Grid {
    type Element = Div;

    fn into_element(mut self) -> Self::Element {
        if let Some(gap) = self.gap {
            self.base = self.base.gap(gap);
        }

        let total_children = self.grid_children.len();
        let mut rows = vec![];
        let mut current_row = vec![];

        for (i, child) in self.grid_children.into_iter().enumerate() {
            current_row.push(child);
            if (i + 1) % self.columns == 0 || i == total_children - 1 {
                rows.push(current_row);
                current_row = vec![];
            }
        }

        for row_children in rows {
            let mut row = div().flex().flex_row().w_full();

            if let Some(gap) = self.gap {
                row = row.gap(gap);
            }

            let cell_count = row_children.len();
            for child in row_children {
                row = row.child(div().flex_1().child(child));
            }
            // Pad the final row so its cells keep the same width as full rows
            for _ in cell_count..self.columns {
                row = row.child(div().flex_1());
            }

            self.base = self.base.child(row);
        }

        self.base
    }
}

pub struct Cluster {
    base: Div,
    spacing: Option<Pixels>,
    align: Option<Align>,
}

impl Default for Cluster {
    fn default() -> Self {
        Self::new()
    }
}

impl Cluster {
    pub fn new() -> Self {
        Self {
            base: div().flex().flex_row(),
            spacing: None,
            align: None,
        }
    }

    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = Some(spacing.into());
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }
}

impl ParentElement for Cluster {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl Styled for Cluster {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Cluster {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Cluster {}

impl IntoElement for Cluster {
    type Element = Div;

    fn into_element(mut self) -> Self::Element {
        if let Some(spacing) = self.spacing {
            self.base = self.base.gap(spacing);
        }

        if let Some(align) = self.align {
            self.base = match align {
                Align::Start => self.base.items_start(),
                Align::Center => self.base.items_center(),
                Align::End => self.base.items_end(),
                Align::Stretch => self.base,
            };
        }

        self.base
    }
}

pub struct Spacer {
    size: Option<Pixels>,
}

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

impl Spacer {
    pub fn new() -> Self {
        Self { size: None }
    }

    pub fn fixed(size: impl Into<Pixels>) -> Self {
        Self {
            size: Some(size.into()),
        }
    }
}

impl IntoElement for Spacer {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        if let Some(size) = self.size {
            div().size(size)
        } else {
            div().flex_1()
        }
    }
}

pub struct Panel {
    base: Div,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel {
    pub fn new() -> Self {
        Self { base: div() }
    }

    pub fn card(mut self) -> Self {
        self.base = self.base.border_1().rounded(px(8.0)).p(px(16.0));
        self
    }

    pub fn elevated(mut self) -> Self {
        self.base = self.base.border_1().rounded(px(8.0));
        self
    }

    pub fn section(mut self) -> Self {
        self.base = self.base.border_b_1().p(px(12.0));
        self
    }

    pub fn border(mut self) -> Self {
        self.base = self.base.border_1();
        self
    }

    pub fn rounded(mut self) -> Self {
        self.base = self.base.rounded(px(8.0));
        self
    }

    pub fn padded(mut self) -> Self {
        self.base = self.base.p(px(16.0));
        self
    }
}

impl ParentElement for Panel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl Styled for Panel {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Panel {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Panel {}

impl IntoElement for Panel {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        self.base
    }
}

pub struct Container {
    base: Div,
    max_width: Option<Pixels>,
    centered: bool,
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Container {
    pub fn new() -> Self {
        Self {
            base: div().w_full(),
            max_width: None,
            centered: false,
        }
    }

    pub fn max_w(mut self, width: impl Into<Pixels>) -> Self {
        self.max_width = Some(width.into());
        self
    }

    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    pub fn sm() -> Self {
        Self::new().max_w(px(640.0)).centered()
    }

    pub fn md() -> Self {
        Self::new().max_w(px(768.0)).centered()
    }

    pub fn lg() -> Self {
        Self::new().max_w(px(1024.0)).centered()
    }

    pub fn xl() -> Self {
        Self::new().max_w(px(1280.0)).centered()
    }

    pub fn xxl() -> Self {
        Self::new().max_w(px(1536.0)).centered()
    }
}

impl ParentElement for Container {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl Styled for Container {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Container {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Container {}

impl IntoElement for Container {
    type Element = Div;

    fn into_element(mut self) -> Self::Element {
        if let Some(max_width) = self.max_width {
            self.base = self.base.max_w(max_width);
        }

        if self.centered {
            self.base = self.base.mx_auto();
        }

        self.base
    }
}

pub struct MasonryItem {
    element: AnyElement,
    estimated_height: f32,
}

impl MasonryItem {
    pub fn new(element: impl IntoElement, estimated_height: f32) -> Self {
        Self {
            element: element.into_any_element(),
            estimated_height,
        }
    }
}

pub struct MasonryGrid {
    base: Div,
    columns: usize,
    gap: Option<Pixels>,
    default_estimated_height: f32,
    items: Vec<MasonryItem>,
}

impl Default for MasonryGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl MasonryGrid {
    pub fn new() -> Self {
        Self {
            base: div().flex().flex_row(),
            columns: 3,
            gap: None,
            default_estimated_height: 100.0,
            items: vec![],
        }
    }

    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    pub fn gap(mut self, gap: impl Into<Pixels>) -> Self {
        self.gap = Some(gap.into());
        self
    }

    /// Estimated height used to place items added without an explicit height
    /// (via `child()`/`extend()`). Defaults to 100px.
    pub fn default_estimated_height(mut self, height: f32) -> Self {
        self.default_estimated_height = height;
        self
    }

    pub fn fill(mut self) -> Self {
        self.base = self.base.size_full();
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.base = self.base.w_full();
        self
    }

    pub fn item(mut self, element: impl IntoElement, estimated_height: f32) -> Self {
        self.items.push(MasonryItem::new(element, estimated_height));
        self
    }

    pub fn items<I, E>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = (E, f32)>,
        E: IntoElement,
    {
        for (element, height) in items {
            self.items.push(MasonryItem::new(element, height));
        }
        self
    }
}

impl ParentElement for MasonryGrid {
    /// Items added through `child()`/`extend()` carry no explicit height and are
    /// placed using `default_estimated_height` (100px unless configured).
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        let estimated_height = self.default_estimated_height;
        for element in elements {
            self.items.push(MasonryItem {
                element,
                estimated_height,
            });
        }
    }
}

impl Styled for MasonryGrid {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for MasonryGrid {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for MasonryGrid {}

impl IntoElement for MasonryGrid {
    type Element = Div;

    fn into_element(mut self) -> Self::Element {
        if let Some(gap) = self.gap {
            self.base = self.base.gap(gap);
        }

        let mut column_heights: Vec<f32> = vec![0.0; self.columns];
        let mut column_items: Vec<Vec<AnyElement>> =
            (0..self.columns).map(|_| Vec::new()).collect();

        let gap_value: f32 = self.gap.map(|g| f32::from(g)).unwrap_or(0.0);

        for item in self.items {
            let min_column = column_heights
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(idx, _)| idx)
                .unwrap_or(0);

            if !column_items[min_column].is_empty() {
                column_heights[min_column] += gap_value;
            }
            column_heights[min_column] += item.estimated_height;
            column_items[min_column].push(item.element);
        }

        for column_children in column_items {
            let mut column = div().flex().flex_col().flex_1();

            if let Some(gap) = self.gap {
                column = column.gap(gap);
            }

            for child in column_children {
                column = column.child(child);
            }

            self.base = self.base.child(column);
        }

        self.base
    }
}

#[cfg(test)]
mod tests {
    use super::{HStack, VStack};
    use gpui::{div, prelude::*, px, size, Context, Render, TestAppContext, Window};

    #[derive(Clone, Copy)]
    enum RootDirection {
        Vertical,
        Horizontal,
    }

    struct AutoSizedRootView {
        direction: RootDirection,
    }

    impl Render for AutoSizedRootView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            match self.direction {
                RootDirection::Vertical => VStack::new()
                    .debug_selector(|| "auto-sized-stack-root".into())
                    .child(div().size_full())
                    .into_any_element(),
                RootDirection::Horizontal => HStack::new()
                    .debug_selector(|| "auto-sized-stack-root".into())
                    .child(div().size_full())
                    .into_any_element(),
            }
        }
    }

    fn assert_auto_sized_root_fills_viewport(direction: RootDirection, cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(move |_, _| AutoSizedRootView { direction });

        cx.update(|window, cx| window.draw(cx).clear());
        let initial_viewport = cx.update(|window, _| window.viewport_size());
        let initial_root = cx
            .debug_bounds("auto-sized-stack-root")
            .expect("the stack root should expose its bounds");
        assert_eq!(initial_root.size, initial_viewport);

        let resized_viewport = size(px(640.0), px(360.0));
        cx.simulate_resize(resized_viewport);
        cx.update(|window, cx| window.draw(cx).clear());
        let resized_root = cx
            .debug_bounds("auto-sized-stack-root")
            .expect("the resized stack root should expose its bounds");
        assert_eq!(resized_root.size, resized_viewport);
    }

    #[gpui::test]
    fn vstack_direct_window_root_fills_the_viewport(cx: &mut TestAppContext) {
        assert_auto_sized_root_fills_viewport(RootDirection::Vertical, cx);
    }

    #[gpui::test]
    fn hstack_direct_window_root_fills_the_viewport(cx: &mut TestAppContext) {
        assert_auto_sized_root_fills_viewport(RootDirection::Horizontal, cx);
    }
}
