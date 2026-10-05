//! Parses SVG path data and renders via GPUI PathBuilder.

use gpui::{prelude::FluentBuilder as _, *};

use crate::capabilities::foundation::theme::use_theme;

#[derive(Clone, Debug, PartialEq)]
enum SvgCommand {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    HLineTo(f32),
    VLineTo(f32),
    CurveTo(f32, f32, f32, f32, f32, f32),
    QuadTo(f32, f32, f32, f32),
    Close,
}

/// Parse an SVG `d` attribute into drawable commands.
///
/// Both the absolute (`M L H V C Q Z`) and relative (`m l h v c q z`) forms of the
/// six supported commands are understood; relative commands are resolved against the
/// current point while parsing, and `Z` returns the current point to the start of the
/// subpath so relative commands after a close resolve correctly. Commands the
/// renderer cannot draw (`A`, `S`, `T` and their relative forms) have their arguments
/// consumed and are skipped, instead of leaving the parser to nibble through the
/// following command's arguments.
fn parse_svg_path(data: &str) -> Vec<SvgCommand> {
    let mut commands = Vec::new();
    let mut chars = data.chars().peekable();
    let mut current_cmd = ' ';
    // Current point, used to resolve relative commands into absolute ones.
    let mut cursor = (0.0_f32, 0.0_f32);
    // Start of the current subpath; `Z` moves the current point back here (per the
    // SVG spec) so relative commands after a close — e.g. the `z m5 5` continuation
    // emitted by SVG optimizers — resolve from the subpath origin.
    let mut subpath_start = (0.0_f32, 0.0_f32);
    let mut relative = false;

    fn skip_ws_and_commas(chars: &mut std::iter::Peekable<std::str::Chars>) {
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() || c == ',' {
                chars.next();
            } else {
                break;
            }
        }
    }

    fn parse_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<f32> {
        skip_ws_and_commas(chars);
        let mut s = String::new();
        if let Some(&c) = chars.peek() {
            if c == '-' || c == '+' {
                s.push(c);
                chars.next();
            }
        }
        let mut has_dot = false;
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                chars.next();
            } else if c == '.' && !has_dot {
                has_dot = true;
                s.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if s.is_empty() || s == "-" || s == "+" {
            None
        } else {
            s.parse().ok()
        }
    }

    /// Consume unsupported command arguments and return the final current point.
    fn skip_unsupported_args(
        chars: &mut std::iter::Peekable<std::str::Chars>,
        command: char,
        mut cursor: (f32, f32),
        relative: bool,
    ) -> Option<(f32, f32)> {
        let argument_count = match command {
            'A' => 7,
            'S' => 4,
            'T' => 2,
            _ => {
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                    chars.next();
                }
                return None;
            }
        };

        let mut endpoint = None;
        loop {
            let mut values = Vec::with_capacity(argument_count);
            for _ in 0..argument_count {
                let Some(value) = parse_number(chars) else {
                    return endpoint;
                };
                values.push(value);
            }

            let x = values[argument_count - 2];
            let y = values[argument_count - 1];
            cursor = if relative {
                (cursor.0 + x, cursor.1 + y)
            } else {
                (x, y)
            };
            endpoint = Some(cursor);
        }
    }

    while chars.peek().is_some() {
        skip_ws_and_commas(&mut chars);
        if let Some(&c) = chars.peek() {
            if c.is_ascii_alphabetic() {
                current_cmd = c.to_ascii_uppercase();
                relative = c.is_ascii_lowercase();
                chars.next();
            }
        }

        // Resolve a possibly-relative coordinate pair against the current point.
        macro_rules! point2 {
            ($x:expr, $y:expr) => {{
                let resolved = if relative {
                    (cursor.0 + $x, cursor.1 + $y)
                } else {
                    ($x, $y)
                };
                cursor = resolved;
                resolved
            }};
        }

        match current_cmd {
            'M' => {
                if let (Some(x), Some(y)) = (parse_number(&mut chars), parse_number(&mut chars)) {
                    let (x, y) = point2!(x, y);
                    subpath_start = (x, y);
                    commands.push(SvgCommand::MoveTo(x, y));
                    current_cmd = 'L';
                } else {
                    break;
                }
            }
            'L' => {
                if let (Some(x), Some(y)) = (parse_number(&mut chars), parse_number(&mut chars)) {
                    let (x, y) = point2!(x, y);
                    commands.push(SvgCommand::LineTo(x, y));
                } else {
                    break;
                }
            }
            'H' => {
                if let Some(x) = parse_number(&mut chars) {
                    let x = if relative { cursor.0 + x } else { x };
                    cursor.0 = x;
                    commands.push(SvgCommand::HLineTo(x));
                } else {
                    break;
                }
            }
            'V' => {
                if let Some(y) = parse_number(&mut chars) {
                    let y = if relative { cursor.1 + y } else { y };
                    cursor.1 = y;
                    commands.push(SvgCommand::VLineTo(y));
                } else {
                    break;
                }
            }
            'C' => {
                if let (Some(x1), Some(y1), Some(x2), Some(y2), Some(x), Some(y)) = (
                    parse_number(&mut chars),
                    parse_number(&mut chars),
                    parse_number(&mut chars),
                    parse_number(&mut chars),
                    parse_number(&mut chars),
                    parse_number(&mut chars),
                ) {
                    // Control points follow the same relative offset as the endpoint.
                    let (x1, y1) = if relative {
                        (cursor.0 + x1, cursor.1 + y1)
                    } else {
                        (x1, y1)
                    };
                    let (x2, y2) = if relative {
                        (cursor.0 + x2, cursor.1 + y2)
                    } else {
                        (x2, y2)
                    };
                    let (x, y) = point2!(x, y);
                    commands.push(SvgCommand::CurveTo(x1, y1, x2, y2, x, y));
                } else {
                    break;
                }
            }
            'Q' => {
                if let (Some(cx1), Some(cy1), Some(x), Some(y)) = (
                    parse_number(&mut chars),
                    parse_number(&mut chars),
                    parse_number(&mut chars),
                    parse_number(&mut chars),
                ) {
                    let (cx1, cy1) = if relative {
                        (cursor.0 + cx1, cursor.1 + cy1)
                    } else {
                        (cx1, cy1)
                    };
                    let (x, y) = point2!(x, y);
                    commands.push(SvgCommand::QuadTo(cx1, cy1, x, y));
                } else {
                    break;
                }
            }
            'Z' => {
                commands.push(SvgCommand::Close);
                current_cmd = 'M';
                relative = false;
                cursor = subpath_start;
            }
            other => {
                // A/S/T are not drawable, but their arguments still advance the
                // SVG current point. Unknown commands are simply discarded.
                if let Some(endpoint) = skip_unsupported_args(&mut chars, other, cursor, relative) {
                    cursor = endpoint;
                }
            }
        }
    }

    commands
}

