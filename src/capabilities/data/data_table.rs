//! DataTable - High-performance table component with virtual scrolling and sorting.

use crate::capabilities::controls::input::{Input, InputSize, InputState};
use crate::capabilities::controls::select::{Select, SelectEvent, SelectOption};
use crate::capabilities::data::table_common::{
    column_resize_handle, compare_cell_values, sortable_header_content, HeaderSortState,
    RowComparator,
};
use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::primitives::icon_source::IconSource;
use crate::capabilities::scroll::uniform_virtual_list::vlist_uniform_view;
use gpui::{prelude::FluentBuilder as _, *};
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

pub type RowActionCallback = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type LoadMoreCallback<T> = Box<dyn Fn(&mut Window, &mut Context<T>) + 'static>;
type CellEditCallback<T> =
    Box<dyn Fn(usize, SharedString, SharedString, SharedString, &mut Context<T>) + 'static>;
type CellDoubleClickCallback<T> =
    Box<dyn Fn(&T, SharedString, SharedString, &mut Window, &mut Context<DataTable<T>>) + 'static>;
type FetchPageCallback<T> = Box<dyn Fn(usize, usize, &mut Window, &mut Context<T>) + 'static>;
type FetchPageRequestCallback<T> =
    Box<dyn Fn(VirtualPageRequest, &mut Window, &mut Context<T>) + 'static>;
type SearchChangeCallback<T> = Box<dyn Fn(String, Option<SharedString>, &mut Context<T>) + 'static>;
type RowClickCallback<T> =
    Box<dyn Fn(usize, &T, &mut Window, &mut Context<DataTable<T>>) + 'static>;
type SelectionChangeCallback<T> = Box<dyn Fn(&[usize], &mut Window, &mut Context<T>) + 'static>;
type SortChangeCallback<T> =
    Box<dyn Fn(SharedString, SortDirection, &mut Window, &mut Context<T>) + 'static>;

#[derive(Clone)]
pub struct RowAction {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Option<IconSource>,
    pub destructive: bool,
    pub on_click: RowActionCallback,
}

impl RowAction {
    pub fn new<S: Into<SharedString>, F: Fn(usize, &mut Window, &mut App) + 'static>(
        id: S,
        label: S,
        handler: F,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            destructive: false,
            on_click: Rc::new(handler),
        }
    }

    pub fn icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Opaque identity of one virtual page load, including its query generation.
///
/// Keep this token with the asynchronous load and return it through
/// [`DataTable::complete_page_request`] or [`DataTable::fail_page_request`].
/// Resetting the table invalidates every outstanding token. A retry gets a new
/// identity, so a late completion from the failed attempt cannot complete it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtualPageRequest {
    generation: u64,
    request_id: u64,
    start: usize,
    size: usize,
}

impl VirtualPageRequest {
    pub fn start(&self) -> usize {
        self.start
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

struct VirtualPageState<T> {
    total_items: usize,
    page_size: usize,
    generation: u64,
    next_request_id: u64,
    cache: HashMap<usize, T>,
    pending: HashMap<usize, VirtualPageRequest>,
    completed: HashSet<usize>,
    legacy_pending: HashSet<usize>,
}

static NEXT_VIRTUAL_GENERATION: AtomicU64 = AtomicU64::new(0);

impl<T> VirtualPageState<T> {
    fn new(total_items: usize, page_size: usize) -> Self {
        // Unique across table instances as well as resets, so returning a
        // token to the wrong table cannot complete an unrelated request.
        let generation = NEXT_VIRTUAL_GENERATION
            .fetch_update(
                AtomicOrdering::Relaxed,
                AtomicOrdering::Relaxed,
                |generation| generation.checked_add(1),
            )
            .expect("virtual generation exhausted");
        Self {
            total_items,
            page_size: page_size.max(1),
            generation,
            next_request_id: 0,
            cache: HashMap::new(),
            pending: HashMap::new(),
            completed: HashSet::new(),
            legacy_pending: HashSet::new(),
        }
    }

    fn requests_for_range(
        &mut self,
        range: Range<usize>,
        tokenized: bool,
    ) -> Vec<VirtualPageRequest> {
        let end = range.end.min(self.total_items);
        if range.start >= end {
            return Vec::new();
        }
        let first_page = range.start / self.page_size * self.page_size;
        let mut requests = Vec::new();
        for start in (first_page..end).step_by(self.page_size) {
            let page_end = start.saturating_add(self.page_size).min(self.total_items);
            if self.pending.contains_key(&start)
                || self.legacy_pending.contains(&start)
                || (tokenized && self.completed.contains(&start))
                || (start..page_end).all(|index| self.cache.contains_key(&index))
            {
                continue;
            }
            self.next_request_id = self
                .next_request_id
                .checked_add(1)
                .expect("page request id exhausted");
            let request = VirtualPageRequest {
                generation: self.generation,
                request_id: self.next_request_id,
                start,
                size: self.page_size,
            };
            if tokenized {
                self.pending.insert(start, request);
            } else {
                self.legacy_pending.insert(start);
            }
            requests.push(request);
        }
        requests
    }

    fn insert_rows(&mut self, start: usize, rows: impl IntoIterator<Item = T>) {
        for (index, row) in (start..self.total_items).zip(rows) {
            self.cache.insert(index, row);
        }
    }

    fn complete(&mut self, request: VirtualPageRequest, rows: Vec<T>) -> bool {
        if self.pending.get(&request.start) != Some(&request) {
            return false;
        }
        self.pending.remove(&request.start);
        self.insert_rows(request.start, rows.into_iter().take(request.size));
        // An empty or short response still completes this request. The caller
        // can reset the table if the source's total count has changed.
        self.completed.insert(request.start);
        true
    }

    fn fail(&mut self, request: VirtualPageRequest) -> bool {
        if self.pending.get(&request.start) != Some(&request) {
            return false;
        }
        self.pending.remove(&request.start);
        true
    }

    fn set_legacy_page(&mut self, start: usize, rows: Vec<T>) {
        self.insert_rows(start, rows);
        self.legacy_pending.remove(&start);
    }
}

#[derive(Debug, Clone)]
struct ViewportState {
    viewport_height: f32,
    row_height: f32,
}

impl ViewportState {
    fn new(row_height: f32, viewport_height: f32) -> Self {
        Self {
            viewport_height,
            row_height,
        }
    }
}

struct VirtualScroller {
    viewport: ViewportState,
    total_items: usize,
}

impl VirtualScroller {
    fn new(total_items: usize, viewport: ViewportState) -> Self {
        Self {
            viewport,
            total_items,
        }
    }

    fn set_total_items(&mut self, count: usize) {
        self.total_items = count;
    }
}

pub struct ColumnDef<T: 'static> {
    pub id: SharedString,
    pub header: SharedString,
    pub accessor: Rc<dyn Fn(&T) -> SharedString>,
    pub width: Pixels,
    pub min_width: Pixels,
    pub resizable: bool,
    pub sortable: bool,
    pub editable: bool,
}

impl<T: 'static> ColumnDef<T> {
    pub fn new<S: Into<SharedString>, F: Fn(&T) -> SharedString + 'static>(
        id: S,
        header: S,
        accessor: F,
    ) -> Self {
        let id_string: SharedString = id.into();
        let header_string: SharedString = header.into();

        Self {
            id: id_string,
            header: header_string,
            accessor: Rc::new(accessor),
            width: px(150.0),
            min_width: px(80.0),
            resizable: true,
            sortable: true,
            editable: false,
        }
    }

    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = width.into();
        self
    }

    pub fn min_width(mut self, width: impl Into<Pixels>) -> Self {
        self.min_width = width.into();
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }
}

enum DataBacking<T: Clone + 'static> {
    InMemory { data: Vec<T> },
    Virtual(VirtualPageState<T>),
}

pub struct DataTableState<T: Clone + 'static> {
    columns: Vec<ColumnDef<T>>,
    column_widths: Vec<Pixels>,
    sort_column: Option<usize>,
    sort_direction: SortDirection,
    sort_comparators: HashMap<SharedString, RowComparator<T>>,
    scroller: VirtualScroller,
    selected_rows: Vec<usize>,
    backing: DataBacking<T>,
    tokenized_page_requests: bool,
}

