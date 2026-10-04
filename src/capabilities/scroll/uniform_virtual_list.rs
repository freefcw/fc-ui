use std::cell::{Cell, RefCell};
use std::cmp;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use crate::capabilities::foundation::util::{AxisExt, PixelsExt};
use crate::capabilities::scroll::scroll_container::{PhysicsScrollState, ScrollDirection};
use gpui::{
    div, point, px, size, Along, AnyElement, App, AvailableSpace, Axis, Bounds, Context, Div,
    Edges, Element, ElementId, Entity, GlobalElementId, Hitbox, InteractiveElement, IntoElement,
    ListSizingBehavior, Pixels, Point, Render, ScrollStrategy, Size, Stateful,
    StatefulInteractiveElement, StyleRefinement, Styled, Window,
};
use smallvec::SmallVec;

/// A `scroll_to` request recorded during element construction and applied in
/// `prepaint`, once the viewport size for the frame is known.
#[derive(Clone, Copy)]
struct PendingScroll {
    index: usize,
    strategy: ScrollStrategy,
}

/// Largest legal scroll distance, mirroring GPUI's `Interactivity::clamp_scroll_position`.
///
/// The list layer must use the same formula as the scroll container it wraps.
/// If the two disagree, the list paints a frame from an offset GPUI is about to
/// reject, producing a visible jump on the next frame.
///
/// Callers must pass padding already snapped to the device-pixel grid
/// (`Window::pixel_snap`), matching upstream's `clamp_scroll_position` after
/// the phantom-scrollbar fix: layout snaps authored padding, so the recomputed
/// value must be snapped too or a fitting container looks scrollable by a
/// sub-pixel amount.
fn scroll_max(
    content_size: Size<Pixels>,
    padding: Edges<Pixels>,
    bounds_size: Size<Pixels>,
) -> Size<Pixels> {
    fn round_to_two_decimals(pixels: Pixels) -> Pixels {
        const ROUNDING_FACTOR: f32 = 100.0;
        (pixels * ROUNDING_FACTOR).round() / ROUNDING_FACTOR
    }

    let padding_size = size(padding.left + padding.right, padding.top + padding.bottom);
    (content_size + padding_size - bounds_size)
        .map(round_to_two_decimals)
        .max(&Size::default())
}

fn clamp_scroll_offset(offset: Point<Pixels>, scroll_max: Size<Pixels>) -> Point<Pixels> {
    point(
        offset.x.clamp(-scroll_max.width, px(0.)),
        offset.y.clamp(-scroll_max.height, px(0.)),
    )
}

/// Resolves a [`ScrollStrategy`] to a scroll distance along the list axis.
///
/// All arguments are extents along that axis, in the list's content coordinate
/// space where `0` is the start of the first item. The result is clamped into
/// `0..=max_scroll`, so items near either end resolve to the closest legal
/// position and different strategies may legitimately agree there.
fn scroll_target_for_strategy(
    item_start: Pixels,
    item_extent: Pixels,
    viewport_extent: Pixels,
    max_scroll: Pixels,
    strategy: ScrollStrategy,
) -> Pixels {
    let target = match strategy {
        ScrollStrategy::Top => item_start,
        ScrollStrategy::Center => item_start + item_extent / 2.0 - viewport_extent / 2.0,
        ScrollStrategy::Bottom => item_start + item_extent - viewport_extent,
    };
    target.max(px(0.)).min(max_scroll.max(px(0.)))
}

/// Resolve against the last completed layout. Deriving the viewport from the
/// content and maximum offset preserves the padding already encoded by GPUI.
fn target_from_laid_out_handle(
    handle: &gpui::ScrollHandle,
    axis: Axis,
    content_extent: Pixels,
    item_start: Pixels,
    item_extent: Pixels,
    strategy: ScrollStrategy,
) -> Option<Pixels> {
    if handle.bounds().size.along(axis) <= px(0.) {
        return None;
    }

    let content_extent = content_extent.max(px(0.));
    let max_scroll = handle
        .max_offset()
        .along(axis)
        .max(px(0.))
        .min(content_extent);
    let viewport_extent = (content_extent - max_scroll).max(px(0.));
    Some(scroll_target_for_strategy(
        item_start,
        item_extent,
        viewport_extent,
        max_scroll,
        strategy,
    ))
}

fn set_scroll_target(handle: &gpui::ScrollHandle, axis: Axis, target: Pixels) {
    let offset = handle.offset().apply_along(axis, |_| -target);
    handle.set_offset(offset);
}