#[derive(Clone)]
struct SvgPaintData {
    commands: Vec<SvgCommand>,
    view_box: Bounds<f32>,
    fill_color: Hsla,
    stroke_color: Option<Hsla>,
    stroke_width: f32,
}

#[derive(IntoElement)]
pub struct SVGRenderer {
    path_data: SharedString,
    view_box: Bounds<f32>,
    fill_color: Option<Hsla>,
    stroke_color: Option<Hsla>,
    stroke_width: f32,
    style: StyleRefinement,
}

impl SVGRenderer {
    pub fn new() -> Self {
        Self {
            path_data: SharedString::default(),
            view_box: Bounds::new(point(0.0_f32, 0.0_f32), size(100.0_f32, 100.0_f32)),
            fill_color: None,
            stroke_color: None,
            stroke_width: 1.0,
            style: StyleRefinement::default(),
        }
    }

    pub fn path_data(mut self, data: impl Into<SharedString>) -> Self {
        self.path_data = data.into();
        self
    }

    pub fn view_box(mut self, x: f32, y: f32, w: f32, h: f32) -> Self {
        self.view_box = Bounds::new(point(x, y), size(w, h));
        self
    }

    pub fn fill(mut self, color: Hsla) -> Self {
        self.fill_color = Some(color);
        self
    }

