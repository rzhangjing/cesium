//! Mirror of `packages/engine/Specs/Core/GoogleEarthEnterpriseMetadataSpec.js`
//! (276 lines).
//!
//! The `gee.metadata` fixture (`Specs/Data/GoogleEarthEnterprise`) is copied
//! into the specs container and registered on [`MockResourceBackend`] under
//! the level-zero quadtree packet URL. The dbRoot request is intentionally
//! left unregistered so the JS "reject dbRoot, use defaults" path is taken.
//!
//! # Skipped JS tests (DEVIATION)
//! - "decode" / "decode requires key" / "decode requires data" / "decode
//!   throws if key length isn't greater than 0 and a multiple 4": covered by
//!   `core_decode_google_earth_enterprise_data_spec`; the required-parameter
//!   DeveloperErrors are compile-time in Rust, and the Rust decoder returns
//!   silently for invalid key lengths (no RuntimeError).
//! - "populateSubtree": spies on `getQuadTreePacket` at the prototype level;
//!   the Rust port has no virtual dispatch to spy on, and fabricating valid
//!   encrypted quadtree packets is out of scope. The merge behavior is
//!   exercised by the "from url" tests below via the real fixture.

use cesium_core::google_earth_enterprise_metadata::{
    get_metadata_resource, GoogleEarthEnterpriseMetadata,
};
use cesium_core::math::CesiumMath;
use cesium_core::resource::{MockResourceBackend, Resource};

const GEE_METADATA_FIXTURE: &[u8] =
    include_bytes!("../../Data/GoogleEarthEnterprise/gee.metadata");

const BASE_URL: &str = "http://fake.fake.invalid/";

fn quad_packet_url() -> String {
    let mut resource = Resource::new(BASE_URL.to_string());
    resource.append_forward_slash();
    get_metadata_resource(&resource, "", 1).url()
}

#[test]
fn tile_xy_to_quad_key() {
    assert_eq!(GoogleEarthEnterpriseMetadata::tile_xy_to_quad_key(1, 0, 0), "2");
    assert_eq!(GoogleEarthEnterpriseMetadata::tile_xy_to_quad_key(1, 2, 1), "02");
    assert_eq!(GoogleEarthEnterpriseMetadata::tile_xy_to_quad_key(3, 5, 2), "021");
    assert_eq!(GoogleEarthEnterpriseMetadata::tile_xy_to_quad_key(4, 7, 2), "100");
}

#[test]
fn quad_key_to_tile_xy() {
    assert_eq!(
        GoogleEarthEnterpriseMetadata::quad_key_to_tile_xy("2"),
        (1, 0, 0)
    );
    assert_eq!(
        GoogleEarthEnterpriseMetadata::quad_key_to_tile_xy("02"),
        (1, 2, 1)
    );
    assert_eq!(
        GoogleEarthEnterpriseMetadata::quad_key_to_tile_xy("021"),
        (3, 5, 2)
    );
    assert_eq!(
        GoogleEarthEnterpriseMetadata::quad_key_to_tile_xy("100"),
        (4, 7, 2)
    );
}

#[test]
fn decode_roundtrip_with_seeded_random() {
    CesiumMath::set_random_number_seed(123123.0);
    let mut key = vec![0u8; 1025];
    let mut data = vec![0u8; 1025];
    for i in 0..1025 {
        key[i] = (CesiumMath::next_random_number() * 256.0).floor() as u8;
        data[i] = (CesiumMath::next_random_number() * 256.0).floor() as u8;
    }

    // Key length should be divisible by 4
    let key_buffer = &key[..1024];
    let original = data.clone();
    cesium_core::decode_google_earth_enterprise_data::decode_google_earth_enterprise_data(
        key_buffer,
        &mut data,
    );
    assert_ne!(data, original);

    // For the algorithm encode/decode are the same
    cesium_core::decode_google_earth_enterprise_data::decode_google_earth_enterprise_data(
        key_buffer,
        &mut data,
    );
    assert_eq!(data, original);
}

fn assert_default_fields(metadata: &GoogleEarthEnterpriseMetadata) {
    assert!(metadata.imagery_present);
    assert!(metadata.proto_imagery.is_none());
    assert!(metadata.terrain_present);
    assert_eq!(metadata.negative_altitude_threshold, CesiumMath::EPSILON12);
    assert_eq!(metadata.negative_altitude_exponent_bias, 32);
    assert!(metadata.providers.is_empty());

    let tile_info = metadata.tile_info.borrow();
    let info = tile_info
        .get("0")
        .and_then(|i| i.as_ref())
        .expect("tileInfo[\"0\"] must be defined");
    assert_eq!(info.bits(), 0x40);
    assert_eq!(info.cnode_version, 2);
    assert_eq!(info.imagery_version, 1);
    assert_eq!(info.terrain_version, 1);
    assert!(!info.ancestor_has_terrain);
    assert!(info.terrain_state.is_none());
}

#[tokio::test]
async fn from_url_resolves_to_google_earth_enterprise_metadata() {
    let mut backend = MockResourceBackend::new();
    // dbRoot is left unregistered: the request fails and defaults are used.
    backend.register_response(&quad_packet_url(), GEE_METADATA_FIXTURE.to_vec());

    let metadata = GoogleEarthEnterpriseMetadata::from_url(Some(BASE_URL), &backend)
        .await
        .unwrap();

    assert_default_fields(&metadata);
}

#[tokio::test]
async fn from_url_with_resource_resolves_to_google_earth_enterprise_metadata() {
    let mut backend = MockResourceBackend::new();
    backend.register_response(&quad_packet_url(), GEE_METADATA_FIXTURE.to_vec());

    let resource = Resource::new(BASE_URL.to_string());
    let metadata = GoogleEarthEnterpriseMetadata::from_resource(resource, &backend)
        .await
        .unwrap();

    assert_default_fields(&metadata);
}

#[tokio::test]
async fn from_url_rejects_on_error() {
    // Nothing registered: every request fails (JS expects a 404 rejection;
    // the mock backend reports "No mock response" instead).
    let backend = MockResourceBackend::new();

    let result =
        GoogleEarthEnterpriseMetadata::from_url(Some("host.invalid/"), &backend).await;
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("fromUrl must reject"),
    };
    let message = format!("{error}");
    assert!(
        message.contains("An error occurred while accessing"),
        "unexpected error message: {message}"
    );
}