pub struct UniformVirtualList {
    id: ElementId,
    axis: Axis,
    item_count: usize,
    item_extent: Pixels,
    overscan: usize,
    base: Stateful<Div>,
    scroll_handle: gpui::ScrollHandle,
    sizing_behavior: ListSizingBehavior,
    renderer: Box<
        dyn for<'a> Fn(Range<usize>, &'a mut Window, &'a mut App) -> SmallVec<[AnyElement; 64]>,
    >,
    on_visible_range: Option<Box<dyn Fn(Range<usize>, &mut Window, &mut App)>>,
    near_end_threshold: Option<(f32, Rc<RefCell<bool>>, Box<dyn Fn(&mut Window, &mut App)>)>,
    physics_state: Option<PhysicsScrollState>,
    pending_scroll: Cell<Option<PendingScroll>>,
}

impl Styled for UniformVirtualList {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl UniformVirtualList {
    pub fn new<R: IntoElement + 'static>(
        id: impl Into<ElementId>,
        axis: Axis,
        item_count: usize,
        item_extent: Pixels,
        renderer: impl 'static + Fn(Range<usize>, &mut Window, &mut App) -> Vec<R>,
    ) -> Self {
        let id = id.into();
        let renderer_boxed = move |range: Range<usize>, window: &mut Window, cx: &mut App| {
            renderer(range, window, cx)
                .into_iter()
                .map(|r| r.into_any_element())
                .collect::<SmallVec<[AnyElement; 64]>>()
        };

        Self {
            id: id.clone(),
            axis,
            item_count,
            item_extent,
            overscan: 5,
            base: div().id(id).size_full().overflow_scroll(),
            scroll_handle: gpui::ScrollHandle::new(),
            sizing_behavior: ListSizingBehavior::Auto,
            renderer: Box::new(renderer_boxed),
            on_visible_range: None,
            near_end_threshold: None,
            physics_state: None,
            pending_scroll: Cell::new(None),
        }
    }

    pub fn overscan(mut self, items: usize) -> Self {
        self.overscan = items;
        self
    }

    pub fn track_scroll(mut self, handle: &gpui::ScrollHandle) -> Self {
        self.base = self.base.track_scroll(handle);
        self.scroll_handle = handle.clone();
        self
    }

    pub fn with_sizing_behavior(mut self, behavior: ListSizingBehavior) -> Self {
        self.sizing_behavior = behavior;
        self
    }

    pub fn on_visible_range(
        mut self,
        f: impl 'static + Fn(Range<usize>, &mut Window, &mut App),
    ) -> Self {
        self.on_visible_range = Some(Box::new(f));
        self
    }

    pub fn on_near_end(
        mut self,
        threshold: f32,
        f: impl 'static + Fn(&mut Window, &mut App),
    ) -> Self {
        self.near_end_threshold = Some((
            threshold.clamp(0.0, 1.0),
            Rc::new(RefCell::new(false)),
            Box::new(f),
        ));
        self
    }

    /// Scroll `index` into view according to `strategy`.
    ///
    /// The item is always aligned to the strategy's position, even when it is
    /// already visible, and any in-flight animated scroll is cancelled: the
    /// latest command wins.
    ///
    /// When this handle has layout metrics, its offset is updated immediately.
    /// The request is also resolved during the next `prepaint` so first-layout
    /// calls and viewport changes use the current frame's exact geometry.
    pub fn scroll_to(&self, index: usize, strategy: ScrollStrategy) {
        if let Some(physics) = self.physics_state.as_ref() {
            physics.stop();
        }
        self.pending_scroll
            .set(Some(PendingScroll { index, strategy }));

        if let Some(target) = self.immediate_target(index, strategy) {
            set_scroll_target(&self.scroll_handle, self.axis, target);
        }
    }

    /// Best-effort target from the last completed layout's metrics. Returns
    /// `None` on the first frame for strategies that need the viewport;
    /// `prepaint` resolves the request with exact geometry in that case.
    fn immediate_target(&self, index: usize, strategy: ScrollStrategy) -> Option<Pixels> {
        let index = index.min(self.item_count.saturating_sub(1));
        let item_start = self.item_extent * index as f32;
        target_from_laid_out_handle(
            &self.scroll_handle,
            self.axis,
            self.item_extent * self.item_count as f32,
            item_start,
            self.item_extent,
            strategy,
        )
        .or(match strategy {
            ScrollStrategy::Top => Some(item_start),
            ScrollStrategy::Center | ScrollStrategy::Bottom => None,
        })
    }

    pub fn with_physics(mut self, state: &PhysicsScrollState) -> Self {
        let physics_c = state.clone();
        let handle_c = self.scroll_handle.clone();
        let dir = if self.axis.is_vertical() {
            ScrollDirection::Vertical
        } else {
            ScrollDirection::Horizontal
        };
        self.base = self.base.on_scroll_wheel(move |event, window, _cx| {
            physics_c.handle_scroll_event(&handle_c, dir, event, window);
        });
        self.physics_state = Some(state.clone());
        self
    }

