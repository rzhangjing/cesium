//! Ported from `packages/engine/Source/Core/GoogleEarthEnterpriseMetadata.js`.
//!
//! # Alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `stringToBuffer` | folded into [`default_key`] | identical bytes |
//! | `defaultKey` | [`default_key`] | identical 1024-byte key |
//! | `GoogleEarthEnterpriseMetadata` constructor | [`GoogleEarthEnterpriseMetadata::new`] | identical defaults |
//! | `url` / `proxy` / `resource` properties | `resource` field + [`GoogleEarthEnterpriseMetadata::url`] | proxy not modeled (browser-only) |
//! | `fromUrl` | [`GoogleEarthEnterpriseMetadata::from_url`] / [`GoogleEarthEnterpriseMetadata::from_resource`] | DEVIATION 1 |
//! | `tileXYToQuadKey` | [`GoogleEarthEnterpriseMetadata::tile_xy_to_quad_key`] | identical |
//! | `quadKeyToTileXY` | [`GoogleEarthEnterpriseMetadata::quad_key_to_tile_xy`] | identical |
//! | `isValid` | [`GoogleEarthEnterpriseMetadata::is_valid`] | identical |
//! | `getQuadTreePacket` | [`GoogleEarthEnterpriseMetadata::get_quad_tree_packet`] | TaskProcessor decode inlined (DEVIATION 2) |
//! | `populateSubtree` (both overloads) | [`GoogleEarthEnterpriseMetadata::populate_subtree`] / [`GoogleEarthEnterpriseMetadata::populate_subtree_xy`] | DEVIATION 3 |
//! | `getTileInformation` | [`GoogleEarthEnterpriseMetadata::get_tile_information`] | |
//! | `getTileInformationFromQuadKey` | [`GoogleEarthEnterpriseMetadata::get_tile_information_from_quad_key`] | |
//! | `getMetadataResource` | [`get_metadata_resource`] | identical URL |
//! | `requestDbRoot` | [`request_db_root`] | DEVIATION 4 |
//!
//! # DEVIATIONS
//!
//! 1. HTTP access goes through the injected [`ResourceBackend`] instead of
//!    XHR; `Request` throttling objects are not modeled (`undefined`
//!    "throttled" results map to `Ok(None)` from the backend).
//! 2. The JS `TaskProcessor("decodeGoogleEarthEnterprisePacket")` worker is
//!    invoked synchronously in-process via
//!    [`crate::decode_google_earth_enterprise_packet`].
//! 3. `populateSubtree` awaits subtree requests inline; the JS
//!    `_subtreePromises` in-flight deduplication map is unnecessary for the
//!    sequential await model. The `Request` parameter is dropped.
//! 4. `requestDbRoot` parsing requires the browser-only protobuf dbroot
//!    parser script (`loadAndExecuteScript` + `window.cesiumGoogleEarthDbRootParser`).
//!    The Rust port always takes the JS `catch` fallback path: the dbRoot
//!    resource is still requested (errors eaten), then defaults are used
//!    (`key = defaultKey`), logging `Failed to retrieve {url}. Using defaults.`.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::check;
use crate::credit::Credit;
use crate::decode_google_earth_enterprise_packet::{
    decode_google_earth_enterprise_packet, GeePacketResult, GeePacketType,
};
use crate::google_earth_enterprise_tile_information::GoogleEarthEnterpriseTileInformation;
use crate::is_bit_set::is_bit_set;
use crate::math::CesiumMath;
use crate::resource::{DerivedResourceOptions, Resource, ResourceBackend};
use crate::runtime_error::RuntimeError;

fn string_to_buffer(str_: &str) -> Vec<u8> {
    str_.chars().map(|c| c as u8).collect()
}

