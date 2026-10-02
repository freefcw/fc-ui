use crate::capabilities::data::palette::default_color;
use crate::capabilities::foundation::theme::use_theme;
use gpui::{prelude::FluentBuilder as _, *};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct DataPoint {
    pub x: f64,
    pub y: f64,
    pub label: Option<SharedString>,
}

impl DataPoint {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y, label: None }
    }

    pub fn xy(x: f64, y: f64) -> Self {
        Self::new(x, y)
    }

    pub fn labeled(x: f64, y: f64, label: impl Into<SharedString>) -> Self {
        Self {
            x,
            y,
            label: Some(label.into()),
        }
    }
}

/// Whether a data point can safely participate in range and geometry calculations.
pub(crate) fn is_finite_data_point(x: f64, y: f64) -> bool {
    x.is_finite() && y.is_finite()
}

#[derive(Clone, Debug)]
pub struct DataRange {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

impl DataRange {
    /// Construct a finite range with ordered, non-zero axes.
    pub fn new(x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> Self {
        let (x_min, x_max) = Self::finite_bounds(x_min, x_max);
        let (y_min, y_max) = Self::finite_bounds(y_min, y_max);
        Self {
            x_min,
            x_max,
            y_min,
            y_max,
        }
    }

    fn empty() -> Self {
        Self {
            x_min: f64::MAX,
            x_max: f64::MIN,
            y_min: f64::MAX,
            y_max: f64::MIN,
        }
    }

    fn finite_bounds(min: f64, max: f64) -> (f64, f64) {
        if !min.is_finite() || !max.is_finite() {
            return (0.0, 1.0);
        }

        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        if (max - min).abs() < f64::EPSILON {
            let padding = (min.abs() * 0.01).max(0.5);
            let padded_min = (min - padding).max(f64::MIN);
            let padded_max = (max + padding).min(f64::MAX);
            (padded_min, padded_max)
        } else {
            (min, max)
        }
    }

    pub fn from_points(points: &[DataPoint]) -> Self {
        let mut x_min = f64::MAX;
        let mut x_max = f64::MIN;
        let mut y_min = f64::MAX;
        let mut y_max = f64::MIN;
        let mut has_finite_point = false;

        for point in points {
            if !is_finite_data_point(point.x, point.y) {
                continue;
            }
            has_finite_point = true;
            x_min = x_min.min(point.x);
            x_max = x_max.max(point.x);
            y_min = y_min.min(point.y);
            y_max = y_max.max(point.y);
        }

        if !has_finite_point {
            return Self::default();
        }

        if (x_max - x_min).abs() < f64::EPSILON {
            x_max = x_min + 1.0;
        }
        if (y_max - y_min).abs() < f64::EPSILON {
            y_max = y_min + 1.0;
        }

        Self::new(x_min, x_max, y_min, y_max)
    }

    pub fn extend(&mut self, other: &DataRange) {
        self.x_min = self.x_min.min(other.x_min);
        self.x_max = self.x_max.max(other.x_max);
        self.y_min = self.y_min.min(other.y_min);
        self.y_max = self.y_max.max(other.y_max);
    }

    pub fn normalize_x(&self, x: f64) -> f32 {
        if !x.is_finite() {
            return 0.0;
        }
        let (min, max) = Self::finite_bounds(self.x_min, self.x_max);
        ((x - min) / (max - min)) as f32
    }

    pub fn normalize_y(&self, y: f64) -> f32 {
        if !y.is_finite() {
            return 0.0;
        }
        let (min, max) = Self::finite_bounds(self.y_min, self.y_max);
        ((y - min) / (max - min)) as f32
    }

    pub fn with_padding(&self, padding: f64) -> Self {
        let x_range = self.x_max - self.x_min;
        let y_range = self.y_max - self.y_min;
        Self::new(
            self.x_min - x_range * padding,
            self.x_max + x_range * padding,
            self.y_min - y_range * padding,
            self.y_max + y_range * padding,
        )
    }
}

impl Default for DataRange {
    fn default() -> Self {
        Self::new(0.0, 1.0, 0.0, 1.0)
    }
}

#[derive(Clone, Default)]
pub struct ChartPadding {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl ChartPadding {
    pub fn new(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    pub fn all(value: f32) -> Self {
        Self::new(value, value, value, value)
    }
}

#[derive(Clone)]
pub struct ChartArea {
    pub bounds: Bounds<Pixels>,
    pub range: DataRange,
    pub padding: ChartPadding,
}

impl ChartArea {
    pub fn chart_left(&self) -> Pixels {
        self.bounds.left() + px(self.padding.left)
    }