    /// Scroll to `index` immediately, animating when physics is configured.
    pub fn scroll_to_animated(&self, index: usize, window: &Window) {
        // `Top` always resolves, with or without layout metrics.
        let target = self
            .immediate_target(index, ScrollStrategy::Top)
            .expect("ScrollStrategy::Top always resolves");

        if let Some(physics) = self.physics_state.as_ref() {
            self.pending_scroll.set(None);
            if self.axis.is_vertical() {
                physics.scroll_to_y_animated(target.as_f32(), &self.scroll_handle, window);
            } else {
                physics.scroll_to_x_animated(target.as_f32(), &self.scroll_handle, window);
            }
        } else {
            self.scroll_to(index, ScrollStrategy::Top);
        }
    }

    /// Consume a pending `scroll_to` and return the offset the frame should use.
    fn apply_pending_scroll(
        &self,
        offset: Point<Pixels>,
        viewport_extent: Pixels,
        max_scroll: Size<Pixels>,
    ) -> Point<Pixels> {
        let Some(pending) = self.pending_scroll.take() else {
            return offset;
        };
        let index = pending.index.min(self.item_count.saturating_sub(1));
        let target = scroll_target_for_strategy(
            self.item_extent * index as f32,
            self.item_extent,
            viewport_extent,
            max_scroll.along(self.axis),
            pending.strategy,
        );
        offset.apply_along(self.axis, |_| -target)
    }
}

pub struct UniformFrameState {
    items: SmallVec<[AnyElement; 32]>,
}

impl IntoElement for UniformVirtualList {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for UniformVirtualList {
    type RequestLayoutState = UniformFrameState;
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let axis = self.axis;
        let item_count = self.item_count;
        let item_extent = self.item_extent;
        let behavior = self.sizing_behavior;

        let layout_id = self.base.interactivity().request_layout(
            global_id,
            inspector_id,
            window,
            cx,
            move |style, window: &mut Window, cx: &mut App| match behavior {
                ListSizingBehavior::Infer => {
                    window.request_measured_layout(style, move |_k, available, _, _| {
                        let mut sz = Size::default();
                        if axis.is_horizontal() {
                            sz.width = match available.width {
                                AvailableSpace::Definite(w) => w,
                                _ => px(item_count as f32 * item_extent.as_f32()),
                            };
                            sz.height = match available.height {
                                AvailableSpace::Definite(h) => h,
                                _ => px(0.),
                            };
                        } else {
                            sz.width = match available.width {
                                AvailableSpace::Definite(w) => w,
                                _ => px(0.),
                            };
                            sz.height = match available.height {
                                AvailableSpace::Definite(h) => h,
                                _ => px(item_count as f32 * item_extent.as_f32()),
                            };
                        }
                        sz
                    })
                }
                ListSizingBehavior::Auto => window.request_layout(style, None, cx),
            },
        );

        (
            layout_id,
            UniformFrameState {
                items: SmallVec::new(),
            },
        )
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let style = self
            .base
            .interactivity()
            .compute_style(global_id, None, window, cx);
        let border = style.border_widths.to_pixels(window.rem_size());
        // Layout snaps authored padding to the device-pixel grid, so every
        // recomputation of it (content bounds, scroll max) must snap too.
        let padding = style
            .padding
            .to_pixels(bounds.size.into(), window.rem_size())
            .map(|edge| window.pixel_snap(*edge));

        let content_bounds = Bounds::from_corners(
            bounds.origin + point(border.left + padding.left, border.top + padding.top),
            bounds.bottom_right()
                - point(border.right + padding.right, border.bottom + padding.bottom),
        );

        let viewport_len = content_bounds.size.along(self.axis);
        let extent = self.item_extent;
        let total = px(self.item_count as f32 * extent.as_f32());

        let content_size = if self.axis.is_horizontal() {
            size(total, content_bounds.size.height)
        } else {
            size(content_bounds.size.width, total)
        };
        let max_scroll = scroll_max(content_size, padding, bounds.size);

        // Resolve any pending `scroll_to` and clamp before the visible range is
        // derived, so this frame's range, this frame's item positions and the
        // offset GPUI will settle on all agree.
        let offset =
            self.apply_pending_scroll(self.scroll_handle.offset(), viewport_len, max_scroll);
        let offset = clamp_scroll_offset(offset, max_scroll);
        self.scroll_handle.set_offset(offset);

        let base = -offset.along(self.axis);
        let first = if extent.as_f32() > 0.0 {
            (base.as_f32() / extent.as_f32()).floor().max(0.0) as usize
        } else {
            0
        };
        let last = if extent.as_f32() > 0.0 {
            ((base + viewport_len).as_f32() / extent.as_f32())
                .ceil()
                .max(0.0) as usize
        } else {
            0
        };