/// Decodes packet with a key that has been around since the beginning of
/// Google Earth Enterprise. Mirrors the module-level `defaultKey`.
pub fn default_key() -> Vec<u8> {
    string_to_buffer(
        "\u{45}\u{f4}\u{bd}\u{0b}\u{79}\u{e2}\u{6a}\u{45}\u{22}\u{05}\u{92}\u{2c}\u{17}\u{cd}\u{06}\u{71}\u{f8}\u{49}\u{10}\u{46}\u{67}\u{51}\u{00}\u{42}\u{25}\u{c6}\u{e8}\u{61}\u{2c}\u{66}\u{29}\u{08}\u{c6}\u{34}\u{dc}\u{6a}\u{62}\u{25}\u{79}\u{0a}\u{77}\u{1d}\u{6d}\u{69}\u{d6}\u{f0}\u{9c}\u{6b}\u{93}\u{a1}\u{bd}\u{4e}\u{75}\u{e0}\u{41}\u{04}\u{5b}\u{df}\u{40}\u{56}\u{0c}\u{d9}\u{bb}\u{72}\u{9b}\u{81}\u{7c}\u{10}\u{33}\u{53}\u{ee}\u{4f}\u{6c}\u{d4}\u{71}\u{05}\u{b0}\u{7b}\u{c0}\u{7f}\u{45}\u{03}\u{56}\u{5a}\u{ad}\u{77}\u{55}\u{65}\u{0b}\u{33}\u{92}\u{2a}\u{ac}\u{19}\u{6c}\u{35}\u{14}\u{c5}\u{1d}\u{30}\u{73}\u{f8}\u{33}\u{3e}\u{6d}\u{46}\u{38}\u{4a}\u{b4}\u{dd}\u{f0}\u{2e}\u{dd}\u{17}\u{75}\u{16}\u{da}\u{8c}\u{44}\u{74}\u{22}\u{06}\u{fa}\u{61}\u{22}\u{0c}\u{33}\u{22}\u{53}\u{6f}\u{af}\u{39}\u{44}\u{0b}\u{8c}\u{0e}\u{39}\u{d9}\u{39}\u{13}\u{4c}\u{b9}\u{bf}\u{7f}\u{ab}\u{5c}\u{8c}\u{50}\u{5f}\u{9f}\u{22}\u{75}\u{78}\u{1f}\u{e9}\u{07}\u{71}\u{91}\u{68}\u{3b}\u{c1}\u{c4}\u{9b}\u{7f}\u{f0}\u{3c}\u{56}\u{71}\u{48}\u{82}\u{05}\u{27}\u{55}\u{66}\u{59}\u{4e}\u{65}\u{1d}\u{98}\u{75}\u{a3}\u{61}\u{46}\u{7d}\u{61}\u{3f}\u{15}\u{41}\u{00}\u{9f}\u{14}\u{06}\u{d7}\u{b4}\u{34}\u{4d}\u{ce}\u{13}\u{87}\u{46}\u{b0}\u{1a}\u{d5}\u{05}\u{1c}\u{b8}\u{8a}\u{27}\u{7b}\u{8b}\u{dc}\u{2b}\u{bb}\u{4d}\u{67}\u{30}\u{c8}\u{d1}\u{f6}\u{5c}\u{8f}\u{50}\u{fa}\u{5b}\u{2f}\u{46}\u{9b}\u{6e}\u{35}\u{18}\u{2f}\u{27}\u{43}\u{2e}\u{eb}\u{0a}\u{0c}\u{5e}\u{10}\u{05}\u{10}\u{a5}\u{73}\u{1b}\u{65}\u{34}\u{e5}\u{6c}\u{2e}\u{6a}\u{43}\u{27}\u{63}\u{14}\u{23}\u{55}\u{a9}\u{3f}\u{71}\u{7b}\u{67}\u{43}\u{7d}\u{3a}\u{af}\u{cd}\u{e2}\u{54}\u{55}\u{9c}\u{fd}\u{4b}\u{c6}\u{e2}\u{9f}\u{2f}\u{28}\u{ed}\u{cb}\u{5c}\u{c6}\u{2d}\u{66}\u{07}\u{88}\u{a7}\u{3b}\u{2f}\u{18}\u{2a}\u{22}\u{4e}\u{0e}\u{b0}\u{6b}\u{2e}\u{dd}\u{0d}\u{95}\u{7d}\u{7d}\u{47}\u{ba}\u{43}\u{b2}\u{11}\u{b2}\u{2b}\u{3e}\u{4d}\u{aa}\u{3e}\u{7d}\u{e6}\u{ce}\u{49}\u{89}\u{c6}\u{e6}\u{78}\u{0c}\u{61}\u{31}\u{05}\u{2d}\u{01}\u{a4}\u{4f}\u{a5}\u{7e}\u{71}\u{20}\u{88}\u{ec}\u{0d}\u{31}\u{e8}\u{4e}\u{0b}\u{00}\u{6e}\u{50}\u{68}\u{7d}\u{17}\u{3d}\u{08}\u{0d}\u{17}\u{95}\u{a6}\u{6e}\u{a3}\u{68}\u{97}\u{24}\u{5b}\u{6b}\u{f3}\u{17}\u{23}\u{f3}\u{b6}\u{73}\u{b3}\u{0d}\u{0b}\u{40}\u{c0}\u{9f}\u{d8}\u{04}\u{51}\u{5d}\u{fa}\u{1a}\u{17}\u{22}\u{2e}\u{15}\u{6a}\u{df}\u{49}\u{00}\u{b9}\u{a0}\u{77}\u{55}\u{c6}\u{ef}\u{10}\u{6a}\u{bf}\u{7b}\u{47}\u{4c}\u{7f}\u{83}\u{17}\u{05}\u{ee}\u{dc}\u{dc}\u{46}\u{85}\u{a9}\u{ad}\u{53}\u{07}\u{2b}\u{53}\u{34}\u{06}\u{07}\u{ff}\u{14}\u{94}\u{59}\u{19}\u{02}\u{e4}\u{38}\u{e8}\u{31}\u{83}\u{4e}\u{b9}\u{58}\u{46}\u{6b}\u{cb}\u{2d}\u{23}\u{86}\u{92}\u{70}\u{00}\u{35}\u{88}\u{22}\u{cf}\u{31}\u{b2}\u{26}\u{2f}\u{e7}\u{c3}\u{75}\u{2d}\u{36}\u{2c}\u{72}\u{74}\u{b0}\u{23}\u{47}\u{b7}\u{d3}\u{d1}\u{26}\u{16}\u{85}\u{37}\u{72}\u{e2}\u{00}\u{8c}\u{44}\u{cf}\u{10}\u{da}\u{33}\u{2d}\u{1a}\u{de}\u{60}\u{86}\u{69}\u{23}\u{69}\u{2a}\u{7c}\u{cd}\u{4b}\u{51}\u{0d}\u{95}\u{54}\u{39}\u{77}\u{2e}\u{29}\u{ea}\u{1b}\u{a6}\u{50}\u{a2}\u{6a}\u{8f}\u{6f}\u{50}\u{99}\u{5c}\u{3e}\u{54}\u{fb}\u{ef}\u{50}\u{5b}\u{0b}\u{07}\u{45}\u{17}\u{89}\u{6d}\u{28}\u{13}\u{77}\u{37}\u{1d}\u{db}\u{8e}\u{1e}\u{4a}\u{05}\u{66}\u{4a}\u{6f}\u{99}\u{20}\u{e5}\u{70}\u{e2}\u{b9}\u{71}\u{7e}\u{0c}\u{6d}\u{49}\u{04}\u{2d}\u{7a}\u{fe}\u{72}\u{c7}\u{f2}\u{59}\u{30}\u{8f}\u{bb}\u{02}\u{5d}\u{73}\u{e5}\u{c9}\u{20}\u{ea}\u{78}\u{ec}\u{20}\u{90}\u{f0}\u{8a}\u{7f}\u{42}\u{17}\u{7c}\u{47}\u{19}\u{60}\u{b0}\u{16}\u{bd}\u{26}\u{b7}\u{71}\u{b6}\u{c7}\u{9f}\u{0e}\u{d1}\u{33}\u{82}\u{3d}\u{d3}\u{ab}\u{ee}\u{63}\u{99}\u{c8}\u{2b}\u{53}\u{a0}\u{44}\u{5c}\u{71}\u{01}\u{c6}\u{cc}\u{44}\u{1f}\u{32}\u{4f}\u{3c}\u{ca}\u{c0}\u{29}\u{3d}\u{52}\u{d3}\u{61}\u{19}\u{58}\u{a9}\u{7d}\u{65}\u{b4}\u{dc}\u{cf}\u{0d}\u{f4}\u{3d}\u{f1}\u{08}\u{a9}\u{42}\u{da}\u{23}\u{09}\u{d8}\u{bf}\u{5e}\u{50}\u{49}\u{f8}\u{4d}\u{c0}\u{cb}\u{47}\u{4c}\u{1c}\u{4f}\u{f7}\u{7b}\u{2b}\u{d8}\u{16}\u{18}\u{c5}\u{31}\u{92}\u{3b}\u{b5}\u{6f}\u{dc}\u{6c}\u{0d}\u{92}\u{88}\u{16}\u{d1}\u{9e}\u{db}\u{3f}\u{e2}\u{e9}\u{da}\u{5f}\u{d4}\u{84}\u{e2}\u{46}\u{61}\u{5a}\u{de}\u{1c}\u{55}\u{cf}\u{a4}\u{00}\u{be}\u{fd}\u{ce}\u{67}\u{f1}\u{4a}\u{69}\u{1c}\u{97}\u{e6}\u{20}\u{48}\u{d8}\u{5d}\u{7f}\u{7e}\u{ae}\u{71}\u{20}\u{0e}\u{4e}\u{ae}\u{c0}\u{56}\u{a9}\u{91}\u{01}\u{3c}\u{82}\u{1d}\u{0f}\u{72}\u{e7}\u{76}\u{ec}\u{29}\u{49}\u{d6}\u{5d}\u{2d}\u{83}\u{e3}\u{db}\u{36}\u{06}\u{a9}\u{3b}\u{66}\u{13}\u{97}\u{87}\u{6a}\u{d5}\u{b6}\u{3d}\u{50}\u{5e}\u{52}\u{b9}\u{4b}\u{c7}\u{73}\u{57}\u{78}\u{c9}\u{f4}\u{2e}\u{59}\u{07}\u{95}\u{93}\u{6f}\u{d0}\u{4b}\u{17}\u{57}\u{19}\u{3e}\u{27}\u{27}\u{c7}\u{60}\u{db}\u{3b}\u{ed}\u{9a}\u{0e}\u{53}\u{44}\u{16}\u{3e}\u{3f}\u{8d}\u{92}\u{6d}\u{77}\u{a2}\u{0a}\u{eb}\u{3f}\u{52}\u{a8}\u{c6}\u{55}\u{5e}\u{31}\u{49}\u{37}\u{85}\u{f4}\u{c5}\u{1f}\u{26}\u{2d}\u{a9}\u{1c}\u{bf}\u{8b}\u{27}\u{54}\u{da}\u{c3}\u{6a}\u{20}\u{e5}\u{2a}\u{78}\u{04}\u{b0}\u{d6}\u{90}\u{70}\u{72}\u{aa}\u{8b}\u{68}\u{bd}\u{88}\u{f7}\u{02}\u{5f}\u{48}\u{b1}\u{7e}\u{c0}\u{58}\u{4c}\u{3f}\u{66}\u{1a}\u{f9}\u{3e}\u{e1}\u{65}\u{c0}\u{70}\u{a7}\u{cf}\u{38}\u{69}\u{af}\u{f0}\u{56}\u{6c}\u{64}\u{49}\u{9c}\u{27}\u{ad}\u{78}\u{74}\u{4f}\u{c2}\u{87}\u{de}\u{56}\u{39}\u{00}\u{da}\u{77}\u{0b}\u{cb}\u{2d}\u{1b}\u{89}\u{fb}\u{35}\u{4f}\u{02}\u{f5}\u{08}\u{51}\u{13}\u{60}\u{c1}\u{0a}\u{5a}\u{47}\u{4d}\u{26}\u{1c}\u{33}\u{30}\u{78}\u{da}\u{c0}\u{9c}\u{46}\u{47}\u{e2}\u{5b}\u{79}\u{60}\u{49}\u{6e}\u{37}\u{67}\u{53}\u{0a}\u{3e}\u{e9}\u{ec}\u{46}\u{39}\u{b2}\u{f1}\u{34}\u{0d}\u{c6}\u{84}\u{53}\u{75}\u{6e}\u{e1}\u{0c}\u{59}\u{d9}\u{1e}\u{de}\u{29}\u{85}\u{10}\u{7b}\u{49}\u{49}\u{a5}\u{77}\u{79}\u{be}\u{49}\u{56}\u{2e}\u{36}\u{e7}\u{0b}\u{3a}\u{bb}\u{4f}\u{03}\u{62}\u{7b}\u{d2}\u{4d}\u{31}\u{95}\u{2f}\u{bd}\u{38}\u{7b}\u{a8}\u{4f}\u{21}\u{e1}\u{ec}\u{46}\u{70}\u{76}\u{95}\u{7d}\u{29}\u{22}\u{78}\u{88}\u{0a}\u{90}\u{dd}\u{9d}\u{5c}\u{da}\u{de}\u{19}\u{51}\u{cf}\u{f0}\u{fc}\u{59}\u{52}\u{65}\u{7c}\u{33}\u{13}\u{df}\u{f3}\u{48}\u{da}\u{bb}\u{2a}\u{75}\u{db}\u{60}\u{b2}\u{02}\u{15}\u{d4}\u{fc}\u{19}\u{ed}\u{1b}\u{ec}\u{7f}\u{35}\u{a8}\u{ff}\u{28}\u{31}\u{07}\u{2d}\u{12}\u{c8}\u{dc}\u{88}\u{46}\u{7c}\u{8a}\u{5b}\u{22}",
    )
}

