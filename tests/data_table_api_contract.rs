use fc_ui::display::data_table::{ColumnDef, DataTable, DataTableState, VirtualPageRequest};
use gpui::{px, Context};
use std::rc::Rc;

#[test]
fn column_literals_and_request_completion_apis_remain_accessible() {
    let column = ColumnDef {
        id: "value".into(),
        header: "Value".into(),
        accessor: Rc::new(|row: &i32| row.to_string().into()),
        width: px(150.),
        min_width: px(80.),
        resizable: true,
        sortable: true,
        editable: false,
    };
    let mut state = DataTableState::new(vec![2, 1], vec![column]);
    assert!(state.set_column_comparator("value", i32::cmp));

    type CompletePage =
        fn(&mut DataTable<i32>, VirtualPageRequest, Vec<i32>, &mut Context<DataTable<i32>>) -> bool;
    let complete: CompletePage = DataTable::complete_page_request;
    let fail: fn(&mut DataTable<i32>, VirtualPageRequest, &mut Context<DataTable<i32>>) -> bool =
        DataTable::fail_page_request;
    let _ = (complete, fail);
}