    pub fn chart_right(&self) -> Pixels {
        self.bounds.right() - px(self.padding.right)
    }

    pub fn chart_top(&self) -> Pixels {
        self.bounds.top() + px(self.padding.top)
    }

    pub fn chart_bottom(&self) -> Pixels {
        self.bounds.bottom() - px(self.padding.bottom)
    }

    pub fn chart_width(&self) -> Pixels {
        self.chart_right() - self.chart_left()
    }

    pub fn chart_height(&self) -> Pixels {
        self.chart_bottom() - self.chart_top()
    }

    pub fn data_to_screen(&self, data_point: &DataPoint) -> Point<Pixels> {
        let norm_x = self.range.normalize_x(data_point.x);
        let norm_y = self.range.normalize_y(data_point.y);
        let screen_x = self.chart_left() + self.chart_width() * norm_x;
        let screen_y = self.chart_bottom() - self.chart_height() * norm_y;
        point(screen_x, screen_y)
    }

    pub fn screen_to_data(&self, screen_point: Point<Pixels>) -> DataPoint {
        let chart_width = self.chart_width();
        let chart_height = self.chart_height();

        let norm_x = (screen_point.x - self.chart_left()) / chart_width;
        let norm_y = 1.0 - (screen_point.y - self.chart_top()) / chart_height;

        let x = self.range.x_min + (self.range.x_max - self.range.x_min) * norm_x as f64;
        let y = self.range.y_min + (self.range.y_max - self.range.y_min) * norm_y as f64;

        DataPoint::new(x, y)
    }
}

#[derive(Clone, Default)]
pub enum AxisPosition {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone)]
pub struct Axis {
    /// Currently not rendered.
    pub label: Option<SharedString>,
    /// Currently not honored; the x-axis always renders on the bottom and the
    /// y-axis on the left.
    pub position: AxisPosition,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub tick_count: usize,
    pub tick_format: Option<Rc<dyn Fn(f64) -> String>>,
    pub show_grid: bool,
    /// Currently not rendered.
    pub show_ticks: bool,
    pub show_labels: bool,
}

impl Default for Axis {
    fn default() -> Self {
        Self {
            label: None,
            position: AxisPosition::default(),
            min: None,
            max: None,
            tick_count: 5,
            tick_format: None,
            show_grid: true,
            show_ticks: true,
            show_labels: true,
        }
    }
}

impl Axis {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn position(mut self, position: AxisPosition) -> Self {
        self.position = position;
        self
    }

    pub fn left(mut self) -> Self {
        self.position = AxisPosition::Left;
        self
    }

    pub fn right(mut self) -> Self {
        self.position = AxisPosition::Right;
        self
    }

    pub fn top(mut self) -> Self {
        self.position = AxisPosition::Top;
        self
    }

    pub fn bottom(mut self) -> Self {
        self.position = AxisPosition::Bottom;
        self
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    pub fn tick_count(mut self, count: usize) -> Self {
        self.tick_count = count;
        self
    }

    pub fn tick_format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.tick_format = Some(Rc::new(format));
        self
    }