/// Provides metadata using the Google Earth Enterprise REST API. This is used
/// by the GoogleEarthEnterpriseImageryProvider and
/// GoogleEarthEnterpriseTerrainProvider to share metadata requests.
pub struct GoogleEarthEnterpriseMetadata {
    /// True if imagery is available.
    pub imagery_present: bool,
    /// True if imagery is sent as a protocol buffer, false if sent as plain
    /// images. If `None` we will try both.
    pub proto_imagery: Option<bool>,
    /// True if terrain is available.
    pub terrain_present: bool,
    /// Exponent used to compute constant to calculate negative height values.
    pub negative_altitude_exponent_bias: u32,
    /// Threshold where any numbers smaller are actually negative values. They
    /// are multiplied by -2^negativeAltitudeExponentBias.
    pub negative_altitude_threshold: f64,
    /// Dictionary of provider id to copyright strings.
    pub providers: HashMap<u32, Credit>,
    /// Key used to decode packets.
    pub key: Option<Vec<u8>>,
    /// The resource used for metadata requests (mirrors `_resource` /
    /// the `resource` property).
    pub resource: Resource,
    /// Mirrors `_quadPacketVersion`.
    pub quad_packet_version: u32,
    /// Mirrors `_tileInfo`: quadkey → tile info (`None` mirrors JS `null`,
    /// a missing entry mirrors JS `undefined`).
    pub tile_info: RefCell<HashMap<String, Option<GoogleEarthEnterpriseTileInformation>>>,
}