        let start = first.saturating_sub(self.overscan);
        let mut end = cmp::min(last + self.overscan, self.item_count);
        if end == 0 {
            end = cmp::min(self.item_count, self.overscan);
        }

        let visible = start..end;

        if let Some(cb) = &self.on_visible_range {
            cb(visible.clone(), window, cx);
        }

        if let Some((threshold, fired, cb)) = &self.near_end_threshold {
            let progress = if self.item_count == 0 {
                0.0
            } else {
                visible.end as f32 / self.item_count as f32
            };
            let mut was_fired = fired.borrow_mut();
            if progress >= *threshold && !*was_fired {
                *was_fired = true;
                cb(window, cx);
            }
            if progress < *threshold {
                *was_fired = false;
            }
        }

        let items = (self.renderer)(visible.clone(), window, cx);

        self.base.interactivity().prepaint(
            global_id,
            inspector_id,
            bounds,
            content_size,
            window,
            cx,
            |_style, _, hitbox, window, cx| {
                let available = match self.axis {
                    Axis::Horizontal => size(
                        AvailableSpace::Definite(extent),
                        AvailableSpace::Definite(content_bounds.size.height),
                    ),
                    Axis::Vertical => size(
                        AvailableSpace::Definite(content_bounds.size.width),
                        AvailableSpace::Definite(extent),
                    ),
                };

                for (mut item, ix) in items.into_iter().zip(visible) {
                    let item_origin = match self.axis {
                        Axis::Horizontal => {
                            content_bounds.origin
                                + point(px(ix as f32 * extent.as_f32()) + offset.x, offset.y)
                        }
                        Axis::Vertical => {
                            content_bounds.origin
                                + point(offset.x, px(ix as f32 * extent.as_f32()) + offset.y)
                        }
                    };
                    item.layout_as_root(available, window, cx);
                    item.prepaint_at(item_origin, window, cx);
                    layout.items.push(item);
                }

                hitbox
            },
        )
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.base.interactivity().paint(
            global_id,
            inspector_id,
            bounds,
            hitbox.as_ref(),
            window,
            cx,
            |_, window, cx| {
                for item in &mut layout.items {
                    item.paint(window, cx);
                }
            },
        )
    }
}

pub trait ItemExtentProvider {
    /// Return an item's extent. The list snapshots each value once during
    /// construction; non-finite and negative values are treated as zero.
    fn extent(&self, index: usize) -> Pixels;
}

const CHUNK_SIZE: usize = 1024;

struct ChunkedExtents<P: ItemExtentProvider> {
    provider: P,
    item_count: usize,
    item_extents: Vec<Pixels>,
    chunk_totals: Vec<Pixels>,
    chunk_offsets: Vec<Pixels>,
    intra_prefix: HashMap<usize, Rc<Vec<Pixels>>>,
}

impl<P: ItemExtentProvider> ChunkedExtents<P> {
    fn new(provider: P, item_count: usize) -> Self {
        let chunk_count = (item_count + CHUNK_SIZE - 1) / CHUNK_SIZE;
        Self {
            provider,
            item_count,
            item_extents: Vec::new(),
            chunk_totals: vec![px(0.0); chunk_count],
            chunk_offsets: vec![px(0.0); chunk_count + 1],
            intra_prefix: HashMap::new(),
        }
    }

    fn initialize_totals(&mut self) {
        if self.item_count == 0 {
            return;
        }
        if self.item_extents.is_empty() {
            self.item_extents = (0..self.item_count)
                .map(|index| Self::sanitize_extent(self.provider.extent(index)))
                .collect();
        }

        let chunk_count = self.chunk_totals.len();
        for c in 0..chunk_count {
            let start = c * CHUNK_SIZE;
            let end = ((c + 1) * CHUNK_SIZE).min(self.item_count);
            let sum = self.item_extents[start..end]
                .iter()
                .map(|extent| extent.as_f32())
                .sum();
            self.chunk_totals[c] = px(sum);
        }
        let mut accum = 0.0;
        self.chunk_offsets[0] = px(0.0);
        for c in 0..chunk_count {
            accum += self.chunk_totals[c].as_f32();
            self.chunk_offsets[c + 1] = px(accum);
        }
    }

    fn sanitize_extent(extent: Pixels) -> Pixels {
        let value = extent.as_f32();
        if value.is_finite() && value >= 0.0 {
            extent
        } else {
            px(0.0)
        }
    }

    fn total_extent(&self) -> Pixels {
        if self.chunk_offsets.is_empty() {
            return px(0.0);
        }
        *self.chunk_offsets.last().unwrap()
    }

