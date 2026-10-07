#![cfg(feature = "http")]

use fc_ui::{HttpSetup, InitError};
use gpui::{
    http_client::{FakeHttpClient, HttpClient},
    TestApp,
};
use std::{error::Error as _, sync::Arc};

#[gpui::test]
fn explicit_default_http_uses_package_version_in_user_agent() {
    let mut app = TestApp::new();

    app.update(|cx| fc_ui::try_init_with(cx, HttpSetup::Default))
        .unwrap();

    app.read(|cx| {
        assert_eq!(
            cx.http_client().user_agent().unwrap().to_str().unwrap(),
            concat!("fc-ui/", env!("CARGO_PKG_VERSION"))
        );
    });
}

#[gpui::test]
fn explicit_http_accepts_a_custom_user_agent() {
    let mut app = TestApp::new();

    app.update(|cx| fc_ui::try_init_with(cx, HttpSetup::UserAgent("example-app/1.2.3".into())))
        .unwrap();

    app.read(|cx| {
        assert_eq!(
            cx.http_client().user_agent().unwrap().to_str().unwrap(),
            "example-app/1.2.3"
        );
    });
}

#[gpui::test]
fn root_init_preserves_the_callers_http_client_by_default() {
    let mut app = TestApp::new();
    let existing: Arc<dyn HttpClient> = FakeHttpClient::with_404_response();

    app.update(|cx| {
        cx.set_http_client(existing.clone());
        fc_ui::init(cx);
    });

    app.read(|cx| assert!(Arc::ptr_eq(&cx.http_client(), &existing)));
}

#[gpui::test]
fn explicit_preserve_keeps_the_callers_http_client() {
    let mut app = TestApp::new();
    let existing: Arc<dyn HttpClient> = FakeHttpClient::with_404_response();

    app.update(|cx| {
        cx.set_http_client(existing.clone());
        fc_ui::try_init_with(cx, HttpSetup::PreserveExisting)
    })
    .unwrap();

    app.read(|cx| assert!(Arc::ptr_eq(&cx.http_client(), &existing)));
}

#[gpui::test]
fn a_second_explicit_root_initialization_returns_an_error() {
    let mut app = TestApp::new();
    let existing: Arc<dyn HttpClient> = FakeHttpClient::with_404_response();

    app.update(|cx| {
        cx.set_http_client(existing.clone());
        fc_ui::init(cx);
    });

    let result =
        app.update(|cx| fc_ui::try_init_with(cx, HttpSetup::UserAgent("ignored/1.0".into())));

    assert!(matches!(result, Err(InitError::AlreadyInitialized)));
    app.read(|cx| assert!(Arc::ptr_eq(&cx.http_client(), &existing)));
}

#[gpui::test]
fn invalid_user_agent_returns_library_error_without_modifying_the_app() {
    let mut app = TestApp::new();
    let existing: Arc<dyn HttpClient> = FakeHttpClient::with_404_response();

    let result = app.update(|cx| {
        cx.set_http_client(existing.clone());
        fc_ui::try_init_with(cx, HttpSetup::UserAgent("invalid\nuser-agent".into()))
    });

    let error = result.unwrap_err();
    assert!(matches!(error, InitError::Http(_)));
    assert!(error.source().is_some());
    app.read(|cx| assert!(Arc::ptr_eq(&cx.http_client(), &existing)));

    app.update(|cx| fc_ui::try_init_with(cx, HttpSetup::PreserveExisting))
        .unwrap();
    app.read(|cx| assert!(Arc::ptr_eq(&cx.http_client(), &existing)));
}