    pub fn stroke(mut self, color: Hsla) -> Self {
        self.stroke_color = Some(color);
        self
    }

    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }
}

impl Default for SVGRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for SVGRenderer {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

fn transform_point(
    vx: f32,
    vy: f32,
    view_box: &Bounds<f32>,
    bounds: &Bounds<Pixels>,
) -> Point<Pixels> {
    let scale_x = bounds.size.width / px(1.0) / view_box.size.width;
    let scale_y = bounds.size.height / px(1.0) / view_box.size.height;
    let scale = scale_x.min(scale_y);

    let offset_x = (bounds.size.width / px(1.0) - view_box.size.width * scale) * 0.5;
    let offset_y = (bounds.size.height / px(1.0) - view_box.size.height * scale) * 0.5;

    point(
        bounds.left() + px(offset_x + (vx - view_box.origin.x) * scale),
        bounds.top() + px(offset_y + (vy - view_box.origin.y) * scale),
    )
}

impl RenderOnce for SVGRenderer {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let user_style = self.style;

        let fill_color = self.fill_color.unwrap_or(theme.tokens.foreground);
        let commands = parse_svg_path(&self.path_data);

        let paint_data = SvgPaintData {
            commands,
            view_box: self.view_box,
            fill_color,
            stroke_color: self.stroke_color,
            stroke_width: self.stroke_width,
        };