    fn ensure_intra_prefix(&mut self, chunk_index: usize) -> Rc<Vec<Pixels>> {
        if let Some(v) = self.intra_prefix.get(&chunk_index) {
            return v.clone();
        }
        let start = chunk_index * CHUNK_SIZE;
        let end = ((chunk_index + 1) * CHUNK_SIZE).min(self.item_count);
        let mut origins = Vec::with_capacity(end - start);
        let mut sum = 0.0;
        for extent in &self.item_extents[start..end] {
            origins.push(px(sum));
            sum += extent.as_f32();
        }
        let rc = Rc::new(origins);
        self.intra_prefix.insert(chunk_index, rc.clone());
        rc
    }

    fn find_index_for_offset(&mut self, offset: Pixels) -> usize {
        if self.item_count == 0 {
            return 0;
        }
        let target = offset.as_f32();
        let mut lo = 0usize;
        let mut hi = self.chunk_offsets.len() - 1;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.chunk_offsets[mid].as_f32() <= target {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let chunk = lo
            .saturating_sub(1)
            .min(self.chunk_totals.len().saturating_sub(1));
        let chunk_base = self.chunk_offsets[chunk].as_f32();
        let within = target - chunk_base;
        let intra = self.ensure_intra_prefix(chunk);
        let mut lo_i = 0usize;
        let mut hi_i = intra.len();
        while lo_i < hi_i {
            let mid = (lo_i + hi_i) / 2;
            if intra[mid].as_f32() <= within {
                lo_i = mid + 1;
            } else {
                hi_i = mid;
            }
        }
        let idx_in_chunk = lo_i.saturating_sub(1).min(intra.len().saturating_sub(1));
        (chunk * CHUNK_SIZE + idx_in_chunk).min(self.item_count.saturating_sub(1))
    }

    fn item_origin(&mut self, index: usize) -> Pixels {
        if self.item_count == 0 {
            return px(0.0);
        }
        let chunk = index / CHUNK_SIZE;
        let intra_ix = index % CHUNK_SIZE;
        let intra = self.ensure_intra_prefix(chunk);
        self.chunk_offsets[chunk] + intra[intra_ix]
    }
}

pub struct VariableVirtualList<P: ItemExtentProvider> {
    id: ElementId,
    axis: Axis,
    overscan: usize,
    base: Stateful<Div>,
    scroll_handle: gpui::ScrollHandle,
    sizing_behavior: ListSizingBehavior,
    engine: ChunkedExtents<P>,
    renderer: Box<
        dyn for<'a> Fn(Range<usize>, &'a mut Window, &'a mut App) -> SmallVec<[AnyElement; 64]>,
    >,
    on_visible_range: Option<Box<dyn Fn(Range<usize>, &mut Window, &mut App)>>,
    near_end_threshold: Option<(f32, Rc<RefCell<bool>>, Box<dyn Fn(&mut Window, &mut App)>)>,
    physics_state: Option<PhysicsScrollState>,
    pending_scroll: Cell<Option<PendingScroll>>,
}

impl<P: ItemExtentProvider> Styled for VariableVirtualList<P> {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl<P: ItemExtentProvider + 'static> VariableVirtualList<P> {
    pub fn new<R: IntoElement + 'static>(
        id: impl Into<ElementId>,
        axis: Axis,
        item_count: usize,
        provider: P,
        renderer: impl 'static + Fn(Range<usize>, &mut Window, &mut App) -> Vec<R>,
    ) -> Self {
        let id = id.into();
        let renderer_boxed = move |range: Range<usize>, window: &mut Window, cx: &mut App| {
            renderer(range, window, cx)
                .into_iter()
                .map(|r| r.into_any_element())
                .collect::<SmallVec<[AnyElement; 64]>>()
        };

        let mut engine = ChunkedExtents::new(provider, item_count);
        engine.initialize_totals();

        Self {
            id: id.clone(),
            axis,
            overscan: 5,
            base: div().id(id).size_full().overflow_scroll(),
            scroll_handle: gpui::ScrollHandle::new(),
            sizing_behavior: ListSizingBehavior::Auto,
            engine,
            renderer: Box::new(renderer_boxed),
            on_visible_range: None,
            near_end_threshold: None,
            physics_state: None,
            pending_scroll: Cell::new(None),
        }
    }

    pub fn overscan(mut self, items: usize) -> Self {
        self.overscan = items;
        self
    }
    pub fn track_scroll(mut self, handle: &gpui::ScrollHandle) -> Self {
        self.base = self.base.track_scroll(handle);
        self.scroll_handle = handle.clone();
        self
    }
    pub fn with_sizing_behavior(mut self, behavior: ListSizingBehavior) -> Self {
        self.sizing_behavior = behavior;
        self
    }

    pub fn on_visible_range(
        mut self,
        f: impl 'static + Fn(Range<usize>, &mut Window, &mut App),
    ) -> Self {
        self.on_visible_range = Some(Box::new(f));
        self
    }