impl GoogleEarthEnterpriseMetadata {
    /// Mirrors the JS constructor (defaults only; `_resource` must be
    /// supplied via [`GoogleEarthEnterpriseMetadata::from_url`] /
    /// [`GoogleEarthEnterpriseMetadata::from_resource`]).
    pub fn new(resource: Resource) -> Self {
        Self {
            imagery_present: true,
            proto_imagery: None,
            terrain_present: true,
            negative_altitude_exponent_bias: 32,
            negative_altitude_threshold: CesiumMath::EPSILON12,
            providers: HashMap::new(),
            key: None,
            resource,
            quad_packet_version: 1,
            tile_info: RefCell::new(HashMap::new()),
        }
    }

    /// Gets the name of the Google Earth Enterprise server.
    pub fn url(&self) -> String {
        self.resource.url()
    }

    /// Creates a metadata object using the Google Earth Enterprise REST API.
    ///
    /// Mirrors `GoogleEarthEnterpriseMetadata.fromUrl` (DEVIATION 1).
    pub async fn from_url(
        resource_or_url: Option<&str>,
        backend: &(impl ResourceBackend + ?Sized),
    ) -> Result<Self, RuntimeError> {
        check::defined("resourceOrUrl", resource_or_url.as_ref());
        let mut resource = Resource::new(resource_or_url.unwrap().to_string());
        resource.append_forward_slash();
        Self::from_resource_internal(resource, backend).await
    }