impl<T: Clone + 'static> DataTableState<T> {
    pub fn new(data: Vec<T>, columns: Vec<ColumnDef<T>>) -> Self {
        let column_widths = columns
            .iter()
            .map(|col| col.width.max(col.min_width))
            .collect();
        let total_items = data.len();
        let viewport = ViewportState::new(48.0, 600.0);

        Self {
            column_widths,
            columns,
            sort_column: None,
            sort_direction: SortDirection::Ascending,
            sort_comparators: HashMap::new(),
            scroller: VirtualScroller::new(total_items, viewport),
            selected_rows: Vec::new(),
            backing: DataBacking::InMemory { data },
            tokenized_page_requests: false,
        }
    }

    fn row_height(&self) -> f32 {
        self.scroller.viewport.row_height
    }

    fn viewport_height(&self) -> f32 {
        self.scroller.viewport.viewport_height
    }

    fn total_items(&self) -> usize {
        match &self.backing {
            DataBacking::InMemory { data } => data.len(),
            DataBacking::Virtual(pages) => pages.total_items,
        }
    }

    fn get_row(&self, index: usize) -> Option<&T> {
        match &self.backing {
            DataBacking::InMemory { data } => data.get(index),
            DataBacking::Virtual(pages) => pages.cache.get(&index),
        }
    }

    fn replace_in_memory_data(&mut self, data: Vec<T>) {
        let mut data = data;
        self.sort_in_memory_data(&mut data);
        let count = data.len();
        self.backing = DataBacking::InMemory { data };
        self.scroller.set_total_items(count);
    }

    fn sort_in_memory_data(&self, data: &mut Vec<T>) {
        let Some(column_index) = self.sort_column else {
            return;
        };
        let Some(column) = self.columns.get(column_index) else {
            return;
        };
        Self::sort_rows(
            data,
            column.accessor.clone(),
            self.sort_comparators.get(&column.id).cloned(),
            self.sort_direction,
        );
    }

    fn resort_current_in_memory_data(&mut self) {
        let Some(column_index) = self.sort_column else {
            return;
        };
        let Some(column) = self.columns.get(column_index) else {
            return;
        };
        let accessor = column.accessor.clone();
        let comparator = self.sort_comparators.get(&column.id).cloned();
        let direction = self.sort_direction;
        if let DataBacking::InMemory { data } = &mut self.backing {
            Self::sort_rows(data, accessor, comparator, direction);
        }
    }

    fn append_in_memory_data(&mut self, mut new_data: Vec<T>) -> bool {
        if let DataBacking::InMemory { data } = &mut self.backing {
            data.append(&mut new_data);
        } else {
            return false;
        }
        self.resort_current_in_memory_data();
        true
    }

    fn sort_rows(
        data: &mut Vec<T>,
        accessor: Rc<dyn Fn(&T) -> SharedString>,
        comparator: Option<RowComparator<T>>,
        direction: SortDirection,
    ) {
        if let Some(comparator) = comparator {
            data.sort_by(|a, b| {
                let order = comparator(a, b);
                match direction {
                    SortDirection::Ascending => order,
                    SortDirection::Descending => order.reverse(),
                }
            });
            return;
        }
        let mut indexed_values: Vec<(usize, String)> = data
            .iter()
            .enumerate()
            .map(|(idx, row)| (idx, accessor(row).to_string()))
            .collect();

        indexed_values.sort_by(|(_, a), (_, b)| {
            let ordering = compare_cell_values(a, b);
            match direction {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            }
        });

        *data = indexed_values
            .into_iter()
            .filter_map(|(idx, _)| data.get(idx).cloned())
            .collect();
    }

    fn virtual_reset(&mut self, total_items: usize, page_size: Option<usize>) {
        let previous_page_size = match &self.backing {
            DataBacking::Virtual(pages) => pages.page_size,
            DataBacking::InMemory { .. } => 200,
        };
        self.backing = DataBacking::Virtual(VirtualPageState::new(
            total_items,
            page_size.unwrap_or(previous_page_size),
        ));
        // Virtual pages arrive in source order and cannot be reordered locally.
        // Clear the indicator so it never claims an order that the cache does
        // not actually provide.
        self.sort_column = None;
        self.sort_direction = SortDirection::Ascending;
        self.scroller.set_total_items(total_items);
        self.selected_rows.clear();
    }

    fn set_tokenized_page_requests(&mut self, tokenized: bool) {
        if self.tokenized_page_requests == tokenized {
            return;
        }
        self.tokenized_page_requests = tokenized;
        if let DataBacking::Virtual(pages) = &self.backing {
            self.virtual_reset(pages.total_items, Some(pages.page_size));
        }
    }

    fn virtual_set_page(&mut self, page_start: usize, rows: Vec<T>) -> bool {
        // Legacy completions have no identity and must not enter tokenized
        // caches or acknowledge a tokenized request.
        if self.tokenized_page_requests {
            return false;
        }
        let DataBacking::Virtual(pages) = &mut self.backing else {
            return false;
        };
        pages.set_legacy_page(page_start, rows);
        true
    }

    /// Override a column's ascending row order independently of its display
    /// accessor. The comparator must provide a consistent total order.
    /// Returns `false` for an unknown column ID.
    pub fn set_column_comparator(
        &mut self,
        column_id: impl Into<SharedString>,
        comparator: impl Fn(&T, &T) -> std::cmp::Ordering + 'static,
    ) -> bool {
        self.install_column_comparator(column_id.into(), Rc::new(comparator))
            .is_some()
    }

    /// Returns `None` for an unknown column, otherwise whether rows were
    /// reordered. Only reordering invalidates index-based selection.
    fn install_column_comparator(
        &mut self,
        column_id: SharedString,
        comparator: RowComparator<T>,
    ) -> Option<bool> {
        if !self.columns.iter().any(|column| column.id == column_id) {
            return None;
        }
        let reorders_rows = matches!(&self.backing, DataBacking::InMemory { .. })
            && self
                .sort_column
                .and_then(|index| self.columns.get(index))
                .is_some_and(|column| column.id == column_id);
        self.sort_comparators.insert(column_id, comparator);
        if reorders_rows {
            self.resort_current_in_memory_data();
            self.clear_selection();
        }
        Some(reorders_rows)
    }

    pub fn sort_by_column(&mut self, column_index: usize, direction: SortDirection) {
        let Some(column) = self.columns.get(column_index) else {
            return;
        };
        if !column.sortable {
            return;
        }

        // Virtual tables delegate ordering to on_sort_change instead of
        // reordering their partial page cache locally.
        if !matches!(&self.backing, DataBacking::InMemory { .. }) {
            return;
        }

        self.sort_column = Some(column_index);
        self.sort_direction = direction;

        if let DataBacking::InMemory { data } = &mut self.backing {
            Self::sort_rows(
                data,
                column.accessor.clone(),
                self.sort_comparators.get(&column.id).cloned(),
                direction,
            );
            self.selected_rows.clear();
        }
    }

    pub fn toggle_row(&mut self, row_index: usize) {
        if let Some(pos) = self.selected_rows.iter().position(|&i| i == row_index) {
            self.selected_rows.remove(pos);
        } else {
            self.selected_rows.push(row_index);
        }
    }

    pub fn is_row_selected(&self, row_index: usize) -> bool {
        self.selected_rows.contains(&row_index)
    }

    fn clear_selection(&mut self) {
        self.selected_rows.clear();
    }

    pub fn resize_column(&mut self, column_index: usize, new_width: Pixels) {
        if let (Some(width), Some(column)) = (
            self.column_widths.get_mut(column_index),
            self.columns.get(column_index),
        ) {
            *width = new_width.max(column.min_width);
        }
    }
}

pub struct DataTable<T: Clone + 'static> {
    state: DataTableState<T>,
    resizing_column: Option<usize>,
    resize_start_x: f32,
    resize_start_width: Pixels,
    on_load_more: Option<LoadMoreCallback<Self>>,
    load_more_threshold: f32,
    load_more_triggered: bool,
    scroll_handle: ScrollHandle,
    editing_cell: Option<(usize, usize)>,
    edit_input: Option<Entity<InputState>>,
    edit_column_id: SharedString,
    edit_old_value: SharedString,
    use_edit_dialog: bool,
    on_cell_edit: Option<CellEditCallback<Self>>,
    on_cell_double_click: Option<CellDoubleClickCallback<T>>,
    on_fetch_page: Option<FetchPageCallback<Self>>,
    on_fetch_page_request: Option<FetchPageRequestCallback<Self>>,
    on_row_click: Option<RowClickCallback<T>>,
    on_sort_change: Option<SortChangeCallback<Self>>,
    on_search_change: Option<SearchChangeCallback<Self>>,
    search_query: String,
    search_column: Option<usize>,
    show_search: bool,
    search_column_select: Entity<Select<usize>>,
    search_input: Entity<InputState>,
    show_selection: bool,
    on_selection_change: Option<SelectionChangeCallback<Self>>,
    row_actions: Vec<RowAction>,
    context_menu: Option<(usize, Point<Pixels>)>,
    style: StyleRefinement,
}