    pub fn show_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    pub fn show_ticks(mut self, show: bool) -> Self {
        self.show_ticks = show;
        self
    }

    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    fn format_value(&self, value: f64) -> String {
        if let Some(ref format) = self.tick_format {
            format(value)
        } else if value.abs() >= 1000.0 {
            format!("{:.0}k", value / 1000.0)
        } else if value.abs() >= 1.0 {
            format!("{:.0}", value)
        } else {
            format!("{:.2}", value)
        }
    }
}

#[derive(Clone, Default)]
pub enum LegendPosition {
    Top,
    #[default]
    Bottom,
    Left,
    Right,
}

#[derive(Clone)]
pub struct Legend {
    /// Currently not honored; the legend always renders below the chart.
    pub position: LegendPosition,
    pub show: bool,
}

impl Default for Legend {
    fn default() -> Self {
        Self {
            position: LegendPosition::default(),
            show: true,
        }
    }
}

impl Legend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn position(mut self, position: LegendPosition) -> Self {
        self.position = position;
        self
    }

    pub fn top(mut self) -> Self {
        self.position = LegendPosition::Top;
        self
    }

    pub fn bottom(mut self) -> Self {
        self.position = LegendPosition::Bottom;
        self
    }

    pub fn left(mut self) -> Self {
        self.position = LegendPosition::Left;
        self
    }

    pub fn right(mut self) -> Self {
        self.position = LegendPosition::Right;
        self
    }

    pub fn show(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    pub fn hide(mut self) -> Self {
        self.show = false;
        self
    }
}

#[derive(Clone, Default)]
pub struct TooltipConfig {
    pub show: bool,
    pub format: Option<Rc<dyn Fn(&DataPoint, &str) -> String>>,
}

impl TooltipConfig {
    pub fn new() -> Self {
        Self {
            show: true,
            format: None,
        }
    }

    pub fn format(mut self, format: impl Fn(&DataPoint, &str) -> String + 'static) -> Self {
        self.format = Some(Rc::new(format));
        self
    }

    pub fn show(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    fn format_tooltip(&self, point: &DataPoint, series_name: &str) -> String {
        if let Some(ref format) = self.format {
            format(point, series_name)
        } else {
            let label = point
                .label
                .clone()
                .unwrap_or_else(|| format!("x: {:.2}", point.x).into());
            format!("{}: {} = {:.2}", series_name, label, point.y)
        }
    }
}

#[derive(Clone, Copy, Default)]
pub enum SeriesType {
    #[default]
    Line,
    Bar,
    Area,
    Scatter,
}

#[derive(Clone)]
pub struct Series {
    pub name: SharedString,
    pub data: Vec<DataPoint>,
    pub series_type: SeriesType,
    pub color: Option<Hsla>,
    pub stroke_width: f32,
    pub show_points: bool,
    pub point_radius: f32,
    pub fill_opacity: f32,
    pub smooth: bool,
    pub bar_width: Option<f32>,
}

impl Series {
    pub fn new(name: impl Into<SharedString>, data: Vec<DataPoint>) -> Self {
        Self {
            name: name.into(),
            data,
            series_type: SeriesType::Line,
            color: None,
            stroke_width: 2.0,
            show_points: false,
            point_radius: 4.0,
            fill_opacity: 0.2,
            smooth: false,
            bar_width: None,
        }
    }

    pub fn line(name: impl Into<SharedString>, data: Vec<DataPoint>) -> Self {
        Self::new(name, data).series_type(SeriesType::Line)
    }

    pub fn bar(name: impl Into<SharedString>, data: Vec<DataPoint>) -> Self {
        Self::new(name, data).series_type(SeriesType::Bar)
    }

    pub fn area(name: impl Into<SharedString>, data: Vec<DataPoint>) -> Self {
        Self::new(name, data)
            .series_type(SeriesType::Area)
            .fill_opacity(0.3)
    }

    pub fn scatter(name: impl Into<SharedString>, data: Vec<DataPoint>) -> Self {
        Self::new(name, data)
            .series_type(SeriesType::Scatter)
            .show_points(true)
    }

    pub fn series_type(mut self, series_type: SeriesType) -> Self {
        self.series_type = series_type;
        self
    }

    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    pub fn show_points(mut self, show: bool) -> Self {
        self.show_points = show;
        self
    }

    pub fn point_radius(mut self, radius: f32) -> Self {
        self.point_radius = radius;
        self
    }