    /// Variant of [`GoogleEarthEnterpriseMetadata::from_url`] accepting an
    /// existing [`Resource`].
    pub async fn from_resource(
        mut resource: Resource,
        backend: &(impl ResourceBackend + ?Sized),
    ) -> Result<Self, RuntimeError> {
        resource.append_forward_slash();
        Self::from_resource_internal(resource, backend).await
    }

    async fn from_resource_internal(
        resource: Resource,
        backend: &(impl ResourceBackend + ?Sized),
    ) -> Result<Self, RuntimeError> {
        let mut metadata = Self::new(resource);

        let result: Result<(), RuntimeError> = async {
            request_db_root(&mut metadata, backend).await;
            let version = metadata.quad_packet_version;
            metadata
                .get_quad_tree_packet(Some(""), Some(version), backend)
                .await?;
            Ok(())
        }
        .await;

        if let Err(error) = result {
            let url = get_metadata_resource(&metadata.resource, "", 1).url();
            return Err(RuntimeError::new(Some(&format!(
                "An error occurred while accessing {url}: {error}"
            ))));
        }

        Ok(metadata)
    }

    /// Converts a tiles (x, y, level) position into a quadkey used to request
    /// an image from a Google Earth Enterprise server.
    ///
    /// Mirrors `GoogleEarthEnterpriseMetadata.tileXYToQuadKey`.
    pub fn tile_xy_to_quad_key(x: i32, y: i32, level: i32) -> String {
        let mut quadkey = String::new();
        for i in (0..=level).rev() {
            let bitmask = 1u32 << i;
            let mut digit = 0u32;

            // Tile Layout
            // ___ ___
            //|   |   |
            //| 3 | 2 |
            //|-------|
            //| 0 | 1 |
            //|___|___|
            //

            if !is_bit_set(y as u32, bitmask) {
                // Top Row
                digit |= 2;
                if !is_bit_set(x as u32, bitmask) {
                    // Right to left
                    digit |= 1;
                }
            } else if is_bit_set(x as u32, bitmask) {
                // Left to right
                digit |= 1;
            }

            quadkey.push_str(&digit.to_string());
        }
        quadkey
    }