    pub fn on_near_end(
        mut self,
        threshold: f32,
        f: impl 'static + Fn(&mut Window, &mut App),
    ) -> Self {
        self.near_end_threshold = Some((
            threshold.clamp(0.0, 1.0),
            Rc::new(RefCell::new(false)),
            Box::new(f),
        ));
        self
    }

    /// Scroll `index` into view according to `strategy`.
    ///
    /// The item is always aligned to the strategy's position, even when it is
    /// already visible, and any in-flight animated scroll is cancelled: the
    /// latest command wins.
    ///
    /// When this handle has layout metrics, its offset is updated immediately.
    /// The request is also resolved during the next `prepaint` so first-layout
    /// calls and viewport changes use the current frame's exact geometry.
    pub fn scroll_to(&mut self, index: usize, strategy: ScrollStrategy) {
        if let Some(physics) = self.physics_state.as_ref() {
            physics.stop();
        }
        self.pending_scroll
            .set(Some(PendingScroll { index, strategy }));

        if let Some(target) = self.immediate_target(index, strategy) {
            set_scroll_target(&self.scroll_handle, self.axis, target);
        }
    }

    /// Best-effort target from the last completed layout's metrics. Returns
    /// `None` for empty lists and, on the first frame, for strategies that
    /// need the viewport; `prepaint` resolves those with exact geometry.
    fn immediate_target(&mut self, index: usize, strategy: ScrollStrategy) -> Option<Pixels> {
        if self.engine.item_count == 0 {
            return None;
        }
        let index = index.min(self.engine.item_count - 1);
        let item_start = self.engine.item_origin(index);
        target_from_laid_out_handle(
            &self.scroll_handle,
            self.axis,
            self.engine.total_extent(),
            item_start,
            self.engine.item_extents[index],
            strategy,
        )
        .or(match strategy {
            ScrollStrategy::Top => Some(item_start),
            ScrollStrategy::Center | ScrollStrategy::Bottom => None,
        })
    }

    pub fn with_physics(mut self, state: &PhysicsScrollState) -> Self {
        let physics_c = state.clone();
        let handle_c = self.scroll_handle.clone();
        let dir = if self.axis.is_vertical() {
            ScrollDirection::Vertical
        } else {
            ScrollDirection::Horizontal
        };
        self.base = self.base.on_scroll_wheel(move |event, window, _cx| {
            physics_c.handle_scroll_event(&handle_c, dir, event, window);
        });
        self.physics_state = Some(state.clone());
        self
    }

    /// Scroll to `index` immediately, animating when physics is configured.
    pub fn scroll_to_animated(&mut self, index: usize, window: &Window) {
        if self.engine.item_count == 0 {
            return;
        }
        // `Top` always resolves, with or without layout metrics.
        let target = self
            .immediate_target(index, ScrollStrategy::Top)
            .expect("ScrollStrategy::Top always resolves");

        if let Some(physics) = self.physics_state.as_ref() {
            self.pending_scroll.set(None);
            if self.axis.is_vertical() {
                physics.scroll_to_y_animated(target.as_f32(), &self.scroll_handle, window);
            } else {
                physics.scroll_to_x_animated(target.as_f32(), &self.scroll_handle, window);
            }
        } else {
            self.scroll_to(index, ScrollStrategy::Top);
        }
    }

    /// Consume a pending `scroll_to` and return the offset the frame should use.
    fn apply_pending_scroll(
        &mut self,
        offset: Point<Pixels>,
        viewport_extent: Pixels,
        max_scroll: Size<Pixels>,
    ) -> Point<Pixels> {
        let Some(pending) = self.pending_scroll.take() else {
            return offset;
        };
        if self.engine.item_count == 0 {
            return offset;
        }
        let index = pending.index.min(self.engine.item_count - 1);
        let target = scroll_target_for_strategy(
            self.engine.item_origin(index),
            self.engine.item_extents[index],
            viewport_extent,
            max_scroll.along(self.axis),
            pending.strategy,
        );
        offset.apply_along(self.axis, |_| -target)
    }
}

pub struct VariableFrameState {
    items: SmallVec<[AnyElement; 32]>,
}

impl<P: ItemExtentProvider + 'static> IntoElement for VariableVirtualList<P> {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl<P: ItemExtentProvider + 'static> Element for VariableVirtualList<P> {
    type RequestLayoutState = VariableFrameState;
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let axis = self.axis;
        let behavior = self.sizing_behavior;
        let engine_total = self.engine.total_extent();