    pub fn fill_opacity(mut self, opacity: f32) -> Self {
        self.fill_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    pub fn smooth(mut self, smooth: bool) -> Self {
        self.smooth = smooth;
        self
    }

    pub fn bar_width(mut self, width: f32) -> Self {
        self.bar_width = Some(width);
        self
    }

    pub fn data_range(&self) -> DataRange {
        DataRange::from_points(&self.data)
    }
}

struct HoveredPoint {
    series_index: usize,
    #[allow(dead_code)]
    point_index: usize,
    screen_pos: Point<Pixels>,
    data_point: DataPoint,
}

struct ChartPaintState {
    series: Vec<Series>,
    x_axis: Axis,
    y_axis: Axis,
    tooltip: TooltipConfig,
    grid_color: Hsla,
    #[allow(dead_code)]
    text_color: Hsla,
    #[allow(dead_code)]
    background: Hsla,
    padding: ChartPadding,
}

#[derive(IntoElement)]
pub struct Chart {
    series: Vec<Series>,
    x_axis: Axis,
    y_axis: Axis,
    legend: Legend,
    tooltip: TooltipConfig,
    style: StyleRefinement,
}

impl Default for Chart {
    fn default() -> Self {
        Self::new()
    }
}

impl Chart {
    pub fn new() -> Self {
        Self {
            series: Vec::new(),
            x_axis: Axis::new().bottom(),
            y_axis: Axis::new().left(),
            legend: Legend::new(),
            tooltip: TooltipConfig::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    pub fn add_series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    pub fn legend(mut self, legend: Legend) -> Self {
        self.legend = legend;
        self
    }

    pub fn show_legend(mut self, show: bool) -> Self {
        self.legend.show = show;
        self
    }

    pub fn tooltip(mut self, tooltip: TooltipConfig) -> Self {
        self.tooltip = tooltip;
        self
    }

    pub fn show_tooltip(mut self, show: bool) -> Self {
        self.tooltip.show = show;
        self
    }

    fn compute_data_range(&self) -> DataRange {
        let mut range = DataRange::empty();

        for series in &self.series {
            let series_range = series.data_range();
            range.extend(&series_range);
        }

        if range.x_min == f64::MAX {
            range = DataRange::new(0.0, 1.0, 0.0, 1.0);
        }

        if let Some(min) = self.x_axis.min {
            range.x_min = min;
        }
        if let Some(max) = self.x_axis.max {
            range.x_max = max;
        }
        if let Some(min) = self.y_axis.min {
            range.y_min = min;
        }
        if let Some(max) = self.y_axis.max {
            range.y_max = max;
        }

        DataRange::new(range.x_min, range.x_max, range.y_min, range.y_max)
    }
}

impl Styled for Chart {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

#[cfg(test)]
mod tests {
    use super::{DataPoint, DataRange};

    #[test]
    fn data_range_normalization_is_finite_for_invalid_public_inputs() {
        for range in [
            DataRange::new(1.0, 1.0, 2.0, 2.0),
            DataRange::new(2.0, -2.0, 4.0, -4.0),
            DataRange::new(f64::NAN, 1.0, 0.0, f64::INFINITY),
        ] {
            assert!(range.normalize_x(0.0).is_finite());
            assert!(range.normalize_y(0.0).is_finite());
            assert!(range.x_min < range.x_max);
            assert!(range.y_min < range.y_max);
        }
    }

    #[test]
    fn extreme_finite_values_produce_finite_ranges() {
        for value in [f64::MAX, f64::MIN] {
            for range in [
                DataRange::new(value, value, 0.0, 1.0),
                DataRange::from_points(&[DataPoint::new(value, 0.0)]),
            ] {
                assert!(range.x_min.is_finite());
                assert!(range.x_max.is_finite());
                assert!(range.x_min < range.x_max);
                assert!(range.x_min <= value && value <= range.x_max);
                assert!(range.normalize_x(value).is_finite());
            }
        }
    }

    #[test]
    fn data_range_ignores_non_finite_points() {
        let range = DataRange::from_points(&[
            DataPoint::new(f64::NAN, 1.0),
            DataPoint::new(2.0, f64::INFINITY),
            DataPoint::new(3.0, 4.0),
        ]);

        assert!(range.x_min <= 3.0 && range.x_max >= 3.0);
        assert!(range.y_min <= 4.0 && range.y_max >= 4.0);
        assert!(range.normalize_x(3.0).is_finite());
        assert!(range.normalize_y(4.0).is_finite());
    }

    #[test]
    fn single_finite_point_keeps_the_existing_one_unit_range() {
        let range = DataRange::from_points(&[DataPoint::new(3.0, 4.0)]);

        assert_eq!(range.x_min, 3.0);
        assert_eq!(range.x_max, 4.0);
        assert_eq!(range.y_min, 4.0);
        assert_eq!(range.y_max, 5.0);
    }

    #[test]
    fn data_range_defaults_when_no_points_are_finite() {
        let range = DataRange::from_points(&[
            DataPoint::new(f64::NAN, 1.0),
            DataPoint::new(2.0, f64::INFINITY),
        ]);

        assert_eq!(range.x_min, 0.0);
        assert_eq!(range.x_max, 1.0);
        assert_eq!(range.y_min, 0.0);
        assert_eq!(range.y_max, 1.0);
    }
}

impl RenderOnce for Chart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);

        let show_y_axis = self.y_axis.show_labels;
        let show_x_axis = self.x_axis.show_labels;

        let padding = ChartPadding::new(
            if show_y_axis { 60.0 } else { 20.0 },
            20.0,
            20.0,
            if show_x_axis { 40.0 } else { 20.0 },
        );

        let data_range = self.compute_data_range();
        let user_style = self.style;
        let series_for_legend = self.series.clone();
        let legend = self.legend.clone();

        let y_axis_clone = self.y_axis.clone();
        let y_labels: Vec<String> = if show_y_axis {
            (0..=self.y_axis.tick_count)
                .map(|i| {
                    let normalized = i as f64 / self.y_axis.tick_count as f64;
                    let value = data_range.y_min
                        + (data_range.y_max - data_range.y_min) * (1.0 - normalized);
                    y_axis_clone.format_value(value)
                })
                .collect()
        } else {
            Vec::new()
        };

        let paint_state = ChartPaintState {
            series: self.series,
            x_axis: self.x_axis,
            y_axis: self.y_axis,
            tooltip: self.tooltip,
            grid_color: theme.tokens.border,
            text_color: theme.tokens.muted_foreground,
            background: theme.tokens.background,
            padding: padding.clone(),
        };

        let text_color = theme.tokens.muted_foreground;
        let tooltip_bg = theme.tokens.popover;
        let tooltip_border = theme.tokens.border;

        div()
            .flex()
            .flex_col()
            .size_full()
            .min_h(px(250.0))
            .map(|this| {
                let mut d = this;
                d.style().refine(&user_style);
                d
            })
            .child(
                div()
                    .flex_1()
                    .relative()
                    .child(
                        canvas(
                            move |bounds, window, _cx| {
                                let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                                (paint_state, data_range, bounds, hitbox)
                            },
                            move |bounds, (state, range, _, hitbox), window, cx| {
                                let hitbox_for_event = hitbox.clone();
                                window.on_mouse_event(
                                    move |_event: &MouseMoveEvent, _phase, window, cx| {
                                        if hitbox_for_event.is_hovered(window) {
                                            cx.refresh_windows();
                                        }
                                    },
                                );

                                let mouse_pos = window.mouse_position();
                                if bounds.size.width <= px(0.0) || bounds.size.height <= px(0.0) {
                                    return;
                                }

                                let area = ChartArea {
                                    bounds,
                                    range: range.clone(),
                                    padding: state.padding.clone(),
                                };

                                if area.chart_width() <= px(0.0) || area.chart_height() <= px(0.0) {
                                    return;
                                }

                                if state.y_axis.show_grid {
                                    for i in 0..=state.y_axis.tick_count {
                                        let y = area.chart_top()
                                            + area.chart_height()
                                                * (i as f32 / state.y_axis.tick_count as f32);
                                        let mut builder = PathBuilder::stroke(px(1.0));
                                        builder.move_to(point(area.chart_left(), y));
                                        builder.line_to(point(area.chart_right(), y));
                                        if let Ok(path) = builder.build() {
                                            window.paint_path(path, state.grid_color.opacity(0.2));
                                        }
                                    }
                                }

                                if state.x_axis.show_grid {
                                    for i in 0..=state.x_axis.tick_count {
                                        let x = area.chart_left()
                                            + area.chart_width()
                                                * (i as f32 / state.x_axis.tick_count as f32);
                                        let mut builder = PathBuilder::stroke(px(1.0));
                                        builder.move_to(point(x, area.chart_top()));
                                        builder.line_to(point(x, area.chart_bottom()));
                                        if let Ok(path) = builder.build() {
                                            window.paint_path(path, state.grid_color.opacity(0.2));
                                        }
                                    }
                                }

                                let mut hovered_point: Option<HoveredPoint> = None;
                                let hover_radius = px(15.0);

                                for (series_index, series) in state.series.iter().enumerate() {
                                    let color =
                                        series.color.unwrap_or_else(|| default_color(series_index));

                                    if series.data.is_empty() {
                                        continue;
                                    }

                                    let screen_points: Vec<Point<Pixels>> = series
                                        .data
                                        .iter()
                                        .map(|p| area.data_to_screen(p))
                                        .collect();

                                    match series.series_type {
                                        SeriesType::Line | SeriesType::Area => {
                                            if matches!(series.series_type, SeriesType::Area)
                                                && screen_points.len() >= 2
                                            {
                                                let mut builder = PathBuilder::fill();
                                                builder.move_to(point(
                                                    screen_points[0].x,
                                                    area.chart_bottom(),
                                                ));
                                                builder.line_to(screen_points[0]);

                                                for pt in screen_points.iter().skip(1) {
                                                    builder.line_to(*pt);
                                                }

                                                builder.line_to(point(
                                                    screen_points.last().unwrap().x,
                                                    area.chart_bottom(),
                                                ));
                                                builder.close();

                                                if let Ok(path) = builder.build() {
                                                    window.paint_path(
                                                        path,
                                                        color.opacity(series.fill_opacity),
                                                    );
                                                }
                                            }

                                            if screen_points.len() >= 2 {
                                                let mut builder =
                                                    PathBuilder::stroke(px(series.stroke_width));
                                                builder.move_to(screen_points[0]);

                                                if series.smooth && screen_points.len() >= 3 {
                                                    for i in 0..screen_points.len() - 1 {
                                                        let p0 = screen_points[i];
                                                        let p1 = screen_points[i + 1];
                                                        let ctrl_x = (p0.x + p1.x) * 0.5;
                                                        builder.curve_to(p1, point(ctrl_x, p0.y));
                                                    }
                                                } else {
                                                    for pt in screen_points.iter().skip(1) {
                                                        builder.line_to(*pt);
                                                    }
                                                }

                                                if let Ok(path) = builder.build() {
                                                    window.paint_path(path, color);
                                                }
                                            }
                                        }
                                        SeriesType::Bar => {
                                            let bar_width = series.bar_width.unwrap_or(20.0);
                                            for (point_index, (data_pt, screen_pt)) in series
                                                .data
                                                .iter()
                                                .zip(screen_points.iter())
                                                .enumerate()
                                            {
                                                let bar_height = area.chart_bottom() - screen_pt.y;
                                                let bar_bounds = Bounds::new(
                                                    point(
                                                        screen_pt.x - px(bar_width / 2.0),
                                                        screen_pt.y,
                                                    ),
                                                    size(px(bar_width), bar_height),
                                                );

                                                window.paint_quad(fill(bar_bounds, color));

                                                if bar_bounds.contains(&mouse_pos) {
                                                    hovered_point = Some(HoveredPoint {
                                                        series_index,
                                                        point_index,
                                                        screen_pos: *screen_pt,
                                                        data_point: data_pt.clone(),
                                                    });
                                                }
                                            }
                                        }
                                        SeriesType::Scatter => {}
                                    }

                                    if series.show_points
                                        || matches!(series.series_type, SeriesType::Scatter)
                                    {
                                        let radius = px(series.point_radius);
                                        for (point_index, (data_pt, screen_pt)) in
                                            series.data.iter().zip(screen_points.iter()).enumerate()
                                        {
                                            let is_hovered = (mouse_pos.x - screen_pt.x).abs()
                                                < hover_radius
                                                && (mouse_pos.y - screen_pt.y).abs() < hover_radius;

                                            let point_radius =
                                                if is_hovered { radius * 1.5 } else { radius };

                                            window.paint_quad(fill(
                                                Bounds::centered_at(
                                                    *screen_pt,
                                                    size(point_radius * 2.0, point_radius * 2.0),
                                                ),
                                                color,
                                            ));

                                            if is_hovered && hovered_point.is_none() {
                                                hovered_point = Some(HoveredPoint {
                                                    series_index,
                                                    point_index,
                                                    screen_pos: *screen_pt,
                                                    data_point: data_pt.clone(),
                                                });
                                            }
                                        }
                                    }
                                }

                                if state.tooltip.show && hitbox.is_hovered(window) {
                                    if let Some(hp) = hovered_point {
                                        let series = &state.series[hp.series_index];
                                        let tooltip_text = state
                                            .tooltip
                                            .format_tooltip(&hp.data_point, &series.name);

                                        let text_style = window.text_style();
                                        let font = text_style.font();
                                        let font_size = px(12.0);
                                        let text_len = tooltip_text.len();

                                        let text_run = TextRun {
                                            len: text_len,
                                            font,
                                            color: text_color,
                                            background_color: None,
                                            underline: None,
                                            strikethrough: None,
                                        };

                                        let shaped_line = window.text_system().shape_line(
                                            tooltip_text.into(),
                                            font_size,
                                            &[text_run],
                                            None,
                                        );

                                        let text_width = shaped_line.width;
                                        let padding_h = px(12.0);
                                        let padding_v = px(8.0);
                                        let tooltip_width = text_width + padding_h * 2.0;
                                        let tooltip_height = font_size + padding_v * 2.0;

                                        let tooltip_x = (hp.screen_pos.x - tooltip_width / 2.0)
                                            .max(bounds.left())
                                            .min(bounds.right() - tooltip_width);
                                        let tooltip_y = hp.screen_pos.y - tooltip_height - px(10.0);

                                        let tooltip_bounds = Bounds::new(
                                            point(tooltip_x, tooltip_y),
                                            size(tooltip_width, tooltip_height),
                                        );

                                        window.paint_quad(quad(
                                            tooltip_bounds,
                                            px(6.0),
                                            tooltip_bg,
                                            px(1.0),
                                            tooltip_border,
                                            BorderStyle::default(),
                                        ));

                                        let text_origin =
                                            point(tooltip_x + padding_h, tooltip_y + padding_v);
                                        let _ =
                                            shaped_line.paint(text_origin, font_size, window, cx);
                                    }
                                }
                            },
                        )
                        .size_full(),
                    )
                    .when(show_y_axis, |this| {
                        this.children(y_labels.iter().enumerate().map(|(i, label)| {
                            let tick_count = y_labels.len() - 1;
                            let top_percent = (i as f32 / tick_count.max(1) as f32) * 100.0;
                            div()
                                .absolute()
                                .left(px(4.0))
                                .top(relative(top_percent / 100.0))
                                .mt(px(padding.top - 6.0))
                                .text_size(px(11.0))
                                .text_color(text_color)
                                .child(label.clone())
                        }))
                    }),
            )
            .when(legend.show && series_for_legend.len() > 1, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(16.0))
                        .px(px(padding.left))
                        .py(px(12.0))
                        .justify_center()
                        .children(series_for_legend.iter().enumerate().map(|(i, s)| {
                            let color = s.color.unwrap_or_else(|| default_color(i));
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(div().size(px(12.0)).rounded(px(2.0)).bg(color))
                                .child(div().text_sm().text_color(text_color).child(s.name.clone()))
                        })),
                )
            })
    }
}