        div()
            .size_full()
            .child(
                canvas(
                    move |_, _, _| paint_data,
                    move |bounds, data, window, _cx| {
                        if data.commands.is_empty() {
                            return;
                        }

                        let mut current_x = 0.0_f32;
                        let mut current_y = 0.0_f32;

                        if let Some(stroke_color) = data.stroke_color {
                            let mut builder = PathBuilder::stroke(px(data.stroke_width));
                            let mut started = false;
                            let mut subpath_start = (0.0_f32, 0.0_f32);

                            for cmd in &data.commands {
                                match cmd {
                                    SvgCommand::MoveTo(x, y) => {
                                        let pt = transform_point(*x, *y, &data.view_box, &bounds);
                                        if started {
                                            if let Ok(path) = builder.build() {
                                                window.paint_path(path, stroke_color);
                                            }
                                            builder = PathBuilder::stroke(px(data.stroke_width));
                                        }
                                        builder.move_to(pt);
                                        current_x = *x;
                                        current_y = *y;
                                        subpath_start = (*x, *y);
                                        started = true;
                                    }
                                    SvgCommand::LineTo(x, y) => {
                                        let pt = transform_point(*x, *y, &data.view_box, &bounds);
                                        builder.line_to(pt);
                                        current_x = *x;
                                        current_y = *y;
                                    }
                                    SvgCommand::HLineTo(x) => {
                                        let pt =
                                            transform_point(*x, current_y, &data.view_box, &bounds);
                                        builder.line_to(pt);
                                        current_x = *x;
                                    }
                                    SvgCommand::VLineTo(y) => {
                                        let pt =
                                            transform_point(current_x, *y, &data.view_box, &bounds);
                                        builder.line_to(pt);
                                        current_y = *y;
                                    }
                                    SvgCommand::CurveTo(x1, y1, x2, y2, x, y) => {
                                        let cp1 =
                                            transform_point(*x1, *y1, &data.view_box, &bounds);
                                        let cp2 =
                                            transform_point(*x2, *y2, &data.view_box, &bounds);
                                        let end = transform_point(*x, *y, &data.view_box, &bounds);
                                        builder.cubic_bezier_to(end, cp1, cp2);
                                        current_x = *x;
                                        current_y = *y;
                                    }
                                    SvgCommand::QuadTo(cx1, cy1, x, y) => {
                                        let cp =
                                            transform_point(*cx1, *cy1, &data.view_box, &bounds);
                                        let end = transform_point(*x, *y, &data.view_box, &bounds);
                                        builder.curve_to(end, cp);
                                        current_x = *x;
                                        current_y = *y;
                                    }
                                    SvgCommand::Close => {
                                        builder.close();
                                        current_x = subpath_start.0;
                                        current_y = subpath_start.1;
                                    }
                                }
                            }

                            if started {
                                if let Ok(path) = builder.build() {
                                    window.paint_path(path, stroke_color);
                                }
                            }
                        }

                        {
                            let mut builder = PathBuilder::fill();
                            let mut started = false;
                            let mut subpath_start = (0.0_f32, 0.0_f32);
                            current_x = 0.0;
                            current_y = 0.0;

                            for cmd in &data.commands {
                                match cmd {
                                    SvgCommand::MoveTo(x, y) => {
                                        if started {
                                            builder.close();
                                            if let Ok(path) = builder.build() {
                                                window.paint_path(path, data.fill_color);
                                            }
                                            builder = PathBuilder::fill();
                                        }
                                        let pt = transform_point(*x, *y, &data.view_box, &bounds);
                                        builder.move_to(pt);
                                        current_x = *x;
                                        current_y = *y;
                                        subpath_start = (*x, *y);
                                        started = true;
                                    }
                                    SvgCommand::LineTo(x, y) => {
                                        let pt = transform_point(*x, *y, &data.view_box, &bounds);
                                        builder.line_to(pt);
                                        current_x = *x;
                                        current_y = *y;
                                    }
                                    SvgCommand::HLineTo(x) => {
                                        let pt =
                                            transform_point(*x, current_y, &data.view_box, &bounds);
                                        builder.line_to(pt);
                                        current_x = *x;
                                    }
                                    SvgCommand::VLineTo(y) => {
                                        let pt =
                                            transform_point(current_x, *y, &data.view_box, &bounds);
                                        builder.line_to(pt);
                                        current_y = *y;
                                    }
                                    SvgCommand::CurveTo(x1, y1, x2, y2, x, y) => {
                                        let cp1 =
                                            transform_point(*x1, *y1, &data.view_box, &bounds);
                                        let cp2 =
                                            transform_point(*x2, *y2, &data.view_box, &bounds);
                                        let end = transform_point(*x, *y, &data.view_box, &bounds);
                                        builder.cubic_bezier_to(end, cp1, cp2);
                                        current_x = *x;
                                        current_y = *y;
                                    }
                                    SvgCommand::QuadTo(cx1, cy1, x, y) => {
                                        let cp =
                                            transform_point(*cx1, *cy1, &data.view_box, &bounds);
                                        let end = transform_point(*x, *y, &data.view_box, &bounds);
                                        builder.curve_to(end, cp);
                                        current_x = *x;
                                        current_y = *y;
                                    }
                                    SvgCommand::Close => {
                                        builder.close();
                                        current_x = subpath_start.0;
                                        current_y = subpath_start.1;
                                    }
                                }
                            }

                            if started {
                                builder.close();
                                if let Ok(path) = builder.build() {
                                    window.paint_path(path, data.fill_color);
                                }
                            }
                        }
                    },
                )
                .size_full(),
            )
            .map(|this| {
                let mut el = this;
                el.style().refine(&user_style);
                el
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_svg_path, SvgCommand};

    #[test]
    fn absolute_commands_parse_verbatim() {
        let commands = parse_svg_path("M10 20 L30 40 H5 V6 Z");
        assert_eq!(
            commands,
            vec![
                SvgCommand::MoveTo(10.0, 20.0),
                SvgCommand::LineTo(30.0, 40.0),
                SvgCommand::HLineTo(5.0),
                SvgCommand::VLineTo(6.0),
                SvgCommand::Close,
            ]
        );
    }

    #[test]
    fn relative_commands_resolve_against_the_current_point() {
        let commands = parse_svg_path("m10 10 l5 0 h5 v5 z");
        assert_eq!(
            commands,
            vec![
                SvgCommand::MoveTo(10.0, 10.0),
                SvgCommand::LineTo(15.0, 10.0),
                SvgCommand::HLineTo(20.0),
                SvgCommand::VLineTo(15.0),
                SvgCommand::Close,
            ]
        );
    }

    #[test]
    fn relative_curves_offset_control_points_and_endpoint() {
        let commands = parse_svg_path("M10 10 c1 2 3 4 5 6");
        assert_eq!(
            commands,
            vec![
                SvgCommand::MoveTo(10.0, 10.0),
                SvgCommand::CurveTo(11.0, 12.0, 13.0, 14.0, 15.0, 16.0),
            ]
        );
    }

    #[test]
    fn repeated_m_exposes_the_implicit_lineto() {
        let commands = parse_svg_path("M0 0 10 10 20 20");
        assert_eq!(
            commands,
            vec![
                SvgCommand::MoveTo(0.0, 0.0),
                SvgCommand::LineTo(10.0, 10.0),
                SvgCommand::LineTo(20.0, 20.0),
            ]
        );
    }

    #[test]
    fn unsupported_arc_is_skipped_without_eating_the_next_command() {
        // `A` used to be consumed one character at a time, silently dropping the
        // following `L` too. Its arguments must be skipped and the `L` still parsed.
        let commands = parse_svg_path("M0 0 A5 5 0 0 1 10 10 L20 20");
        assert_eq!(
            commands,
            vec![SvgCommand::MoveTo(0.0, 0.0), SvgCommand::LineTo(20.0, 20.0),]
        );
    }

    #[test]
    fn unsupported_command_at_the_end_does_not_panic() {
        assert_eq!(
            parse_svg_path("M1 1 A2 2 0 0 1 3 3"),
            vec![SvgCommand::MoveTo(1.0, 1.0)]
        );
    }

    #[test]
    fn unsupported_arc_updates_the_current_point_for_relative_commands() {
        let commands = parse_svg_path("M0 0 A5 5 0 0 1 10 10 l5 0");
        assert_eq!(
            commands,
            vec![SvgCommand::MoveTo(0.0, 0.0), SvgCommand::LineTo(15.0, 10.0),]
        );
    }

    #[test]
    fn close_resets_horizontal_and_vertical_relative_coordinates() {
        let commands = parse_svg_path("M10 10 L20 20 Z h5 v5");
        assert_eq!(
            commands,
            vec![
                SvgCommand::MoveTo(10.0, 10.0),
                SvgCommand::LineTo(20.0, 20.0),
                SvgCommand::Close,
                SvgCommand::HLineTo(15.0),
                SvgCommand::VLineTo(15.0),
            ]
        );
    }

    #[test]
    fn close_returns_the_cursor_to_the_subpath_start() {
        // SVG optimizers commonly continue with a relative move after a close
        // (`z m5 5`); per the spec the current point after `Z` is the subpath
        // origin, not the last drawn point.
        let commands = parse_svg_path("M10 10 L20 20 Z m5 5");
        assert_eq!(
            commands,
            vec![
                SvgCommand::MoveTo(10.0, 10.0),
                SvgCommand::LineTo(20.0, 20.0),
                SvgCommand::Close,
                SvgCommand::MoveTo(15.0, 15.0),
            ]
        );
    }

    #[test]
    fn implicit_lineto_after_relative_moveto_stays_relative() {
        let commands = parse_svg_path("m10 10 5 5");
        assert_eq!(
            commands,
            vec![
                SvgCommand::MoveTo(10.0, 10.0),
                SvgCommand::LineTo(15.0, 15.0),
            ]
        );
    }
}
