//! Smoke test: the spec data root must resolve to an existing directory in
//! the monorepo layout (`<workspace>/../Specs/Data`).

#[test]
fn specs_data_root_resolves_to_existing_directory() {
    let root = cesium_specs::specs_data_root();
    assert!(
        root.is_dir(),
        "specs data root does not exist: {} (set CESIUM_SPECS_DATA to override)",
        root.display()
    );
}

#[test]
fn data_path_joins_relative_asset() {
    let path = cesium_specs::data_path("test.geojson");
    let root = cesium_specs::specs_data_root();
    assert_eq!(path.parent().unwrap(), root.as_path());
    // The CesiumJS repo ships this asset in Specs/Data.
    assert!(
        path.is_file(),
        "expected spec asset to exist: {}",
        path.display()
    );
}