impl<T: Clone + 'static> DataTable<T> {
    pub fn new(data: Vec<T>, columns: Vec<ColumnDef<T>>, cx: &mut Context<Self>) -> Self {
        Self::from_state(DataTableState::new(data, columns), cx)
    }

    fn from_state(state: DataTableState<T>, cx: &mut Context<Self>) -> Self {
        let (search_column_select, search_input) = Self::create_search_controls(&state.columns, cx);

        Self {
            state,
            resizing_column: None,
            resize_start_x: 0.0,
            resize_start_width: px(0.0),
            on_load_more: None,
            load_more_threshold: 0.7,
            load_more_triggered: false,
            scroll_handle: ScrollHandle::new(),
            editing_cell: None,
            edit_input: None,
            edit_column_id: SharedString::from(""),
            edit_old_value: SharedString::from(""),
            use_edit_dialog: false,
            on_cell_edit: None,
            on_cell_double_click: None,
            on_fetch_page: None,
            on_fetch_page_request: None,
            on_row_click: None,
            on_sort_change: None,
            on_search_change: None,
            search_query: String::new(),
            search_column: None,
            show_search: true,
            search_column_select,
            search_input,
            show_selection: false,
            on_selection_change: None,
            row_actions: Vec::new(),
            context_menu: None,
            style: StyleRefinement::default(),
        }
    }

    fn create_search_controls(
        columns: &[ColumnDef<T>],
        cx: &mut Context<Self>,
    ) -> (Entity<Select<usize>>, Entity<InputState>) {
        let mut select_options = vec![SelectOption::new(usize::MAX, "All Columns")];
        select_options.extend(
            columns
                .iter()
                .enumerate()
                .map(|(idx, column)| SelectOption::new(idx, column.header.clone())),
        );

        let search_column_select = cx.new(|cx| {
            Select::new(cx)
                .options(select_options)
                .selected_index(Some(0))
                .placeholder("Select column...")
        });

        cx.subscribe(
            &search_column_select,
            |this, _select, event: &SelectEvent, cx| match event {
                SelectEvent::Change => {
                    let selected = this.search_column_select.read(cx).selected_value().copied();
                    this.search_column =
                        (selected != Some(usize::MAX)).then_some(selected).flatten();
                    this.dispatch_search_change(cx);
                    cx.notify();
                }
            },
        )
        .detach();

        (search_column_select, cx.new(InputState::new))
    }

    #[deprecated(note = "DataTable keeps its header outside the virtualized body; use the default")]
    pub fn sticky_header(self, _sticky: bool) -> Self {
        self
    }

    pub fn show_selection(mut self, show: bool) -> Self {
        self.show_selection = show;
        self
    }

    pub fn on_selection_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&[usize], &mut Window, &mut Context<Self>) + 'static,
    {
        self.on_selection_change = Some(Box::new(callback));
        self
    }

    pub fn new_virtual(
        total_items: usize,
        columns: Vec<ColumnDef<T>>,
        page_size: usize,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut state = DataTableState::new(Vec::new(), columns);
        state.virtual_reset(total_items, Some(page_size));
        Self::from_state(state, cx)
    }

    /// Set callback for when user scrolls near the end
    ///
    /// This is useful for infinite scrolling / pagination - load more data when needed
    pub fn on_load_more<F>(mut self, callback: F) -> Self
    where
        F: Fn(&mut Window, &mut Context<Self>) + 'static,
    {
        self.on_load_more = Some(Box::new(callback));
        self
    }

    /// Set the threshold (0.0-1.0) for when to trigger load_more
    ///
    /// Default is 0.7 (70%) - callback fires when user scrolls past 70% of loaded data
    pub fn load_more_threshold(mut self, threshold: f32) -> Self {
        self.load_more_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Legacy page loader receiving only `(start, size)`.
    ///
    /// [`Self::set_page_data`] cannot distinguish late results after a reset
    /// or retry. Callers must cancel or discard those results themselves.
    /// Prefer [`Self::on_fetch_page_request`] for asynchronous sources.
    /// Registering either loader replaces the other loader; switching loader
    /// kinds resets virtual pages and invalidates outstanding requests.
    pub fn on_fetch_page<F>(mut self, callback: F) -> Self
    where
        F: Fn(usize, usize, &mut Window, &mut Context<Self>) + 'static,
    {
        self.on_fetch_page = Some(Box::new(callback));
        self.on_fetch_page_request = None;
        self.state.set_tokenized_page_requests(false);
        self
    }

    /// Load virtual pages with reset and retry safe request identities.
    ///
    /// Fetch `request.start()` and `request.size()` from the source, then pass
    /// that same token to [`Self::complete_page_request`] on success or
    /// [`Self::fail_page_request`] on failure. Call [`Self::virtual_reset`]
    /// when the source query, sort, or total changes. Old completions are
    /// ignored. This loader disables legacy [`Self::set_page_data`] writes.
    /// Switching loader kinds resets virtual pages and outstanding requests.
    pub fn on_fetch_page_request<F>(mut self, callback: F) -> Self
    where
        F: Fn(VirtualPageRequest, &mut Window, &mut Context<Self>) + 'static,
    {
        self.on_fetch_page_request = Some(Box::new(callback));
        self.on_fetch_page = None;
        self.state.set_tokenized_page_requests(true);
        self
    }

    /// Set the callback used to request remote sorting for virtual tables.
    ///
    /// In-memory tables sort locally. Virtual tables do not reorder cached
    /// pages, so this callback must request sorted data from the backing
    /// source and refresh the table when the result arrives.
    pub fn on_sort_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(SharedString, SortDirection, &mut Window, &mut Context<Self>) + 'static,
    {
        self.on_sort_change = Some(Box::new(callback));
        self
    }

    /// Handle virtual-table search changes as `(query, column_id, context)`.
    /// `None` means all columns. Refresh the remote source and call
    /// [`Self::virtual_reset`] to invalidate pages from the previous search.
    /// Virtual tables show search controls only when this handler exists;
    /// in-memory tables continue filtering their display values locally.
    ///
    /// The table is being updated while this runs, so updating it again from
    /// inside the handler panics. Defer the reset instead:
    ///
    /// ```ignore
    /// table.on_search_change(|query, column, cx| {
    ///     let table = cx.entity().downgrade();
    ///     cx.defer(move |cx| {
    ///         if let Some(table) = table.upgrade() {
    ///             table.update(cx, |table, cx| table.virtual_reset(total_for(&query, column), None, cx));
    ///         }
    ///     });
    /// })
    /// ```
    ///
    /// Pages requested before the deferred reset carry the previous
    /// generation and are rejected by [`Self::complete_page_request`].
    pub fn on_search_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(String, Option<SharedString>, &mut Context<Self>) + 'static,
    {
        self.on_search_change = Some(Box::new(callback));
        self
    }

    /// Register a column's ascending row comparator without changing its
    /// display/search accessor or the public [`ColumnDef`] struct shape.
    pub fn column_comparator(
        mut self,
        column_id: impl Into<SharedString>,
        comparator: impl Fn(&T, &T) -> std::cmp::Ordering + 'static,
    ) -> Self {
        self.state.set_column_comparator(column_id, comparator);
        self
    }

    /// Update a comparator and reapply any active in-memory sort. Reordering
    /// discards the positional editor and the index-based selection, and
    /// notifies selection observers when a selection was cleared.
    pub fn set_column_comparator(
        &mut self,
        column_id: impl Into<SharedString>,
        comparator: impl Fn(&T, &T) -> std::cmp::Ordering + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let had_selection = !self.state.selected_rows.is_empty();
        let Some(reordered) = self
            .state
            .install_column_comparator(column_id.into(), Rc::new(comparator))
        else {
            return false;
        };
        if reordered {
            self.cancel_edit(cx);
            if had_selection {
                if let Some(ref callback) = self.on_selection_change {
                    callback(&self.state.selected_rows, window, cx);
                }
            }
        }
        cx.notify();
        true
    }

    /// Set callback for when a cell is edited
    ///
    /// The callback receives: (row_index, column_id, old_value, new_value, context)
    pub fn on_cell_edit<F>(mut self, callback: F) -> Self
    where
        F: Fn(usize, SharedString, SharedString, SharedString, &mut Context<Self>) + 'static,
    {
        self.on_cell_edit = Some(Box::new(callback));
        self
    }

    /// Set callback for cell double-click: (row_data, column_id, cell_value)
    pub fn on_cell_double_click<F>(mut self, callback: F) -> Self
    where
        F: Fn(&T, SharedString, SharedString, &mut Window, &mut Context<Self>) + 'static,
    {
        self.on_cell_double_click = Some(Box::new(callback));
        self
    }

    /// Set whether losing focus keeps the in-progress cell edit
    ///
    /// No confirmation dialog is rendered; the flag only controls blur behavior.
    /// - `false` (default): losing focus saves the edit (Enter always saves)
    /// - `true`: losing focus discards the edit
    pub fn use_edit_dialog(mut self, use_dialog: bool) -> Self {
        self.use_edit_dialog = use_dialog;
        self
    }

    /// Set callback for when a row is clicked
    ///
    pub fn on_row_click<F>(mut self, callback: F) -> Self
    where
        F: Fn(usize, &T, &mut Window, &mut Context<Self>) + 'static,
    {
        self.on_row_click = Some(Box::new(callback));
        self
    }

    /// Show search controls when the backing supports searching. A virtual
    /// table also requires [`Self::on_search_change`], even when `show` is true.
    pub fn show_search(mut self, show: bool) -> Self {
        self.show_search = show;
        self
    }

    pub fn row_actions(mut self, actions: Vec<RowAction>) -> Self {
        self.row_actions = actions;
        self
    }

    pub fn set_search(&mut self, query: String, cx: &mut Context<Self>) {
        self.search_query = query;
        self.dispatch_search_change(cx);
        cx.notify();
    }

    pub fn set_search_column(&mut self, column_index: Option<usize>, cx: &mut Context<Self>) {
        self.search_column = column_index;
        self.dispatch_search_change(cx);
        cx.notify();
    }

    fn dispatch_search_change(&self, cx: &mut Context<Self>) {
        if !matches!(&self.state.backing, DataBacking::Virtual(_)) {
            return;
        }
        if let Some(callback) = &self.on_search_change {
            let column_id = self.search_column.and_then(|index| {
                self.state
                    .columns
                    .get(index)
                    .map(|column| column.id.clone())
            });
            callback(self.search_query.clone(), column_id, cx);
        }
    }

    fn can_show_search(&self) -> bool {
        self.show_search
            && (matches!(&self.state.backing, DataBacking::InMemory { .. })
                || self.on_search_change.is_some())
    }

    fn sort_from_header(
        &mut self,
        column_index: usize,
        direction: SortDirection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(&self.state.backing, DataBacking::Virtual(_)) {
            self.state.sort_by_column(column_index, direction);
            return;
        }

        let Some(callback) = self.on_sort_change.as_ref() else {
            return;
        };
        let Some(column) = self.state.columns.get(column_index) else {
            return;
        };

        self.state.sort_column = Some(column_index);
        self.state.sort_direction = direction;
        self.state.selected_rows.clear();
        callback(column.id.clone(), direction, window, cx);
    }

    fn row_matches_search(&self, row: &T, query_lower: &str) -> bool {
        if query_lower.is_empty() {
            return true;
        }

        if let Some(col_idx) = self.search_column {
            if let Some(column) = self.state.columns.get(col_idx) {
                let cell_value = (column.accessor)(row);
                cell_value.to_string().to_lowercase().contains(query_lower)
            } else {
                false
            }
        } else {
            self.state.columns.iter().any(|column| {
                let cell_value = (column.accessor)(row);
                cell_value.to_string().to_lowercase().contains(query_lower)
            })
        }
    }

    fn get_filtered_indices(&self) -> Vec<usize> {
        if let DataBacking::InMemory { data } = &self.state.backing {
            let query_lower = self.search_query.to_lowercase();
            data.iter()
                .enumerate()
                .filter(|(_, row)| self.row_matches_search(row, &query_lower))
                .map(|(idx, _)| idx)
                .collect()
        } else {
            (0..self.state.total_items()).collect()
        }
    }

    pub fn set_data(&mut self, data: Vec<T>, cx: &mut Context<Self>) {
        // Replacing rows invalidates the editing index (and the input entity)
        // even when the new data has the same length.
        self.cancel_edit(cx);
        self.state.replace_in_memory_data(data);
        self.state.clear_selection();
        self.load_more_triggered = false;
        cx.notify();
    }

    /// Replace in-memory data and notify selection observers when the old
    /// index-based selection is cleared. Use this variant from an event
    /// handler that has a window context.
    pub fn set_data_with_window(
        &mut self,
        data: Vec<T>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let had_selection = !self.state.selected_rows.is_empty();
        self.cancel_edit(cx);
        self.state.replace_in_memory_data(data);
        self.state.clear_selection();
        self.load_more_triggered = false;
        if had_selection {
            if let Some(ref callback) = self.on_selection_change {
                callback(&self.state.selected_rows, window, cx);
            }
        }
        cx.notify();
    }

    pub fn append_data(&mut self, new_data: Vec<T>, cx: &mut Context<Self>) {
        // Appending under an active sort can reorder existing rows, so discard
        // the positional edit before changing the backing data.
        self.cancel_edit(cx);
        let was_sorted = self.state.sort_column.is_some();
        if !self.state.append_in_memory_data(new_data) {
            return;
        }
        if was_sorted {
            // Selection is represented by row indices. Reordering can make an
            // existing index refer to a different row, so clear it explicitly.
            self.state.clear_selection();
        }
        let new_count = self.state.total_items();
        self.state.scroller.set_total_items(new_count);
        self.load_more_triggered = false;
        cx.notify();
    }

    pub fn virtual_reset(
        &mut self,
        total_items: usize,
        page_size: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        self.cancel_edit(cx);
        self.state.virtual_reset(total_items, page_size);
        self.state.clear_selection();
        self.load_more_triggered = false;
        cx.notify();
    }

    /// Reset virtual data and notify selection observers when stale row
    /// indices are discarded.
    pub fn virtual_reset_with_window(
        &mut self,
        total_items: usize,
        page_size: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let had_selection = !self.state.selected_rows.is_empty();
        self.cancel_edit(cx);
        self.state.virtual_reset(total_items, page_size);
        self.load_more_triggered = false;
        if had_selection {
            if let Some(ref callback) = self.on_selection_change {
                callback(&self.state.selected_rows, window, cx);
            }
        }
        cx.notify();
    }

    /// Install legacy page results. This API cannot reject results from an
    /// earlier query/reset; the caller owns cancellation or stale-result checks.
    /// Writes are ignored when [`Self::on_fetch_page_request`] is configured.
    pub fn set_page_data(&mut self, page_start: usize, rows: Vec<T>, cx: &mut Context<Self>) {
        // A page refresh may replace the row currently being edited. The
        // virtual backing has no stable row identity, so drop the positional
        // editor before installing new cache entries.
        if !self.state.virtual_set_page(page_start, rows) {
            return;
        }
        self.cancel_edit(cx);
        self.load_more_triggered = false;
        cx.notify();
    }

    /// Complete exactly one tokenized request. Returns `false` for stale,
    /// failed, duplicate, or reset-invalidated tokens without changing rows.
    /// A short/empty success completes the page; reset to reload it later.
    pub fn complete_page_request(
        &mut self,
        request: VirtualPageRequest,
        rows: Vec<T>,
        cx: &mut Context<Self>,
    ) -> bool {
        let DataBacking::Virtual(pages) = &mut self.state.backing else {
            return false;
        };
        if !pages.complete(request, rows) {
            return false;
        }
        self.cancel_edit(cx);
        self.load_more_triggered = false;
        cx.notify();
        true
    }

    /// Release a failed tokenized request so a later visible-range update can
    /// retry it. Returns `false` for stale/already completed tokens. This does
    /// not trigger an immediate render/retry loop; notify the context when an
    /// immediate retry is desired.
    pub fn fail_page_request(
        &mut self,
        request: VirtualPageRequest,
        _cx: &mut Context<Self>,
    ) -> bool {
        let DataBacking::Virtual(pages) = &mut self.state.backing else {
            return false;
        };
        pages.fail(request)
    }

    /// Release a legacy page after a failed fetch. Callers must ensure this
    /// failure belongs to the current query; only tokenized requests can
    /// identify and reject stale failures.
    pub fn fail_page(&mut self, page_start: usize, _cx: &mut Context<Self>) -> bool {
        if self.state.tokenized_page_requests {
            return false;
        }
        let DataBacking::Virtual(pages) = &mut self.state.backing else {
            return false;
        };
        pages.legacy_pending.remove(&page_start)
    }

    fn fetch_visible_pages(
        &mut self,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.on_fetch_page.is_none() && self.on_fetch_page_request.is_none() {
            return;
        }
        let DataBacking::Virtual(pages) = &mut self.state.backing else {
            return;
        };
        let requests = pages.requests_for_range(range, self.state.tokenized_page_requests);
        for request in requests {
            if let Some(callback) = &self.on_fetch_page_request {
                callback(request, window, cx);
            } else if let Some(callback) = &self.on_fetch_page {
                callback(request.start(), request.size(), window, cx);
            }
        }
    }

    pub fn data(&self) -> &[T] {
        match &self.state.backing {
            DataBacking::InMemory { data } => data,
            _ => &[],
        }
    }

    pub fn data_count(&self) -> usize {
        self.state.total_items()
    }

    pub fn selected_rows(&self) -> &[usize] {
        &self.state.selected_rows
    }

    pub fn toggle_row_selection(
        &mut self,
        row_index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.toggle_row(row_index);

        if let Some(ref callback) = self.on_selection_change {
            callback(&self.state.selected_rows, window, cx);
        }

        cx.notify();
    }

    pub fn select_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let total = self.state.total_items();
        self.state.selected_rows = (0..total).collect();

        if let Some(ref callback) = self.on_selection_change {
            callback(&self.state.selected_rows, window, cx);
        }

        cx.notify();
    }

    pub fn clear_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.state.selected_rows.clear();

        if let Some(ref callback) = self.on_selection_change {
            callback(&self.state.selected_rows, window, cx);
        }

        cx.notify();
    }

    fn is_all_selected(&self) -> bool {
        let total = self.state.total_items();
        total > 0 && self.state.selected_rows.len() == total
    }

    fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        if self.editing_cell.is_none() {
            return;
        }

        self.editing_cell = None;
        self.edit_input = None;
        self.edit_column_id = SharedString::default();
        self.edit_old_value = SharedString::default();
        cx.notify();
    }

    fn total_table_width(&self) -> Pixels {
        let mut total: f32 = self
            .state
            .column_widths
            .iter()
            .map(|w| {
                let w_f32: f32 = (*w).into();
                w_f32
            })
            .sum();

        if self.show_selection {
            total += 50.0;
        }

        px(total)
    }

    fn save_edit(&mut self, cx: &mut Context<Self>) {
        if let Some((row_idx, _col_idx)) = self.editing_cell {
            let new_value_string: String = if let Some(ref input) = self.edit_input {
                input.read(cx).content().to_string()
            } else {
                String::new()
            };

            let row_idx_copy = row_idx;
            let column_id = self.edit_column_id.clone();
            let old_value = self.edit_old_value.clone();

            self.editing_cell = None;
            self.edit_input = None;

            if let Some(ref callback) = self.on_cell_edit {
                callback(
                    row_idx_copy,
                    column_id,
                    old_value,
                    new_value_string.into(),
                    cx,
                );
            }

            cx.notify();
        }
    }

    fn render_search_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = use_theme(cx);

        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(16.0))
            .py(px(12.0))
            .border_b_1()
            .border_color(theme.tokens.border)
            .bg(theme.tokens.muted.opacity(0.3))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(theme.tokens.muted_foreground)
                            .child("Search in:"),
                    )
                    .child(div().w(px(200.0)).child(self.search_column_select.clone())),
            )
            .child(
                div().w(px(300.0)).child(
                    Input::new(&self.search_input)
                        .size(InputSize::Sm)
                        .placeholder("Type to search...")
                        .on_change({
                            let entity = cx.entity();
                            move |value: SharedString, cx| {
                                entity.update(cx, |this, cx| {
                                    this.set_search(value.to_string(), cx);
                                });
                            }
                        }),
                ),
            )
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = use_theme(cx);

        let total_width = self.total_table_width();
        let mut header_row = div().flex().w(total_width).min_w(total_width);

        if self.show_selection {
            let all_selected = self.is_all_selected();

            header_row = header_row.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(50.0))
                    .px(px(16.0))
                    .py(px(12.0))
                    .text_size(px(13.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.tokens.muted_foreground)
                    .border_b_1()
                    .border_r_1()
                    .border_color(theme.tokens.border)
                    .bg(theme.tokens.muted.opacity(0.5))
                    .cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(theme.tokens.muted.opacity(0.7)))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _event, window, cx| {
                            if this.is_all_selected() {
                                this.clear_selection(window, cx);
                            } else {
                                this.select_all(window, cx);
                            }
                        }),
                    )
                    .child(
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(if all_selected {
                                theme.tokens.primary
                            } else {
                                theme.tokens.border
                            })
                            .bg(if all_selected {
                                theme.tokens.primary
                            } else {
                                theme.tokens.background
                            }),
                    ),
            );
        }

        let header_cells = self
            .state
            .columns
            .iter()
            .enumerate()
            .map(|(col_idx, column)| {
                let width = self.state.column_widths[col_idx];
                let is_sorted = self.state.sort_column == Some(col_idx);
                let sortable = column.sortable;

                let mut header_cell = div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(16.0))
                    .py(px(12.0))
                    .w(width)
                    .text_size(px(13.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.tokens.muted_foreground)
                    .border_b_1()
                    .border_r_1()
                    .border_color(theme.tokens.border)
                    .bg(theme.tokens.muted.opacity(0.5))
                    .hover(|style| {
                        if sortable {
                            style
                                .bg(theme.tokens.muted.opacity(0.7))
                                .cursor(CursorStyle::PointingHand)
                        } else {
                            style
                        }
                    })
                    .child(sortable_header_content(
                        column.header.clone(),
                        if is_sorted {
                            match self.state.sort_direction {
                                SortDirection::Ascending => HeaderSortState::Ascending,
                                SortDirection::Descending => HeaderSortState::Descending,
                            }
                        } else {
                            HeaderSortState::None
                        },
                        px(8.0),
                    ));

                if sortable {
                    header_cell = header_cell.on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _event, window, cx| {
                            let new_direction = if this.state.sort_column == Some(col_idx) {
                                match this.state.sort_direction {
                                    SortDirection::Ascending => SortDirection::Descending,
                                    SortDirection::Descending => SortDirection::Ascending,
                                }
                            } else {
                                SortDirection::Ascending
                            };

                            if this.editing_cell.is_some() {
                                if this.use_edit_dialog {
                                    this.cancel_edit(cx);
                                } else {
                                    this.save_edit(cx);
                                }
                            }
                            let had_selection = !this.state.selected_rows.is_empty();
                            this.sort_from_header(col_idx, new_direction, window, cx);
                            if had_selection {
                                if let Some(ref callback) = this.on_selection_change {
                                    callback(&this.state.selected_rows, window, cx);
                                }
                            }
                            cx.notify();
                        }),
                    );
                }

                header_cell = header_cell.when(column.resizable, |el| {
                    el.child(column_resize_handle(
                        theme.tokens.primary.opacity(0.5),
                        cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                            cx.stop_propagation();
                            this.resizing_column = Some(col_idx);
                            this.resize_start_x = event.position.x.into();
                            this.resize_start_width = this.state.column_widths[col_idx];
                            cx.notify();
                        }),
                    ))
                });

                header_cell
            });

        header_row.children(header_cells)
    }
}