    /// Converts a tile's quadkey used to request an image from a Google Earth
    /// Enterprise server into the (x, y, level) position.
    ///
    /// Mirrors `GoogleEarthEnterpriseMetadata.quadKeyToTileXY`.
    pub fn quad_key_to_tile_xy(quadkey: &str) -> (i32, i32, i32) {
        let mut x = 0u32;
        let mut y = 0u32;
        let level = quadkey.len() as i32 - 1;
        for i in (0..=level).rev() {
            let bitmask = 1u32 << i;
            let digit = quadkey
                .chars()
                .nth((level - i) as usize)
                .and_then(|c| c.to_digit(10))
                .unwrap_or(0);

            if is_bit_set(digit, 2) {
                // Top Row
                if !is_bit_set(digit, 1) {
                    // // Right to left
                    x |= bitmask;
                }
            } else {
                y |= bitmask;
                if is_bit_set(digit, 1) {
                    // Left to right
                    x |= bitmask;
                }
            }
        }
        (x as i32, y as i32, level)
    }

    /// Mirrors `GoogleEarthEnterpriseMetadata.prototype.isValid`.
    pub fn is_valid(&self, quad_key: &str) -> bool {
        let info = self.get_tile_information_from_quad_key(quad_key);
        if let Some(info) = info {
            return info.is_some();
        }

        let mut valid = true;
        let mut q = quad_key.to_string();
        while q.len() > 1 {
            let last = q[q.len() - 1..].to_string();
            q = q[..q.len() - 1].to_string();
            let info = self.get_tile_information_from_quad_key(&q);
            if let Some(info) = info {
                match info {
                    Some(info) => {
                        if !info.has_subtree()
                            && !info.has_child(last.parse::<usize>().unwrap_or(0))
                        {
                            // We have no subtree or child available at some
                            // point in this node's ancestry
                            valid = false;
                        }
                    }
                    None => {
                        // Some node in the ancestry was loaded and said there
                        // wasn't a subtree
                        valid = false;
                    }
                }
                break;
            }
        }

        valid
    }

