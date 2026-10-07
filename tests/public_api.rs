//! External-consumer compatibility contract for the legacy public API.

#[allow(unused_imports)]
use fc_ui::{
    animate, animated_state, animation_coordinator, animations, charts, components,
    content_transition, display, fonts, gestures, gpui_ext, http, icon_config, layout, navigation,
    overlays, prelude, responsive, scroll_physics, spring, styled_ext, theme, transitions, util,
    virtual_list,
};

#[test]
fn legacy_root_modules_and_functions_remain_public() {
    let _ = std::any::type_name_of_val(&fc_ui::init);
    let _ = std::any::type_name_of_val(&fc_ui::init_http);
    let _ = std::any::type_name_of_val(&fc_ui::init_http_with_user_agent);
    let set_icon_base_path: fn(String) = fc_ui::set_icon_base_path;
    let _ = set_icon_base_path;
    let _ = std::any::type_name::<prelude::Button>();
}

#[cfg(feature = "http")]
#[test]
fn explicit_http_initialization_api_is_public() {
    #[allow(unused_imports)]
    use fc_ui::prelude::{
        try_init_http, try_init_http_with_user_agent, HttpInitError, HttpSetup, InitError,
        DEFAULT_USER_AGENT,
    };
    use std::sync::Arc;

    let try_init: fn(&mut gpui::App, HttpSetup) -> Result<(), InitError> = fc_ui::try_init_with;
    let new_client: fn(&str) -> Result<Arc<fc_ui::http::SimpleHttpClient>, HttpInitError> =
        fc_ui::http::SimpleHttpClient::new;
    let _ = try_init;
    let _ = new_client;
    let _ = std::any::type_name_of_val(&try_init_http);
    let _ = std::any::type_name_of_val(&try_init_http_with_user_agent);
    let _ = std::any::type_name::<HttpInitError>();
    assert_eq!(
        DEFAULT_USER_AGENT,
        concat!("fc-ui/", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn representative_builders_traits_and_generic_apis_remain_public() {
    fn assert_styled<T: gpui::Styled>() {}

    assert_styled::<prelude::LineChart>();
    assert_styled::<prelude::AreaChart>();
    assert_styled::<prelude::PieChart>();
    assert_styled::<prelude::DonutChart>();

    let line: prelude::LineChart = prelude::LineChart::new(vec![prelude::LineChartSeries::new(
        "revenue",
        vec![prelude::LineChartPoint::new(0.0, 1.0)],
    )])
    .show_grid(false)
    .show_x_axis(false)
    .show_y_axis(false)
    .y_range(0.0, 2.0)
    .x_labels(vec!["January"])
    .show_legend(false);
    let _ = line;

    let area: prelude::AreaChart = prelude::AreaChart::new()
        .series(prelude::AreaChartSeries::new("revenue", vec![(0.0, 1.0)]))
        .mode(prelude::AreaChartMode::Stacked)
        .size(prelude::AreaChartSize::Sm)
        .show_grid(false)
        .show_x_axis(false)
        .show_y_axis(false)
        .show_legend(false)
        .x_labels(vec!["January"])
        .y_label_count(3)
        .fill_opacity(0.5);
    let _ = area;

    let segment = prelude::PieChartSegment::new("revenue", 1.0);
    let pie: prelude::PieChart = prelude::PieChart::donut(vec![segment.clone()])
        .size(prelude::PieChartSize::Sm)
        .show_percentages(true)
        .center_label("Total")
        .donut_thickness(0.4)
        .label_position(prelude::PieChartLabelPosition::Legend);
    let _ = pie;

    let donut: prelude::DonutChart = prelude::DonutChart::new()
        .segment(segment)
        .inner_radius(0.5)
        .center_label("Total")
        .center_value("1")
        .size(prelude::DonutChartSize::Sm)
        .show_legend(true)
        .show_percentages(true);
    let _ = donut;

    struct FixedExtent;

    impl virtual_list::ItemExtentProvider for FixedExtent {
        fn extent(&self, _index: usize) -> gpui::Pixels {
            gpui::px(20.0)
        }
    }

    let variable_list = virtual_list::vlist_variable("api-contract", 2, FixedExtent, |_, _, _| {
        Vec::<gpui::Div>::new()
    })
    .overscan(2);
    let _ = variable_list;
}

#[test]
fn data_grid_facade_and_prelude_expose_custom_editor_fallback() {
    let facade_editor = display::data_grid::CellEditor::Custom;
    let prelude_editor = prelude::CellEditor::Custom;

    assert_eq!(facade_editor, prelude_editor);
    let _ = std::any::type_name::<display::data_grid::DataGrid<()>>();
    let _ = std::any::type_name::<prelude::DataGrid<()>>();
    let _ = std::any::type_name::<prelude::DataGridState<()>>();
}

#[test]
fn every_legacy_component_module_remains_public() {
    #[allow(unused_imports)]
    use components::{
        alert, animated_collapsible, animated_counter, animated_list, animated_presence,
        animated_progress, animated_switch, animated_text, audio_player, aurora, avatar,
        avatar_group, badge, button, calendar, canvas_component, carousel, checkbox, code_block,
        collapsible, color_picker, combobox, confetti, confirm_dialog, copy_button, countdown,
        crop_area, date_picker, dock, dot_pattern, drag_drop, drawer_navigation, dropdown,
        empty_state, expandable_card, file_upload, floating_action_button, form, glass_morphism,
        gradient_border, gradient_text, hotkey_input, icon, icon_button, icon_source, image_viewer,
        infinite_scroll, inline_edit, input, input_state, kbd, keyboard_shortcuts, label,
        layout_transition, magnetic_button, marquee, mention_input, meteors, navigation_menu,
        noise, notification_center, number_input, number_ticker, otp_input, pagination,
        particle_emitter, progress, pulse_indicator, radio, range_slider, rating, resizable,
        ripple, scrollable, scrollbar, search_input, segmented_nav, select, separator,
        shared_element_transition, shimmer, skeleton, skeleton_loader, slider, sortable_list,
        sparkline, spinner, split_pane, spotlight, stepper, svg_renderer, tag_input, text,
        text_highlight, text_reveal, textarea, textarea_state, tilt_card, time_picker, timeline,
        toggle, toggle_group, tooltip, type_writer, video_player, view_router, waveform,
    };
}

#[cfg(feature = "qrcode")]
#[test]
fn qrcode_legacy_module_remains_public() {
    let _ = std::any::type_name::<components::qr_code::QRCodeComponent>();
}

#[cfg(feature = "editor")]
#[test]
fn editor_legacy_module_remains_public() {
    let _ = std::any::type_name::<components::editor::Editor>();
}

#[test]
fn category_facades_and_direct_aliases_remain_public() {
    let _ = std::any::type_name::<charts::BarChart>();
    let _ = std::any::type_name::<overlays::CommandPalette>();
    let _ = std::any::type_name::<components::IconSize>();
    let _ = std::any::type_name::<components::IconVariant>();
    let _ = std::any::type_name::<components::IconSource>();
    let _ = std::any::type_name::<components::SliderAxis>();
    let _ = std::any::type_name::<components::badge::Badge>();
}

#[test]
fn gpui_09_reexports_remain_public_without_replacing_the_crate_spring() {
    use fc_ui::prelude::{
        container_query, div, sampled_easing, AnimationPhase, ContainerQuery, Interpolate,
        ParentElement, QuitMode, Spring, SpringAnimation, SpringConfig, SpringPlayback,
        SpringState, SpringTarget,
    };

    fn assert_interpolate<T: Interpolate>() {}
    fn assert_spring_target<T: SpringTarget>() {}

    assert_interpolate::<gpui::Pixels>();
    assert_spring_target::<gpui::Pixels>();

    let config = SpringConfig::new(100.0, 20.0, 1.0);
    let settled = config.step(SpringState::default(), 1.0, 1.0);
    let animation: SpringAnimation<f32> = SpringAnimation::new(config)
        .to(1.0)
        .playback(SpringPlayback::Running);
    let _ = (settled, animation, sampled_easing(config, 0.001).0);
    let _ = AnimationPhase(0.5).interpolate(0.0, 1.0);
    let _ = QuitMode::Explicit;

    let query: ContainerQuery = container_query(|_size, _window, _cx| div().child("sized"));
    let _ = query;

    // The prelude's `Spring` stays the crate-local one rather than a GPUI type.
    assert!(std::any::type_name::<Spring>().starts_with("fc_ui::"));
}

#[allow(unused_imports)]
mod prelude_export_contract {
    use fc_ui::prelude::{
        alert, animate_bounce_in, animate_fade_in, animate_fade_out, animate_scale_in,
        animate_slide_down, animate_slide_in_left, animate_slide_in_right, animate_slide_up, body,
        body_large, body_small, bounce, caption, code, code_small, current_breakpoint, ease_in_out,
        ease_out_quint, edit_menu, file_menu, h1, h2, h3, h4, h5, h6, help_menu, icon, icon_button,
        init_http, init_http_with_user_agent, init_image_viewer, init_mention_input,
        init_video_player, install_theme, label, label_small, lerp_color, lerp_f32, lerp_pixels,
        lerp_shadow, lerp_shadows, linear, muted, muted_small, pulsating_between, quadratic,
        responsive_columns, responsive_value, scrollable_both, scrollable_horizontal,
        scrollable_vertical, timeline, tooltip, use_theme, view_menu, window_menu,
        wire_input_actions, wire_textarea_actions, Accordion, AccordionItem, Alert, AlertDialog,
        AlertVariant, Align, AnimatedCollapsible, AnimatedCounter, AnimatedCounterState,
        AnimatedInteraction, AnimatedList, AnimatedListState, AnimatedPresence,
        AnimatedPresenceState, AnimatedProgress, AnimatedSwitch, AnimatedSwitchTransition,
        AnimatedText, AnimationCoordinator, AnimationPreset, AnimationRepeat, AppMenu, AppMenuBar,
        AreaChart, AreaChartMode, AreaChartSeries, AreaChartSize, AudioPlayer, AudioPlayerSize,
        AudioPlayerState, Aurora, Avatar, AvatarGroup, AvatarItem, AvatarSize, Axis, AxisPosition,
        Badge, BadgeVariant, BarChart, BarChartData, BarChartMode, BarChartOrientation,
        BarChartSeries, BottomSheet, BottomSheetSize, BreadcrumbItem, Breadcrumbs, Breakpoint,
        Button, ButtonSize, ButtonVariant, Calendar, CalendarLocale, CanvasComponent, Card,
        Carousel, CarouselSize, CarouselSlide, CarouselState, CarouselTransition, CellEditor,
        CellPosition, Chart, ChartArea, ChartPadding, Checkbox, CheckboxSize, CircularProgress,
        Cluster, CodeBlock, Collapsible, CollapsiblePane, ColorMode, ColorPicker, ColorPickerState,
        ColumnDef, Combobox, ComboboxEvent, ComboboxState, Command, CommandPalette,
        CommandPaletteState, Confetti, ConfettiState, Container, ContentTransition,
        ContentTransitionState, ContextMenu, CopyButton, CopyButtonState, Countdown,
        CountdownFormat, CountdownSeparator, CountdownSize, CountdownState, CropArea,
        CropAreaState, DataGrid, DataGridState, DataPoint, DataRange, DataTable, DateFormat,
        DatePicker, DatePickerState, DateValue, Dialog, DialogSize, Dock, DockState, DonutChart,
        DonutChartSize, DotPattern, DragData, DragHandle, Draggable, DrawerNavigation, DrawerSide,
        DrawerState, DropZone, DropZoneStyle, Dropdown, DropdownAlign, DropdownItem, DropdownState,
        EmptyState, EmptyStateSize, ExpandableCard, ExpandableCardState, FABSize, FABState,
        FileNode, FileNodeKind, FileTree, FileTypeFilter, FileUpload, FileUploadError,
        FileUploadSize, FileUploadState, FloatingActionButton, Flow, FlowDirection, Form,
        FormState, Gauge, GaugeSize, GestureDetector, GestureEvent, GlassIntensity, GlassMorphism,
        GradientBorder, GradientText, Grid, GridColumnDef, GridSortDirection, HStack, Heatmap,
        HotkeyInput, HotkeyInputSize, HotkeyInputState, HotkeyValue, HoverCard, HoverCardAlignment,
        HoverCardPosition, Html, Icon, IconButton, IconPosition, IconSize, IconSource, IconVariant,
        ImageItem, ImageViewer, ImageViewerSize, ImageViewerState, InfiniteScroll,
        InfiniteScrollState, InlineEdit, InlineEditState, InlineEditTrigger, Justify, KBDSize,
        KeyboardShortcuts, KeyframeAnimation, Label, LabelSide, LayoutAnimation, LayoutTransition,
        Legend, LegendPosition, LineChart, LineChartPoint, LineChartSeries, LoadingState,
        LongPressGesture, MagneticButton, MagneticButtonState, Markdown, Marquee, MarqueeDirection,
        MasonryGrid, MasonryItem, Mention, MentionInput, MentionInputEvent, MentionInputState,
        MentionItem, Menu, MenuBar, MenuBarItem, MenuItem, MenuItemKind, MeteorState, Meteors,
        NavigationMenu, NavigationMenuItem, Noise, NotificationBell, NotificationCenter,
        NotificationCenterState, NotificationItem, NotificationVariant, NumberInput,
        NumberInputSize, NumberInputState, NumberTicker, OTPInput, OTPInputEvent, OTPInputSize,
        OTPInputState, OTPState, PageTransition, Pagination, PanGesture, Panel, ParticleEmitter,
        ParticleEmitterConfig, ParticleEmitterState, PhysicsScrollState, PieChart,
        PieChartLabelPosition, PieChartSegment, PieChartSize, PieChartVariant, PlaybackSpeed,
        Popover, PopoverMenu, PopoverMenuItem, ProgressBar, ProgressSize, ProgressVariant,
        PulseIndicator, RadarChart, RadarChartSize, RadarDataset, Radio, RadioGroup, RadioLayout,
        RangeSlider, RangeSliderState, Rating, RatingSize, RatingState, ResizablePanel,
        ResizablePanelGroup, ResizableState, Responsive, RevealMode, RichBlock, RichInline,
        RichTableAlignment, Ripple, SVGRenderer, ScrollContainer, ScrollDirection, ScrollList,
        ScrollPhysics, SearchFilter, SearchInput, SearchInputState, SegmentedNav, SegmentedNavSize,
        SegmentedNavState, Select, SelectOption, SelectedFile, Separator, SeparatorOrientation,
        Series, SeriesType, SharedElementState, SharedElementTransition, Sheet, SheetSide,
        SheetSize, Shimmer, ShortcutCategory, ShortcutItem, Skeleton, SkeletonLoader,
        SkeletonLoaderState, SkeletonVariant, Slider, SliderAxis, SliderSize, SliderState,
        SortDirection, SortableList, SortableListState, Spacer, Sparkline, SparklineSize,
        SparklineTrend, SparklineVariant, Spinner, SpinnerSize, SpinnerVariant, SplitDirection,
        SplitPane, SplitPaneEvent, SplitPaneState, Spotlight, SpotlightState, Spring,
        StaggerConfig, StandardMacMenuBar, StatusBar, StatusItem, StepItem, StepStatus, Stepper,
        StepperOrientation, StepperSize, StepperState, StyledExt, SwipeDirection, SwipeGesture,
        TabItem, Table, TableColumn, TableRow, Tabs, TagInput, TagInputState, TapGesture, Text,
        TextAnimation, TextHighlight, TextReveal, TextVariant, Textarea, TextareaEvent,
        TextareaState, Theme, ThemeTokens, ThemeVariant, TiltCard, TiltCardState, TimeFormat,
        TimePeriod, TimePicker, TimePickerState, TimeUnits, TimeValue, Timeline,
        TimelineConnectorStyle, TimelineIndicatorStyle, TimelineItem, TimelineItemPosition,
        TimelineItemVariant, TimelineLayout, TimelineOrientation, TimelineSize, ToastItem,
        ToastManager, ToastPosition, ToastVariant, Toggle, ToggleGroup, ToggleGroupItem,
        ToggleGroupSize, ToggleGroupVariant, ToggleSize, Toolbar, ToolbarButton,
        ToolbarButtonVariant, ToolbarGroup, ToolbarItem, ToolbarSize, TooltipConfig, Transition,
        TreeList, TreeMap, TreeMapNode, TreeNode, TypeWriter, TypeWriterState, VStack,
        VideoPlaybackSpeed, VideoPlaybackState, VideoPlayer, VideoPlayerSize, VideoPlayerState,
        ViewRouter, ViewRouterState, Waveform, KBD,
    };

    #[cfg(feature = "qrcode")]
    use fc_ui::prelude::QRCodeComponent;
    #[cfg(feature = "editor")]
    use fc_ui::prelude::{Editor, EditorLanguage, EditorState};
}