impl<T: Clone + 'static> Styled for DataTable<T> {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl<T: Clone + 'static> Render for DataTable<T> {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = use_theme(cx);

        let user_style = self.style.clone();

        let viewport_height = self.state.viewport_height();

        let (total_items, filtered_indices): (usize, Option<Rc<Vec<usize>>>) =
            match &self.state.backing {
                DataBacking::InMemory { .. } => {
                    let indices = Rc::new(self.get_filtered_indices());
                    (indices.len(), Some(indices))
                }
                DataBacking::Virtual(_) => (self.state.total_items(), None),
            };
        let row_extent = px(self.state.row_height());
        let total_width = self.total_table_width();

        let view_entity = cx.entity().clone();
        let filtered_indices_for_render = filtered_indices.clone();
        let renderer = move |this: &mut DataTable<T>,
                             range: Range<usize>,
                             _window: &mut Window,
                             cx: &mut Context<DataTable<T>>| {
            let theme = use_theme(cx);
            range
                .map(|row_idx| {
                    let actual_idx = if let Some(ref map) = filtered_indices_for_render {
                        map.get(row_idx).copied().unwrap_or(row_idx)
                    } else {
                        row_idx
                    };

                    if let Some(row_data) = this.state.get_row(actual_idx) {
                        let is_selected = this.state.is_row_selected(actual_idx);

                        let mut row_div = div()
                            .flex()
                            .w(total_width)
                            .min_w(total_width)
                            .h(row_extent)
                            .bg(if is_selected {
                                theme.tokens.accent.opacity(0.2)
                            } else if row_idx % 2 == 0 {
                                theme.tokens.background
                            } else {
                                theme.tokens.muted.opacity(0.3)
                            })
                            .hover(|style| style.bg(theme.tokens.accent.opacity(0.1)));

                        if !this.row_actions.is_empty() {
                            row_div = row_div.on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                                    this.context_menu = Some((actual_idx, event.position));
                                    cx.notify();
                                }),
                            );
                        }

                        if this.on_row_click.is_some() {
                            row_div = row_div.on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                                    if event.click_count > 1 {
                                        return;
                                    }
                                    if let Some(row) = this.state.get_row(actual_idx) {
                                        if let Some(ref cb) = this.on_row_click {
                                            (cb)(actual_idx, row, window, cx);
                                        }
                                    }
                                }),
                            );
                        }

                        if this.show_selection {
                            row_div = row_div.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(50.0))
                                    .px(px(16.0))
                                    .py(px(12.0))
                                    .border_b_1()
                                    .border_r_1()
                                    .border_color(theme.tokens.border.opacity(0.5))
                                    .cursor(CursorStyle::PointingHand)
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, _event, window, cx| {
                                            this.toggle_row_selection(actual_idx, window, cx);
                                        }),
                                    )
                                    .child(
                                        div()
                                            .w(px(16.0))
                                            .h(px(16.0))
                                            .rounded(px(3.0))
                                            .border_1()
                                            .border_color(if is_selected {
                                                theme.tokens.primary
                                            } else {
                                                theme.tokens.border
                                            })
                                            .bg(if is_selected {
                                                theme.tokens.primary
                                            } else {
                                                theme.tokens.background
                                            }),
                                    ),
                            );
                        }

                        let cells =
                            this.state
                                .columns
                                .iter()
                                .enumerate()
                                .map(|(col_idx, column)| {
                                    let width = this.state.column_widths[col_idx];
                                    let cell_value = (column.accessor)(row_data);
                                    let is_editable = column.editable;
                                    let is_editing =
                                        this.editing_cell == Some((actual_idx, col_idx));

                                    let mut cell_div = div()
                                        .flex()
                                        .items_center()
                                        .px(px(16.0))
                                        .py(px(12.0))
                                        .w(width)
                                        .text_size(px(13.0))
                                        .text_color(theme.tokens.foreground)
                                        .border_b_1()
                                        .border_r_1()
                                        .border_color(theme.tokens.border.opacity(0.5))
                                        .overflow_hidden()
                                        .text_ellipsis();

                                    if is_editable && !is_editing {
                                        let cell_value_for_closure = cell_value.clone();
                                        let column_id = column.id.clone();
                                        let row_data_clone = row_data.clone();
                                        cell_div = cell_div
                                            .cursor(CursorStyle::IBeam)
                                            .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(
                                                move |this, event: &MouseDownEvent, window, cx| {
                                                    if event.click_count < 2 {
                                                        return;
                                                    }

                                                    if this.on_cell_double_click.is_some() {
                                                        if let Some(ref cb) =
                                                            this.on_cell_double_click
                                                        {
                                                            (cb)(
                                                                &row_data_clone,
                                                                column_id.clone(),
                                                                cell_value_for_closure.clone(),
                                                                window,
                                                                cx,
                                                            );
                                                        }
                                                        return;
                                                    }

                                                    let input_state = cx.new(|cx| {
                                                        let mut state = InputState::new(cx);
                                                        state.set_value(
                                                            cell_value_for_closure.clone(),
                                                            window,
                                                            cx,
                                                        );
                                                        state
                                                    });
                                                    use crate::capabilities::controls::input::InputEvent;
                                                    cx.subscribe(
                                                        &input_state,
                                                        |this, _, event: &InputEvent, cx| {
                                                            match event {
                                                                InputEvent::Enter => {
                                                                    this.save_edit(cx)
                                                                }
                                                                InputEvent::Blur => {
                                                                    if !this.use_edit_dialog {
                                                                        this.save_edit(cx);
                                                                    } else {
                                                                        this.cancel_edit(cx);
                                                                    }
                                                                }
                                                                _ => {}
                                                            }
                                                        },
                                                    )
                                                    .detach();
                                                    this.editing_cell = Some((actual_idx, col_idx));
                                                    this.edit_input = Some(input_state);
                                                    this.edit_column_id = column_id.clone();
                                                    this.edit_old_value =
                                                        cell_value_for_closure.clone();
                                                    if let Some(ref input) = this.edit_input {
                                                        window.focus(
                                                            &input.read(cx).focus_handle(cx),
                                                        );
                                                    }
                                                    cx.notify();
                                                },
                                            ),
                                        );
                                    }

                                    if is_editing {
                                        if let Some(ref input_state) = this.edit_input {
                                            cell_div
                                                .child(Input::new(input_state).size(InputSize::Sm))
                                        } else {
                                            cell_div.child(cell_value)
                                        }
                                    } else {
                                        cell_div.child(cell_value)
                                    }
                                });

                        row_div.children(cells)
                    } else {
                        let mut skeleton_row = div()
                            .flex()
                            .w(total_width)
                            .min_w(total_width)
                            .h(row_extent)
                            .bg(if row_idx % 2 == 0 {
                                theme.tokens.background
                            } else {
                                theme.tokens.muted.opacity(0.3)
                            });
                        if this.show_selection {
                            skeleton_row = skeleton_row.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(50.0))
                                    .px(px(16.0))
                                    .py(px(12.0))
                                    .border_b_1()
                                    .border_r_1()
                                    .border_color(theme.tokens.border.opacity(0.5)),
                            );
                        }
                        let cells = this.state.columns.iter().enumerate().map(|(col_idx, _)| {
                            let width = this.state.column_widths[col_idx];
                            div()
                                .flex()
                                .items_center()
                                .px(px(16.0))
                                .py(px(12.0))
                                .w(width)
                                .border_b_1()
                                .border_r_1()
                                .border_color(theme.tokens.border.opacity(0.5))
                                .child(
                                    div()
                                        .w(px(96.0))
                                        .h(px(12.0))
                                        .rounded(px(4.0))
                                        .bg(theme.tokens.muted.opacity(0.6)),
                                )
                        });
                        skeleton_row.children(cells)
                    }
                })
                .collect::<Vec<_>>()
        };

        let view_for_visible = view_entity.clone();
        let view_for_near_end = view_entity.clone();
        let body_scroll = vlist_uniform_view(
            view_entity,
            "data-table-body",
            total_items,
            row_extent,
            renderer,
        )
        .track_scroll(&self.scroll_handle)
        .overscan(8)
        .h(px(viewport_height))
        .on_visible_range(move |range, window, app| {
            let start = range.start;
            let end = range.end;
            let _ = window;
            view_for_visible.update(app, |this: &mut DataTable<T>, cx| {
                let total_items = match &this.state.backing {
                    DataBacking::InMemory { .. } => this.get_filtered_indices().len(),
                    DataBacking::Virtual(_) => this.state.total_items(),
                };
                if total_items > 0 && !this.load_more_triggered {
                    let progress = end as f32 / total_items as f32;
                    if progress >= this.load_more_threshold {
                        if let Some(ref callback) = this.on_load_more {
                            this.load_more_triggered = true;
                            callback(window, cx);
                        }
                    }
                }

                this.fetch_visible_pages(start..end, window, cx);
            });
        })
        .on_near_end(self.load_more_threshold, move |window, app| {
            view_for_near_end.update(app, |this: &mut DataTable<T>, cx| {
                if !this.load_more_triggered {
                    if let Some(ref callback) = this.on_load_more {
                        this.load_more_triggered = true;
                        callback(window, cx);
                    }
                }
            });
        });

        let body_container = div()
            .id("data-table-body-container")
            .debug_selector(|| "data-table-body-container".to_string())
            .h(px(viewport_height))
            .on_scroll_wheel(cx.listener(|_view, _event: &ScrollEvent, _window, cx| {
                cx.stop_propagation();
                cx.notify();
            }))
            .child(body_scroll);

        let scrollable_content = div()
            .id("data-table-content")
            .debug_selector(|| "data-table-content".to_string())
            .flex()
            .flex_col()
            .overflow_x_scroll()
            .w_full()
            .cursor(CursorStyle::PointingHand)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(total_width)
                    .min_w(total_width)
                    .child(self.render_header(cx))
                    .child(body_container),
            );

        let table_div = div()
            .debug_selector(|| "data-table-root".to_string())
            .flex()
            .flex_col()
            .w_full()
            .border_1()
            .border_color(theme.tokens.border)
            .rounded(theme.tokens.radius_lg)
            .overflow_hidden()
            .bg(theme.tokens.card)
            .shadow_sm()
            .when(self.can_show_search(), |div| {
                div.child(self.render_search_bar(cx))
            })
            .child(scrollable_content)
            .map(|mut this| {
                this.style().refine(&user_style);
                this
            });

        let context_menu_elem = self
            .context_menu
            .map(|(row_idx, position)| self.render_context_menu(row_idx, position, cx));

        div()
            .relative()
            .w_full()
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                if let Some(col_idx) = this.resizing_column {
                    let current_x: f32 = event.position.x.into();
                    let delta_x = current_x - this.resize_start_x;
                    let new_width_f32: f32 = this.resize_start_width.into();
                    let new_width = px(new_width_f32 + delta_x);

                    let min_width = this.state.columns[col_idx].min_width;
                    let final_width = if new_width > min_width {
                        new_width
                    } else {
                        min_width
                    };

                    this.state.resize_column(col_idx, final_width);
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    if this.resizing_column.is_some() {
                        this.resizing_column = None;
                        cx.notify();
                    }
                }),
            )
            .child(table_div)
            .children(context_menu_elem)
    }
}