    /// Retrieves a Google Earth Enterprise quadtree packet.
    ///
    /// Mirrors `getQuadTreePacket(quadKey, version, request)` (DEVIATIONS
    /// 1/2). Returns `Ok(())` when the packet was merged (or the request was
    /// "throttled": backend yielded `None`).
    pub async fn get_quad_tree_packet(
        &self,
        quad_key: Option<&str>,
        version: Option<u32>,
        backend: &(impl ResourceBackend + ?Sized),
    ) -> Result<(), RuntimeError> {
        let version = version.unwrap_or(1);
        let quad_key = quad_key.unwrap_or("");
        let mut resource = get_metadata_resource(&self.resource, quad_key, version);

        let metadata = match resource.fetch_array_buffer(backend).await {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(()), // Throttled
            Err(error) => return Err(RuntimeError::new(Some(&format!("{error}")))),
        };

        let key = self.key.clone().unwrap_or_default();
        let mut buffer = metadata;
        let result = decode_google_earth_enterprise_packet(
            &key,
            &mut buffer,
            GeePacketType::Metadata,
            quad_key,
        )
        .map_err(|e| RuntimeError::new(Some(&e.message)))?;

        let result = match result {
            GeePacketResult::Metadata(result) => result,
            _ => unreachable!(),
        };

        let mut tile_info = self.tile_info.borrow_mut();
        let mut root: Option<GoogleEarthEnterpriseTileInformation> = None;
        let mut top_level_key_length: i64 = -1;
        let mut result = result;
        if !quad_key.is_empty() {
            // Root tile has no data except children bits, so put them into
            // the tile info
            top_level_key_length = quad_key.len() as i64 + 1;
            let top = result.remove(quad_key).flatten();
            let existing = tile_info.get(quad_key).and_then(|i| i.clone());
            if let (Some(existing), Some(top)) = (existing, top) {
                let mut merged = existing;
                merged.or_bits(top.bits());
                root = Some(merged);
            }
        }

        // Copy the resulting objects into tileInfo
        // Make sure we start with shorter quadkeys first, so we know the
        //  parents have already been processed. Otherwise we can lose
        //  ancestorHasTerrain along the way.
        let mut keys: Vec<String> = result.keys().cloned().collect();
        keys.sort_by(|a, b| a.len().cmp(&b.len()));
        for key in keys {
            let r = result.get(&key).unwrap();
            if let Some(r) = r {
                let mut info = GoogleEarthEnterpriseTileInformation::clone_info(r);
                let key_length = key.len() as i64;
                if key_length == top_level_key_length {
                    if let Some(root) = root.as_ref() {
                        info.set_parent(root);
                    }
                } else if key_length > 1 {
                    let parent = tile_info
                        .get(&key[..key.len() - 1])
                        .and_then(|i| i.clone())
                        .expect("parent tile information must be loaded first");
                    info.set_parent(&parent);
                }
                tile_info.insert(key, Some(info));
            } else {
                tile_info.insert(key, None);
            }
        }
        if let Some(root) = root {
            tile_info.insert(quad_key.to_string(), Some(root));
        }

        Ok(())
    }

    /// Populates the metadata subtree down to the specified tile.
    ///
    /// Mirrors `populateSubtree(x, y, level, request)`.
    pub async fn populate_subtree_xy(
        &self,
        x: i32,
        y: i32,
        level: i32,
        backend: &(impl ResourceBackend + ?Sized),
    ) -> Result<Option<GoogleEarthEnterpriseTileInformation>, RuntimeError> {
        let quadkey = Self::tile_xy_to_quad_key(x, y, level);
        self.populate_subtree(&quadkey, backend).await
    }

