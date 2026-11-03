use super::*;
use gpui::TestAppContext;
use serde_json::json;
fn locations() -> Locations {
    Locations {
        cargo: "/cargo".into(),
        rustup: "/rustup".into(),
        go_roots: vec!["/go-sdk".into()],
        go_modules: "/gomod".into(),
    }
}
#[gpui::test]
async fn discovers_only_installed_manifest_dependencies_and_sdk_sources(cx: &mut TestAppContext) {
    let fs = fs::FakeFs::new(cx.executor());
    fs.insert_tree("/project", json!({
        "package.json":"{}", "node_modules":{"module":{"index.js":""}},
        "go.mod":"module example.test/app", "go.sum":"example.test/SDK v1.0.0 hash\nexample.test/SDK v1.0.0/go.mod hash\nmissing.test/pkg v9.0.0 hash\n",
        "Cargo.toml":"[package]\nname='app'\nversion='0.1.0'", "Cargo.lock":"[[package]]\nname='dep'\nversion='1.0.0'\nsource='registry+https://github.com/rust-lang/crates.io-index'\n", "rust-toolchain.toml":"[toolchain]\nchannel='stable'",
        "pyproject.toml":"", ".venv":{"lib":{"python3.13":{"site-packages":{"pkg":{"__init__.py":""}}}}}
    })).await;
    fs.insert_tree("/cargo", json!({"registry":{"src":{"index.crates.io-test":{"dep-1.0.0":{"lib.rs":""},"unrelated-2.0.0":{}}}}})).await;
    fs.insert_tree("/rustup", json!({"toolchains":{"stable-test-target":{"lib":{"rustlib":{"src":{"rust":{"library":{"core":{}}}}}}}}})).await;
    fs.insert_tree("/go-sdk", json!({"src":{"fmt":{"print.go":""}}}))
        .await;
    fs.insert_tree(
        "/gomod",
        json!({"example.test":{"!s!d!k@v1.0.0":{"api.go":""}}}),
    )
    .await;
    let found = discover(fs, &[PathBuf::from("/project")], &locations())
        .await
        .unwrap();
    for path in [
        "/project/node_modules",
        "/project/.venv/lib/python3.13",
        "/go-sdk/src",
        "/gomod/example.test/!s!d!k@v1.0.0",
        "/cargo/registry/src/index.crates.io-test/dep-1.0.0",
        "/rustup/toolchains/stable-test-target/lib/rustlib/src/rust/library",
    ] {
        assert!(found.contains_key(Path::new(path)), "{path}");
    }
    assert_eq!(found.len(), 6);
}
#[gpui::test]
async fn missing_installed_sources_are_not_invented(cx: &mut TestAppContext) {
    let fs = fs::FakeFs::new(cx.executor());
    fs.insert_tree(
        "/project",
        json!({"package.json":"{}", "go.mod":"module app", "pyproject.toml":""}),
    )
    .await;
    assert!(
        discover(fs, &[PathBuf::from("/project")], &locations())
            .await
            .unwrap()
            .is_empty()
    );
}
#[test]
fn manifest_cache_paths_cannot_escape_the_cache() {
    for path in [
        "../../outside@v1",
        "/absolute@v1",
        "path\\..\\outside@v1",
        "path//outside@v1",
    ] {
        assert!(RepoRelative::validate(path).is_err());
    }
    assert_eq!(go_cache_name("example.test/SDK"), "example.test/!s!d!k");
}
