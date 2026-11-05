//! Shared construction and painting helpers for the chart family
//! (`Chart`, `LineChart`, `AreaChart`, `Sparkline`, `BarChart`).
//!
//! Every helper is a lifted copy of logic that used to be duplicated across
//! the individual chart files. Numeric parameters (grid opacity, legend
//! padding, axis gutters) intentionally stay per-chart instead of being
//! merged, so each chart reproduces its previous output exactly.

use gpui::*;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Tick label formatting
// ---------------------------------------------------------------------------

/// Format a tick label, preferring `custom` when provided.
///
/// Superset of the formatting that used to exist as three verbatim copies
/// (`chart.rs`, `line_chart.rs`, `area_chart.rs`); those call sites must keep
/// producing identical strings.
pub(crate) fn format_tick_value(value: f64, custom: Option<&Rc<dyn Fn(f64) -> String>>) -> String {
    if let Some(format) = custom {
        format(value)
    } else if value.abs() >= 1000.0 {
        format!("{:.0}k", value / 1000.0)
    } else if value.abs() >= 1.0 {
        format!("{:.0}", value)
    } else {
        format!("{:.2}", value)
    }
}

// ---------------------------------------------------------------------------
// Axis padding
// ---------------------------------------------------------------------------

/// Horizontal gutter `Chart` reserves for y-axis labels.
const CHART_Y_AXIS_GUTTER: f32 = 60.0;
/// Vertical gutter `Chart` reserves below the plot for x-axis labels.
const CHART_X_AXIS_GUTTER: f32 = 40.0;
/// Padding `Chart` uses on every side that has no visible axis.
const CHART_SIDE_PADDING: f32 = 20.0;

/// Axis padding used by `Chart`, as `(left, right, top, bottom)`.
///
/// Left is 60px and bottom 40px when the matching axis is visible; hidden
/// axes fall back to 20px. These values deliberately differ from the
/// line/area gutters below, which are narrower.
pub(crate) fn chart_axis_padding(show_x_axis: bool, show_y_axis: bool) -> (f32, f32, f32, f32) {
    (
        if show_y_axis {
            CHART_Y_AXIS_GUTTER
        } else {
            CHART_SIDE_PADDING
        },
        CHART_SIDE_PADDING,
        CHART_SIDE_PADDING,
        if show_x_axis {
            CHART_X_AXIS_GUTTER
        } else {
            CHART_SIDE_PADDING
        },
    )
}

/// Horizontal gutter `LineChart`/`AreaChart` reserve for y-axis labels.
const LINE_AREA_Y_AXIS_GUTTER: f32 = 50.0;
/// Vertical gutter `LineChart`/`AreaChart` reserve for x-axis labels.
const LINE_AREA_X_AXIS_GUTTER: f32 = 40.0;
/// Padding on a label-bearing side when that axis is hidden.
const LINE_AREA_HIDDEN_GUTTER: f32 = 10.0;
/// Padding on the sides that never carry axis labels.
const LINE_AREA_SIDE_PADDING: f32 = 20.0;

/// Axis padding used by `LineChart` and `AreaChart`, as
/// `(left, right, top, bottom)`.
pub(crate) fn line_area_axis_padding(show_x_axis: bool, show_y_axis: bool) -> (f32, f32, f32, f32) {
    (
        if show_y_axis {
            LINE_AREA_Y_AXIS_GUTTER
        } else {
            LINE_AREA_HIDDEN_GUTTER
        },
        LINE_AREA_SIDE_PADDING,
        LINE_AREA_SIDE_PADDING,
        if show_x_axis {
            LINE_AREA_X_AXIS_GUTTER
        } else {
            LINE_AREA_HIDDEN_GUTTER
        },
    )
}

// ---------------------------------------------------------------------------
// Grid painting
// ---------------------------------------------------------------------------

/// Paint `divisions` evenly spaced horizontal grid lines spanning the plot
/// area. `color` must already carry the caller's grid opacity (charts differ:
/// 0.2 for `Chart`/`AreaChart`, 0.3 for `LineChart`).
pub(crate) fn paint_horizontal_grid_lines(
    window: &mut Window,
    plot_left: Pixels,
    plot_right: Pixels,
    plot_top: Pixels,
    plot_height: Pixels,
    divisions: usize,
    color: Hsla,
) {
    for i in 0..=divisions {
        let y = plot_top + plot_height * (i as f32 / divisions as f32);
        let mut builder = PathBuilder::stroke(px(1.0));
        builder.move_to(point(plot_left, y));
        builder.line_to(point(plot_right, y));
        if let Ok(path) = builder.build() {
            window.paint_path(path, color);
        }
    }
}