        let layout_id = self.base.interactivity().request_layout(
            global_id,
            inspector_id,
            window,
            cx,
            move |style, window: &mut Window, cx: &mut App| match behavior {
                ListSizingBehavior::Infer => {
                    window.request_measured_layout(style, move |_k, available, _, _| {
                        let mut sz = Size::default();
                        if axis.is_horizontal() {
                            sz.width = match available.width {
                                AvailableSpace::Definite(w) => w,
                                _ => engine_total,
                            };
                            sz.height = match available.height {
                                AvailableSpace::Definite(h) => h,
                                _ => px(0.),
                            };
                        } else {
                            sz.width = match available.width {
                                AvailableSpace::Definite(w) => w,
                                _ => px(0.),
                            };
                            sz.height = match available.height {
                                AvailableSpace::Definite(h) => h,
                                _ => engine_total,
                            };
                        }
                        sz
                    })
                }
                ListSizingBehavior::Auto => window.request_layout(style, None, cx),
            },
        );
        (
            layout_id,
            VariableFrameState {
                items: SmallVec::new(),
            },
        )
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let style = self
            .base
            .interactivity()
            .compute_style(global_id, None, window, cx);
        let border = style.border_widths.to_pixels(window.rem_size());
        // Layout snaps authored padding to the device-pixel grid, so every
        // recomputation of it (content bounds, scroll max) must snap too.
        let padding = style
            .padding
            .to_pixels(bounds.size.into(), window.rem_size())
            .map(|edge| window.pixel_snap(*edge));

        let content_bounds = Bounds::from_corners(
            bounds.origin + point(border.left + padding.left, border.top + padding.top),
            bounds.bottom_right()
                - point(border.right + padding.right, border.bottom + padding.bottom),
        );

        let viewport_len = content_bounds.size.along(self.axis);
        let total = self.engine.total_extent();

        let content_size = if self.axis.is_horizontal() {
            size(total, content_bounds.size.height)
        } else {
            size(content_bounds.size.width, total)
        };
        let max_scroll = scroll_max(content_size, padding, bounds.size);

        // Resolve any pending `scroll_to` and clamp before the visible range is
        // derived, so this frame's range, this frame's item positions and the
        // offset GPUI will settle on all agree.
        let offset =
            self.apply_pending_scroll(self.scroll_handle.offset(), viewport_len, max_scroll);
        let offset = clamp_scroll_offset(offset, max_scroll);
        self.scroll_handle.set_offset(offset);

        let start_px = -offset.along(self.axis);
        let end_px = start_px + viewport_len;

        let mut start_ix = self.engine.find_index_for_offset(start_px.max(px(0.)));
        start_ix = start_ix.saturating_sub(self.overscan);

        let mut end_ix = self.engine.find_index_for_offset(end_px.max(px(0.)));
        end_ix = (end_ix + 1 + self.overscan).min(self.engine.item_count);

        let visible = start_ix..end_ix;

        if let Some(cb) = &self.on_visible_range {
            cb(visible.clone(), window, cx);
        }
        if let Some((threshold, fired, cb)) = &self.near_end_threshold {
            let progress = if self.engine.item_count == 0 {
                0.0
            } else {
                visible.end as f32 / self.engine.item_count as f32
            };
            let mut was_fired = fired.borrow_mut();
            if progress >= *threshold && !*was_fired {
                *was_fired = true;
                cb(window, cx);
            }
            if progress < *threshold {
                *was_fired = false;
            }
        }

        let items = (self.renderer)(visible.clone(), window, cx);

        self.base.interactivity().prepaint(
            global_id,
            inspector_id,
            bounds,
            content_size,
            window,
            cx,
            |_style, _, hitbox, window, cx| {
                for (mut item, ix) in items.into_iter().zip(visible) {
                    let origin_along = self.engine.item_origin(ix);
                    let item_origin = match self.axis {
                        Axis::Horizontal => {
                            content_bounds.origin + point(origin_along + offset.x, offset.y)
                        }
                        Axis::Vertical => {
                            content_bounds.origin + point(offset.x, origin_along + offset.y)
                        }
                    };

                    let available = match self.axis {
                        Axis::Horizontal => size(
                            AvailableSpace::Definite(px(CHUNK_SIZE as f32)),
                            AvailableSpace::Definite(content_bounds.size.height),
                        ),
                        Axis::Vertical => size(
                            AvailableSpace::Definite(content_bounds.size.width),
                            AvailableSpace::Definite(px(CHUNK_SIZE as f32)),
                        ),
                    };

                    item.layout_as_root(available, window, cx);
                    item.prepaint_at(item_origin, window, cx);
                    layout.items.push(item);
                }
                hitbox
            },
        )
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.base.interactivity().paint(
            global_id,
            inspector_id,
            bounds,
            hitbox.as_ref(),
            window,
            cx,
            |_, window, cx| {
                for item in &mut layout.items {
                    item.paint(window, cx);
                }
            },
        )
    }
}