    /// Mirrors the module-level `populateSubtree(that, quadKey, request)`
    /// (DEVIATION 3). Returns the resolved tile info, `Ok(None)` mirroring a
    /// throttled request, or the load error.
    pub async fn populate_subtree(
        &self,
        quad_key: &str,
        backend: &(impl ResourceBackend + ?Sized),
    ) -> Result<Option<GoogleEarthEnterpriseTileInformation>, RuntimeError> {
        loop {
            let t = self
                .tile_info
                .borrow()
                .get(quad_key)
                .cloned();

            // If we have tileInfo make sure sure it is not a node with a
            // subtree that's not loaded
            if let Some(Some(t)) = t.as_ref() {
                if !t.has_subtree() || t.has_children() {
                    return Ok(Some(t.clone()));
                }
            }

            let mut q = quad_key.to_string();
            let mut t = t;
            while t.is_none() && q.len() > 1 {
                q = q[..q.len() - 1].to_string();
                t = self.tile_info.borrow().get(&q).cloned();
            }

            // t is either
            //   null so one of its parents was a leaf node, so this tile
            //     doesn't exist
            //   exists but doesn't have a subtree to request
            //   undefined so no parent exists - this shouldn't ever happen
            //     once the provider is ready
            let has_subtree = matches!(t.as_ref(), Some(Some(info)) if info.has_subtree());
            if !has_subtree {
                return Err(RuntimeError::new(Some(&format!(
                    "Couldn't load metadata for tile {quad_key}"
                ))));
            }

            let cnode_version = match t.as_ref() {
                Some(Some(info)) => info.cnode_version,
                _ => unreachable!(),
            };

            self.get_quad_tree_packet(Some(&q), Some(cnode_version), backend)
                .await?;

            // Recursively loop in case we need multiple subtree requests
            // (JS recursively calls populateSubtree after each packet).
        }
    }

    /// Gets information about a tile.
    ///
    /// Mirrors `getTileInformation(x, y, level)`. `None` mirrors JS
    /// `undefined`; `Some(None)` mirrors JS `null`.
    pub fn get_tile_information(
        &self,
        x: i32,
        y: i32,
        level: i32,
    ) -> Option<Option<GoogleEarthEnterpriseTileInformation>> {
        let quadkey = Self::tile_xy_to_quad_key(x, y, level);
        self.tile_info.borrow().get(&quadkey).cloned()
    }

    /// Gets information about a tile from a quadKey.
    ///
    /// Mirrors `getTileInformationFromQuadKey(quadkey)`.
    pub fn get_tile_information_from_quad_key(
        &self,
        quadkey: &str,
    ) -> Option<Option<GoogleEarthEnterpriseTileInformation>> {
        self.tile_info.borrow().get(quadkey).cloned()
    }
}

/// Mirrors `getMetadataResource(that, quadKey, version)`.
pub fn get_metadata_resource(resource: &Resource, quad_key: &str, version: u32) -> Resource {
    resource
        .clone_resource()
        .get_derived_resource_with_options(DerivedResourceOptions {
            url: Some(&format!("flatfile?q2-0{quad_key}-q.{version}")),
            ..Default::default()
        })
}

/// Mirrors `requestDbRoot(that)` (DEVIATION 4: the protobuf dbroot parser is
/// browser-only, so the JS `catch` fallback is always taken).
async fn request_db_root(
    that: &mut GoogleEarthEnterpriseMetadata,
    backend: &(impl ResourceBackend + ?Sized),
) {
    let query = HashMap::from([("output".to_string(), "proto".to_string())]);
    let mut resource = that
        .resource
        .clone_resource()
        .get_derived_resource_with_options(DerivedResourceOptions {
            url: Some("dbRoot.v5"),
            query_parameters: Some(&query),
            ..Default::default()
        });

    // The JS implementation loads the dbroot parser script, decodes the
    // EncryptedDbRootProto protobuf and fills imagery/terrain/provider
    // defaults from it. That path requires the browser-only script loader;
    // any failure falls through to the defaults below (JS `.catch`).
    let _ = resource.fetch_array_buffer(backend).await;

    // Just eat the error and use the default values.
    println!("Failed to retrieve {}. Using defaults.", resource.url());
    that.key = Some(default_key());
}