/// Paint `divisions` evenly spaced vertical grid lines spanning the plot
/// area. `color` must already carry the caller's grid opacity.
pub(crate) fn paint_vertical_grid_lines(
    window: &mut Window,
    plot_top: Pixels,
    plot_bottom: Pixels,
    plot_left: Pixels,
    plot_width: Pixels,
    divisions: usize,
    color: Hsla,
) {
    for i in 0..=divisions {
        let x = plot_left + plot_width * (i as f32 / divisions as f32);
        let mut builder = PathBuilder::stroke(px(1.0));
        builder.move_to(point(x, plot_top));
        builder.line_to(point(x, plot_bottom));
        if let Ok(path) = builder.build() {
            window.paint_path(path, color);
        }
    }
}

// ---------------------------------------------------------------------------
// Series paths
// ---------------------------------------------------------------------------

/// Paint a stroked series line through `screen_points`. When `smooth` is set
/// (and at least three points exist), consecutive points are joined with
/// quadratic curves whose control point sits at each segment's horizontal
/// midpoint — the smoothing previously duplicated in `chart.rs` and
/// `line_chart.rs`.
pub(crate) fn paint_series_line(
    window: &mut Window,
    screen_points: &[Point<Pixels>],
    smooth: bool,
    stroke_width: Pixels,
    color: Hsla,
) {
    if screen_points.len() < 2 {
        return;
    }

    let mut builder = PathBuilder::stroke(stroke_width);
    builder.move_to(screen_points[0]);

    if smooth && screen_points.len() >= 3 {
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

/// Paint the area between `screen_points` and the horizontal `baseline_y`.
/// `fill` must already carry the caller's fill opacity (charts differ).
pub(crate) fn paint_area_under_line(
    window: &mut Window,
    screen_points: &[Point<Pixels>],
    baseline_y: Pixels,
    fill: Hsla,
) {
    if screen_points.len() < 2 {
        return;
    }

    let mut builder = PathBuilder::fill();
    builder.move_to(point(screen_points[0].x, baseline_y));
    builder.line_to(screen_points[0]);
    for pt in screen_points.iter().skip(1) {
        builder.line_to(*pt);
    }
    builder.line_to(point(screen_points.last().unwrap().x, baseline_y));
    builder.close();
    if let Ok(path) = builder.build() {
        window.paint_path(path, fill);
    }
}

/// Paint the filled band between a `top_points` polyline and a reversed
/// `bottom_points` polyline (stacked areas in `AreaChart`).
pub(crate) fn paint_area_band(
    window: &mut Window,
    top_points: &[Point<Pixels>],
    bottom_points: &[Point<Pixels>],
    fill: Hsla,
) {
    if top_points.is_empty() || bottom_points.is_empty() {
        return;
    }

    let mut builder = PathBuilder::fill();
    builder.move_to(bottom_points[0]);
    for pt in top_points {
        builder.line_to(*pt);
    }
    for pt in bottom_points.iter().rev() {
        builder.line_to(*pt);
    }
    builder.close();
    if let Ok(path) = builder.build() {
        window.paint_path(path, fill);
    }
}

// ---------------------------------------------------------------------------
// Legend row
// ---------------------------------------------------------------------------

/// Label text size for [`chart_legend_row`]; charts historically differ
/// between `text_sm` and `text_xs`.
#[derive(Copy, Clone)]
pub(crate) enum LegendText {
    Sm,
    Xs,
}

/// Build the shared legend row: a 12px rounded swatch plus a label per entry,
/// centered and wrapping. `pad_x`/`pad_y` reproduce each chart's historical
/// padding (`Chart`: 12px vertical, `LineChart`/`AreaChart`: 8px vertical and
/// horizontal label-gutter padding, bar charts: none).
pub(crate) fn chart_legend_row(
    entries: Vec<(SharedString, Hsla)>,
    text_color: Hsla,
    text: LegendText,
    pad_x: Option<f32>,
    pad_y: Option<f32>,
) -> Div {
    let mut row = div().flex().flex_wrap().gap(px(16.0)).justify_center();
    if let Some(x) = pad_x {
        row = row.px(px(x));
    }
    if let Some(y) = pad_y {
        row = row.py(px(y));
    }
    row.children(entries.into_iter().map(|(name, color)| {
        let label = match text {
            LegendText::Sm => div().text_sm(),
            LegendText::Xs => div().text_xs(),
        };
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(div().size(px(12.0)).rounded(px(2.0)).bg(color))
            .child(label.text_color(text_color).child(name))
    }))
}

// ---------------------------------------------------------------------------
// Bar chart helpers
// ---------------------------------------------------------------------------

/// Clamp non-finite values to `0.0` so bar geometry never sees NaN/Infinity.
pub(crate) fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

/// One signed contribution inside a stacked bar: the accumulated
/// `[start, end]` value window for `series_index`.
pub(crate) struct StackedSegment {
    pub series_index: usize,
    pub value: f64,
    pub start: f64,
    pub end: f64,
}

/// Accumulate per-series values (one label's worth) into signed stack
/// segments plus their total: positive values stack up from zero, negative
/// values stack down. Non-finite values count as zero. Previously duplicated
/// by the vertical and horizontal stacked bar layouts.
pub(crate) fn signed_stack_segments(values: &[f64]) -> (Vec<StackedSegment>, f64) {
    let mut positive = 0.0;
    let mut negative = 0.0;
    let mut segments = Vec::with_capacity(values.len());

    for (series_index, &raw) in values.iter().enumerate() {
        let value = finite_or_zero(raw);
        let (start, end) = if value >= 0.0 {
            let start = positive;
            positive += value;
            (start, positive)
        } else {
            let start = negative;
            negative += value;
            (start, negative)
        };
        segments.push(StackedSegment {
            series_index,
            value,
            start,
            end,
        });
    }

    let total: f64 = segments.iter().map(|segment| segment.value).sum();
    (segments, total)
}

/// Five evenly spaced horizontal separators behind vertical bars.
pub(crate) fn vertical_bar_grid(color: Hsla) -> Div {
    div()
        .absolute()
        .inset_0()
        .flex()
        .flex_col()
        .justify_between()
        .children((0..5).map(move |_| div().w_full().h(px(1.0)).bg(color)))
}

/// Five evenly spaced vertical separators behind horizontal bars.
pub(crate) fn horizontal_bar_grid(color: Hsla) -> Div {
    div()
        .absolute()
        .inset_0()
        .flex()
        .justify_between()
        .children((0..5).map(move |_| div().h_full().w(px(1.0)).bg(color)))
}

/// Horizontal zero-axis line for vertical bar layouts, offset from the plot
/// top by `top`.
pub(crate) fn horizontal_zero_axis(top: Pixels, color: Hsla) -> Div {
    div()
        .absolute()
        .left_0()
        .right_0()
        .top(top)
        .h(px(1.0))
        .bg(color)
}

/// Vertical zero-axis line for horizontal bar layouts, positioned by the
/// zero `left_ratio` within the plot.
pub(crate) fn vertical_zero_axis(left_ratio: f32, color: Hsla) -> Div {
    div()
        .absolute()
        .top_0()
        .bottom_0()
        .left(relative(left_ratio))
        .w(px(1.0))
        .bg(color)
}

/// Evenly distributed category labels below a vertical bar chart.
pub(crate) fn bar_category_labels(labels: &[SharedString], text_color: Hsla) -> Div {
    div()
        .flex()
        .justify_around()
        .px(px(16.0))
        .children(labels.iter().map(|label| {
            div()
                .text_xs()
                .text_color(text_color)
                .text_center()
                .child(label.clone())
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn tick_format_matches_the_historical_shared_output() {
        assert_eq!(format_tick_value(1000.0, None), "1k");
        assert_eq!(format_tick_value(-2000.0, None), "-2k");
        assert_eq!(format_tick_value(999.0, None), "999");
        assert_eq!(format_tick_value(1.0, None), "1");
        assert_eq!(format_tick_value(0.5, None), "0.50");
        assert_eq!(format_tick_value(-0.25, None), "-0.25");
    }

    #[::core::prelude::v1::test]
    fn tick_format_prefers_the_custom_hook() {
        let custom: Rc<dyn Fn(f64) -> String> = Rc::new(|value| format!("v={value}"));
        assert_eq!(
            format_tick_value(1000.0, Some(&custom)),
            "v=1000".to_string()
        );
    }

    #[::core::prelude::v1::test]
    fn finite_or_zero_clamps_only_non_finite_values() {
        assert_eq!(finite_or_zero(3.5), 3.5);
        assert_eq!(finite_or_zero(f64::NAN), 0.0);
        assert_eq!(finite_or_zero(f64::INFINITY), 0.0);
    }

    #[::core::prelude::v1::test]
    fn signed_stack_splits_positive_and_negative_accumulations() {
        let (segments, total) = signed_stack_segments(&[10.0, -3.0, 4.0]);

        assert_eq!(total, 11.0);
        assert_eq!(segments.len(), 3);
        assert_eq!((segments[0].start, segments[0].end), (0.0, 10.0));
        assert_eq!((segments[1].start, segments[1].end), (0.0, -3.0));
        assert_eq!((segments[2].start, segments[2].end), (10.0, 14.0));
    }

    #[::core::prelude::v1::test]
    fn signed_stack_treats_non_finite_values_as_zero() {
        let (segments, total) = signed_stack_segments(&[f64::NAN, 2.0]);

        assert_eq!(total, 2.0);
        assert_eq!((segments[0].start, segments[0].end), (0.0, 0.0));
        assert_eq!((segments[1].start, segments[1].end), (0.0, 2.0));
    }
}