impl<T: Clone + 'static> DataTable<T> {
    fn render_context_menu(
        &self,
        row_idx: usize,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = use_theme(cx);

        deferred(
            anchored()
                .position(position)
                .snap_to_window_with_margin(px(8.))
                .anchor(Corner::TopLeft)
                .child(
                    div()
                        .occlude()
                        .min_w(px(200.0))
                        .bg(theme.tokens.popover)
                        .border_1()
                        .border_color(theme.tokens.border)
                        .rounded(theme.tokens.radius_md)
                        .shadow_xl()
                        .p(px(4.0))
                        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            this.context_menu = None;
                            cx.notify();
                        }))
                        .children(self.row_actions.iter().map(|action| {
                            let action = action.clone();
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .px(px(12.0))
                                .py(px(8.0))
                                .rounded(theme.tokens.radius_sm)
                                .cursor(CursorStyle::PointingHand)
                                .hover(|style| style.bg(theme.tokens.accent))
                                .text_size(px(14.0))
                                .text_color(if action.destructive {
                                    theme.tokens.destructive
                                } else {
                                    theme.tokens.popover_foreground
                                })
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _event, window, cx| {
                                        (action.on_click)(row_idx, window, cx);
                                        this.context_menu = None;
                                        cx.notify();
                                    }),
                                )
                                .when_some(action.icon, |div, icon| {
                                    div.child(
                                        crate::capabilities::primitives::icon::Icon::new(icon)
                                            .size(px(16.0))
                                            .color(if action.destructive {
                                                theme.tokens.destructive
                                            } else {
                                                theme.tokens.popover_foreground
                                            }),
                                    )
                                })
                                .child(action.label)
                                .into_any_element()
                        })),
                ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    #[derive(Clone)]
    struct Row {
        value: String,
    }

    fn columns() -> Vec<ColumnDef<Row>> {
        vec![ColumnDef::new("value", "Value", |row: &Row| {
            row.value.clone().into()
        })]
    }

    #[::core::prelude::v1::test]
    fn numeric_sort_uses_numeric_order_and_clears_selection() {
        let mut state = DataTableState::new(
            vec![
                Row { value: "10".into() },
                Row { value: "2".into() },
                Row { value: "1".into() },
            ],
            columns(),
        );
        state.toggle_row(1);
        state.sort_by_column(0, SortDirection::Ascending);

        let DataBacking::InMemory { data } = &state.backing else {
            panic!("expected in-memory data");
        };
        assert_eq!(
            data.iter()
                .map(|row| row.value.as_str())
                .collect::<Vec<_>>(),
            vec!["1", "2", "10"]
        );
        assert!(state.selected_rows.is_empty());
    }

    #[::core::prelude::v1::test]
    fn replacing_data_reapplies_the_active_sort() {
        let mut state = DataTableState::new(
            vec![Row { value: "2".into() }, Row { value: "1".into() }],
            columns(),
        );
        state.sort_by_column(0, SortDirection::Ascending);
        state.replace_in_memory_data(vec![Row { value: "10".into() }, Row { value: "3".into() }]);

        let DataBacking::InMemory { data } = &state.backing else {
            panic!("expected in-memory data");
        };
        assert_eq!(
            data.iter()
                .map(|row| row.value.as_str())
                .collect::<Vec<_>>(),
            vec!["3", "10"]
        );
    }

    #[::core::prelude::v1::test]
    fn appending_data_reapplies_the_active_sort() {
        let mut state = DataTableState::new(
            vec![Row { value: "1".into() }, Row { value: "3".into() }],
            columns(),
        );
        state.sort_by_column(0, SortDirection::Descending);
        assert!(state.append_in_memory_data(vec![Row { value: "10".into() }]));

        let DataBacking::InMemory { data } = &state.backing else {
            panic!("expected in-memory data");
        };
        assert_eq!(
            data.iter()
                .map(|row| row.value.as_str())
                .collect::<Vec<_>>(),
            vec!["10", "3", "1"]
        );
    }

    #[::core::prelude::v1::test]
    fn virtual_sort_does_not_claim_a_local_order() {
        let mut state = DataTableState::new(Vec::<Row>::new(), columns());
        state.virtual_reset(10, Some(2));
        state.sort_by_column(0, SortDirection::Ascending);

        assert!(state.sort_column.is_none());
    }

    #[gpui::test]
    fn virtual_sort_dispatches_remote_sort_request() {
        let mut app = TestApp::new();
        let requests = Rc::new(RefCell::new(Vec::new()));
        let requests_for_callback = requests.clone();
        let table = app.update(|cx| {
            cx.new(|cx| {
                DataTable::new_virtual(10, columns(), 2, cx).on_sort_change(
                    move |column, direction, _, _| {
                        requests_for_callback
                            .borrow_mut()
                            .push((column.to_string(), direction));
                    },
                )
            })
        });
        let mut window = app.open_window(|_, _| gpui::Empty);

        window.update(|_, window, cx| {
            table.update(cx, |table, cx| {
                table.sort_from_header(0, SortDirection::Descending, window, cx);
            });
        });

        assert_eq!(
            requests.borrow().as_slice(),
            &[("value".to_string(), SortDirection::Descending)]
        );
    }

    #[::core::prelude::v1::test]
    fn virtual_reset_clears_index_based_selection() {
        let mut state = DataTableState::new(Vec::<Row>::new(), columns());
        state.virtual_reset(10, Some(2));
        state.toggle_row(7);
        state.virtual_reset(3, Some(2));
        assert!(state.selected_rows.is_empty());
    }

    #[::core::prelude::v1::test]
    fn virtual_reset_rejects_previous_generation_completion() {
        let mut state = DataTableState::new(Vec::<Row>::new(), columns());
        state.virtual_reset(10, Some(2));
        let DataBacking::Virtual(pages) = &mut state.backing else {
            unreachable!()
        };
        let previous = pages.requests_for_range(0..2, true)[0];
        state.virtual_reset(10, Some(2));
        let DataBacking::Virtual(pages) = &mut state.backing else {
            unreachable!()
        };
        let current = pages.requests_for_range(0..2, true)[0];
        assert!(!pages.complete(
            previous,
            vec![Row {
                value: "query A".into()
            }]
        ));
        assert!(!pages.fail(previous));
        assert!(pages.cache.is_empty());
        assert_eq!(pages.pending.get(&0), Some(&current));
        assert!(pages.complete(
            current,
            vec![Row {
                value: "query B".into()
            }]
        ));
        assert_eq!(state.get_row(0).unwrap().value, "query B");
    }

    #[::core::prelude::v1::test]
    fn virtual_page_requests_deduplicate_and_failed_attempts_can_retry() {
        let mut pages = VirtualPageState::new(5, 2);
        let first = pages.requests_for_range(0..2, true)[0];
        assert!(pages.requests_for_range(0..2, true).is_empty());
        assert!(pages.fail(first));
        let retry = pages.requests_for_range(0..2, true)[0];
        assert_ne!(first, retry);
        assert!(!pages.complete(first, vec!["old"]));
        assert!(!pages.fail(first));
        assert_eq!(pages.pending.get(&0), Some(&retry));
        assert!(pages.complete(retry, vec!["current"]));
        assert!(!pages.complete(retry, vec!["duplicate"]));
        assert_eq!(pages.cache.get(&0), Some(&"current"));
        // A short response completed the page; it is not fetched every render.
        assert!(pages.requests_for_range(0..2, true).is_empty());
    }

    #[::core::prelude::v1::test]
    fn virtual_page_completion_respects_bounds_and_empty_success() {
        let mut pages = VirtualPageState::new(3, 2);
        assert!(pages.requests_for_range(0..0, true).is_empty());
        assert!(pages.requests_for_range(3..10, true).is_empty());
        let requests = pages.requests_for_range(0..10, true);
        assert_eq!(requests.len(), 2);
        assert!(pages.complete(requests[0], vec![0, 1, 99]));
        assert!(pages.complete(requests[1], Vec::new()));
        assert_eq!(pages.cache.len(), 2);
        assert!(!pages.cache.contains_key(&2));
        assert!(pages.requests_for_range(0..3, true).is_empty());
    }

    #[::core::prelude::v1::test]
    fn virtual_request_tokens_are_scoped_to_their_table_instance() {
        let mut first = VirtualPageState::<&str>::new(2, 2);
        let mut second = VirtualPageState::new(2, 2);
        let first_request = first.requests_for_range(0..2, true)[0];
        let second_request = second.requests_for_range(0..2, true)[0];
        assert!(!second.complete(first_request, vec!["wrong table"]));
        assert!(!second.fail(first_request));
        assert!(second.complete(second_request, vec!["right table"]));
    }

    #[::core::prelude::v1::test]
    fn switching_loaders_invalidates_old_pending_requests() {
        let mut state = DataTableState::new(Vec::<Row>::new(), columns());
        state.virtual_reset(10, Some(2));
        let DataBacking::Virtual(pages) = &mut state.backing else {
            unreachable!()
        };
        assert_eq!(pages.requests_for_range(0..2, false).len(), 1);
        state.set_tokenized_page_requests(true);
        let DataBacking::Virtual(pages) = &mut state.backing else {
            unreachable!()
        };
        let token = pages.requests_for_range(0..2, true)[0];
        assert!(pages.legacy_pending.is_empty());
        state.set_tokenized_page_requests(false);
        let DataBacking::Virtual(pages) = &mut state.backing else {
            unreachable!()
        };
        assert!(!pages.complete(token, Vec::new()));
        assert_eq!(pages.requests_for_range(0..2, false).len(), 1);
    }

    #[::core::prelude::v1::test]
    fn legacy_page_writes_cannot_acknowledge_tokenized_requests() {
        let mut state = DataTableState::new(Vec::<Row>::new(), columns());
        state.tokenized_page_requests = true;
        state.virtual_reset(10, Some(2));
        let DataBacking::Virtual(pages) = &mut state.backing else {
            unreachable!()
        };
        let request = pages.requests_for_range(0..2, true)[0];
        assert!(!state.virtual_set_page(
            0,
            vec![Row {
                value: "legacy".into()
            }]
        ));
        let DataBacking::Virtual(pages) = &state.backing else {
            unreachable!()
        };
        assert!(pages.cache.is_empty());
        assert_eq!(pages.pending.get(&0), Some(&request));
    }

    #[::core::prelude::v1::test]
    fn virtual_generation_survives_switching_to_in_memory_and_back() {
        let mut state = DataTableState::new(Vec::<Row>::new(), columns());
        state.virtual_reset(10, Some(2));
        let DataBacking::Virtual(pages) = &mut state.backing else {
            unreachable!()
        };
        let previous = pages.requests_for_range(0..2, true)[0];
        state.replace_in_memory_data(Vec::new());
        state.virtual_reset(10, Some(2));
        let DataBacking::Virtual(pages) = &mut state.backing else {
            unreachable!()
        };
        let current = pages.requests_for_range(0..2, true)[0];
        assert_ne!(previous, current);
        assert!(!pages.complete(previous, Vec::new()));
        assert!(pages.complete(current, Vec::new()));
    }

    #[::core::prelude::v1::test]
    fn custom_table_comparator_is_independent_of_display_and_reapplies_after_changes() {
        #[derive(Clone)]
        struct Price(u32);
        let mut state = DataTableState::new(
            vec![Price(200), Price(1000)],
            vec![ColumnDef::new("price", "Price", |row: &Price| {
                format!("${}", row.0).into()
            })],
        );
        assert!(state.set_column_comparator("price", |a, b| a.0.cmp(&b.0)));
        assert!(!state.set_column_comparator("missing", |a, b| a.0.cmp(&b.0)));
        state.sort_by_column(0, SortDirection::Ascending);
        assert_eq!(state.get_row(0).unwrap().0, 200);
        state.replace_in_memory_data(vec![Price(1000), Price(300)]);
        assert_eq!(state.get_row(0).unwrap().0, 300);
        assert!(state.append_in_memory_data(vec![Price(100)]));
        assert_eq!(state.get_row(0).unwrap().0, 100);
        state.sort_by_column(0, SortDirection::Descending);
        assert_eq!(state.get_row(0).unwrap().0, 1000);
        assert_eq!(
            (state.columns[0].accessor)(state.get_row(0).unwrap()).as_ref(),
            "$1000"
        );
    }

    #[gpui::test]
    fn tokenized_loader_dispatches_and_public_completion_api_rejects_stale_results() {
        let mut app = TestApp::new();
        let requests = Rc::new(RefCell::new(Vec::new()));
        let collected = requests.clone();
        let table = app.update(|cx| {
            cx.new(|cx| {
                DataTable::new_virtual(10, columns(), 2, cx).on_fetch_page_request(
                    move |request, _, _| collected.borrow_mut().push(request),
                )
            })
        });
        let mut window = app.open_window(|_, _| gpui::Empty);
        window.update(|_, window, cx| {
            table.update(cx, |table, cx| {
                table.fetch_visible_pages(0..2, window, cx);
                table.fetch_visible_pages(0..2, window, cx);
                assert_eq!(requests.borrow().len(), 1);
                let old = requests.borrow()[0];
                table.virtual_reset(10, None, cx);
                table.fetch_visible_pages(0..2, window, cx);
                let current = requests.borrow()[1];
                table.set_page_data(
                    0,
                    vec![Row {
                        value: "legacy".into(),
                    }],
                    cx,
                );
                assert!(!table.fail_page(0, cx));
                assert!(!table.complete_page_request(
                    old,
                    vec![Row {
                        value: "old".into()
                    }],
                    cx
                ));
                assert!(!table.fail_page_request(old, cx));
                assert!(table.fail_page_request(current, cx));
                table.fetch_visible_pages(0..2, window, cx);
                let retry = requests.borrow()[2];
                assert!(table.complete_page_request(
                    retry,
                    vec![Row {
                        value: "current".into()
                    }],
                    cx
                ));
                assert_eq!(table.state.get_row(0).unwrap().value, "current");
            })
        });
    }

    #[gpui::test]
    fn legacy_loader_keeps_its_callback_and_completion_contract_and_can_retry() {
        let mut app = TestApp::new();
        let requests = Rc::new(RefCell::new(Vec::new()));
        let collected = requests.clone();
        let table = app.update(|cx| {
            cx.new(|cx| {
                DataTable::new_virtual(1, columns(), 2, cx).on_fetch_page(
                    move |start, size, _, _| collected.borrow_mut().push((start, size)),
                )
            })
        });
        let mut window = app.open_window(|_, _| gpui::Empty);
        window.update(|_, window, cx| {
            table.update(cx, |table, cx| {
                table.fetch_visible_pages(0..1, window, cx);
                table.fetch_visible_pages(0..1, window, cx);
                assert_eq!(requests.borrow().as_slice(), &[(0, 2)]);
                assert!(table.fail_page(0, cx));
                table.fetch_visible_pages(0..1, window, cx);
                assert_eq!(requests.borrow().as_slice(), &[(0, 2), (0, 2)]);
                table.set_page_data(
                    0,
                    vec![Row {
                        value: "legacy".into(),
                    }],
                    cx,
                );
                table.fetch_visible_pages(0..1, window, cx);
                assert_eq!(requests.borrow().len(), 2);
                assert_eq!(table.state.get_row(0).unwrap().value, "legacy");
            })
        });
    }

    #[gpui::test]
    fn virtual_search_requires_a_remote_handler_and_dispatches_query_and_column() {
        let mut app = TestApp::new();
        let changes = Rc::new(RefCell::new(Vec::new()));
        let collected = changes.clone();
        let table = app.update(|cx| {
            cx.new(|cx| {
                let table = DataTable::new_virtual(10, columns(), 2, cx).show_search(true);
                assert!(!table.can_show_search());
                table.on_search_change(move |query, column, _| {
                    collected.borrow_mut().push((query, column))
                })
            })
        });
        app.update(|cx| {
            table.update(cx, |table, cx| {
                assert!(table.can_show_search());
                table.set_search("needle".into(), cx);
                table.set_search_column(Some(0), cx);
            })
        });
        let changes = changes.borrow();
        assert_eq!(changes[0], ("needle".to_string(), None));
        assert_eq!(changes[1], ("needle".to_string(), Some("value".into())));
    }

    #[gpui::test]
    fn window_aware_data_reset_notifies_selection_observers() {
        let mut app = TestApp::new();
        let calls = Rc::new(Cell::new(0));
        let calls_for_callback = calls.clone();
        let table = app.update(|cx| {
            cx.new(|cx| {
                DataTable::new(
                    vec![Row {
                        value: "one".into(),
                    }],
                    columns(),
                    cx,
                )
                .on_selection_change(move |selected, _, _| {
                    if selected.is_empty() {
                        calls_for_callback.set(calls_for_callback.get() + 1);
                    }
                })
            })
        });
        let mut window = app.open_window(|_, _| gpui::Empty);

        window.update(|_, window, cx| {
            table.update(cx, |table, _cx| table.state.toggle_row(0));
            table.update(cx, |table, cx| {
                table.set_data_with_window(
                    vec![Row {
                        value: "two".into(),
                    }],
                    window,
                    cx,
                );
            });
        });

        assert_eq!(calls.get(), 1);
    }

    #[gpui::test]
    fn comparator_changes_clear_selection_only_when_rows_reorder() {
        let mut app = TestApp::new();
        let calls = Rc::new(Cell::new(0));
        let calls_for_callback = calls.clone();
        let rows = ["b", "a"].map(|value| Row {
            value: value.into(),
        });
        let table = app.update(|cx| {
            cx.new(|cx| {
                DataTable::new(rows.to_vec(), columns(), cx).on_selection_change(move |_, _, _| {
                    calls_for_callback.set(calls_for_callback.get() + 1)
                })
            })
        });
        let mut window = app.open_window(|_, _| gpui::Empty);
        window.update(|_, window, cx| {
            table.update(cx, |table, cx| {
                table.state.toggle_row(0);
                assert!(table.set_column_comparator(
                    "value",
                    |a: &Row, b: &Row| a.value.cmp(&b.value),
                    window,
                    cx
                ));
                assert_eq!(table.state.selected_rows, [0], "unsorted rows keep order");

                table.state.sort_by_column(0, SortDirection::Ascending);
                assert_eq!(table.state.get_row(0).unwrap().value, "a");
                table.state.toggle_row(1);
                assert!(table.set_column_comparator(
                    "value",
                    |a: &Row, b: &Row| b.value.cmp(&a.value),
                    window,
                    cx
                ));
                assert_eq!(table.state.get_row(0).unwrap().value, "b");
                assert!(table.state.selected_rows.is_empty());

                assert!(!table.set_column_comparator(
                    "missing",
                    |a: &Row, b: &Row| a.value.cmp(&b.value),
                    window,
                    cx
                ));
            });
        });
        assert_eq!(calls.get(), 1);
    }

    #[gpui::test]
    fn search_change_handler_can_defer_a_virtual_reset() {
        let mut app = TestApp::new();
        let requests = Rc::new(RefCell::new(Vec::new()));
        let collected = requests.clone();
        let table = app.update(|cx| {
            cx.new(|cx| {
                DataTable::new_virtual(10, columns(), 2, cx)
                    .on_fetch_page_request(move |request, _, _| {
                        collected.borrow_mut().push(request)
                    })
                    .on_search_change(|_, _, cx| {
                        let table = cx.entity().downgrade();
                        cx.defer(move |cx| {
                            if let Some(table) = table.upgrade() {
                                table.update(cx, |table, cx| table.virtual_reset(4, None, cx));
                            }
                        });
                    })
            })
        });
        let mut window = app.open_window(|_, _| gpui::Empty);
        window.update(|_, window, cx| {
            table.update(cx, |table, cx| {
                table.fetch_visible_pages(0..2, window, cx);
                table.set_search("needle".into(), cx);
            });
        });
        window.update(|_, window, cx| {
            table.update(cx, |table, cx| {
                assert_eq!(table.state.total_items(), 4);
                let stale = requests.borrow()[0];
                assert!(!table.complete_page_request(
                    stale,
                    vec![Row {
                        value: "previous query".into()
                    }],
                    cx
                ));
                table.fetch_visible_pages(0..2, window, cx);
                assert_eq!(requests.borrow().len(), 2);
            });
        });
    }
}