pub fn vlist_uniform<R: IntoElement + 'static>(
    id: impl Into<ElementId>,
    item_count: usize,
    item_extent: Pixels,
    renderer: impl 'static + Fn(Range<usize>, &mut Window, &mut App) -> Vec<R>,
) -> UniformVirtualList {
    UniformVirtualList::new(id, Axis::Vertical, item_count, item_extent, renderer)
}

pub fn hlist_uniform<R: IntoElement + 'static>(
    id: impl Into<ElementId>,
    item_count: usize,
    item_extent: Pixels,
    renderer: impl 'static + Fn(Range<usize>, &mut Window, &mut App) -> Vec<R>,
) -> UniformVirtualList {
    UniformVirtualList::new(id, Axis::Horizontal, item_count, item_extent, renderer)
}

pub fn vlist_variable<R: IntoElement + 'static, P: ItemExtentProvider + 'static>(
    id: impl Into<ElementId>,
    item_count: usize,
    provider: P,
    renderer: impl 'static + Fn(Range<usize>, &mut Window, &mut App) -> Vec<R>,
) -> VariableVirtualList<P> {
    VariableVirtualList::new(id, Axis::Vertical, item_count, provider, renderer)
}

pub fn hlist_variable<R: IntoElement + 'static, P: ItemExtentProvider + 'static>(
    id: impl Into<ElementId>,
    item_count: usize,
    provider: P,
    renderer: impl 'static + Fn(Range<usize>, &mut Window, &mut App) -> Vec<R>,
) -> VariableVirtualList<P> {
    VariableVirtualList::new(id, Axis::Horizontal, item_count, provider, renderer)
}

pub fn vlist_uniform_view<R, V>(
    view: Entity<V>,
    id: impl Into<ElementId>,
    item_count: usize,
    item_extent: Pixels,
    f: impl 'static + Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R>,
) -> UniformVirtualList
where
    R: IntoElement,
    V: Render,
{
    let id: ElementId = id.into();
    let render_range = move |visible_range: Range<usize>, window: &mut Window, cx: &mut App| {
        view.update(cx, |this, cx| {
            f(this, visible_range, window, cx)
                .into_iter()
                .map(|component| component.into_any_element())
                .collect::<SmallVec<[AnyElement; 64]>>()
        })
    };

    UniformVirtualList::new(
        id,
        Axis::Vertical,
        item_count,
        item_extent,
        move |range, window, cx| {
            render_range(range, window, cx)
                .into_iter()
                .collect::<Vec<_>>()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{scroll_target_for_strategy, ChunkedExtents, ItemExtentProvider};
    use gpui::{px, Pixels, ScrollStrategy};
    use std::{cell::Cell, rc::Rc};

    struct CountingProvider {
        calls: Rc<Cell<usize>>,
        extents: Vec<f32>,
    }

    impl ItemExtentProvider for CountingProvider {
        fn extent(&self, index: usize) -> Pixels {
            self.calls.set(self.calls.get() + 1);
            px(self.extents[index])
        }
    }

    #[test]
    fn variable_extents_are_snapshotted_once_and_sanitized() {
        let calls = Rc::new(Cell::new(0));
        let mut extents = ChunkedExtents::new(
            CountingProvider {
                calls: calls.clone(),
                extents: vec![10.0, f32::NAN, -5.0, 20.0],
            },
            4,
        );

        extents.initialize_totals();
        let prefix = extents.ensure_intra_prefix(0);

        assert_eq!(calls.get(), 4);
        assert_eq!(extents.total_extent(), px(30.0));
        assert_eq!(
            prefix.as_ref(),
            &vec![px(0.0), px(10.0), px(10.0), px(10.0)]
        );
    }

    #[test]
    fn strategies_resolve_to_distinct_middle_positions() {
        let item_start = px(200.);
        let item_extent = px(20.);
        let viewport_extent = px(100.);
        let max_scroll = px(300.);

        assert_eq!(
            scroll_target_for_strategy(
                item_start,
                item_extent,
                viewport_extent,
                max_scroll,
                ScrollStrategy::Top,
            ),
            px(200.)
        );
        assert_eq!(
            scroll_target_for_strategy(
                item_start,
                item_extent,
                viewport_extent,
                max_scroll,
                ScrollStrategy::Center,
            ),
            px(160.)
        );
        assert_eq!(
            scroll_target_for_strategy(
                item_start,
                item_extent,
                viewport_extent,
                max_scroll,
                ScrollStrategy::Bottom,
            ),
            px(120.)
        );
    }

    #[test]
    fn strategy_targets_clamp_at_both_ends() {
        assert_eq!(
            scroll_target_for_strategy(px(0.), px(20.), px(100.), px(300.), ScrollStrategy::Bottom,),
            px(0.)
        );
        assert_eq!(
            scroll_target_for_strategy(px(380.), px(20.), px(100.), px(300.), ScrollStrategy::Top,),
            px(300.)
        );
    }
}
