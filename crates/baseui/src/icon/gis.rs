//! `font-gis` icon pack constants (generated from font-gis.css).
//!
//! font-gis by Jean-Marc Viglino, licensed OFL/MIT/Apache/CC-BY.
//! Each constant is an [`Icon`](super::Icon) into the embedded icon font at
//! [`FontId::Icon(GIS)`](baseui_core::FontId). Enabled by the `icons-gis` feature.
//!
//! [`by_name`] resolves an icon from its `fg-` name at runtime, which is how
//! config files and scripts (e.g. `icon = "gis:compass"`) reference icons.

use super::Icon;
use baseui_core::FontId;

/// Index of the font-gis pack in the icon-font registry.
pub const GIS: u16 = 0;

/// An arbitrary font-gis glyph by code point (e.g. `gis(0xea90)`).
pub const fn gis(code: u32) -> Icon {
    Icon::font(FontId::Icon(GIS), unsafe { char::from_u32_unchecked(code) })
}

/// Resolve a font-gis icon from its name (the `fg-` prefix is optional):
/// `by_name("compass")` or `by_name("fg-compass")`.
pub fn by_name(name: &str) -> Option<Icon> {
    let name = name.strip_prefix("fg-").unwrap_or(name);
    let code: u32 = match name {
        "north-arrow" => 0xea8b,
        "north-arrow-n" => 0xea8c,
        "compass" => 0xea90,
        "compass-needle" => 0xea91,
        "compass-rose" => 0xea92,
        "compass-rose-n" => 0xea93,
        "compass-alt" => 0xeb06,
        "compass-alt-o" => 0xeb07,
        "gpx-file" => 0xea99,
        "geojson-file" => 0xea9a,
        "kml-file" => 0xea9b,
        "wms" => 0xea9c,
        "wmts" => 0xea9d,
        "wfs" => 0xea9e,
        "wfs-t" => 0xea9f,
        "mvt" => 0xeaa0,
        "xyz" => 0xeaa1,
        "shape-file" => 0xeaa2,
        "esri-json-file" => 0xeaa3,
        "topojson-file" => 0xeaa4,
        "folder-map" => 0xeb2f,
        "world-folder-o" => 0xeb30,
        "world-folder" => 0xeb31,
        "folder-globe" => 0xeb32,
        "folder-globe-o" => 0xeb33,
        "folder-maps" => 0xeb34,
        "folder-poi" => 0xeb35,
        "folder-poi-o" => 0xeb36,
        "folder-pois" => 0xeb37,
        "earth-net" => 0xeb38,
        "earth-net-o" => 0xeb39,
        "wcs" => 0xeb59,
        "i3s-file" => 0xeb5a,
        "i3s-web" => 0xeb5b,
        "3dtiles-file" => 0xeb5c,
        "3dtiles-web" => 0xeb5d,
        "wps" => 0xeb5e,
        "wcps" => 0xeb5f,
        "openls" => 0xeb60,
        "wmc" => 0xeb61,
        "tjs" => 0xeb62,
        "sld" => 0xeb63,
        "sos" => 0xeb64,
        "sps" => 0xeb65,
        "csw" => 0xeb66,
        "arrow-o" => 0xea3a,
        "arrow" => 0xea3b,
        "modify-line" => 0xea3c,
        "modify-poly" => 0xea3d,
        "modify-poly-o" => 0xea40,
        "copy-point" => 0xea4f,
        "copy-line" => 0xea50,
        "copy-poly" => 0xea51,
        "buffer" => 0xea6e,
        "difference" => 0xea6f,
        "intersection" => 0xea70,
        "union" => 0xea71,
        "sym-difference" => 0xea72,
        "move" => 0xea73,
        "move-alt" => 0xea74,
        "offset" => 0xea75,
        "snap" => 0xea76,
        "split" => 0xea77,
        "split-line" => 0xea78,
        "split-polygon" => 0xea79,
        "convex-hull" => 0xeaa8,
        "select-extent" => 0xeaad,
        "snap-ortho" => 0xeaae,
        "color" => 0xeaaf,
        "rotate" => 0xeae3,
        "flip-h" => 0xeae4,
        "flip-v" => 0xeae5,
        "simplify" => 0xeae6,
        "proj-point" => 0xeae7,
        "scale-poly" => 0xeae8,
        "skeletonize" => 0xeb17,
        "dilatation" => 0xeb18,
        "erosion" => 0xeb19,
        "translate" => 0xeb26,
        "translate-x" => 0xeb27,
        "translate-y" => 0xeb28,
        "map" => 0xea53,
        "map-o" => 0xea54,
        "map-poi" => 0xea55,
        "world-map-alt" => 0xea56,
        "map-route" => 0xea57,
        "road-map" => 0xea58,
        "cadastre-map" => 0xea59,
        "landcover-map" => 0xea5a,
        "bus-map" => 0xea5b,
        "contour-map" => 0xea5c,
        "hydro-map" => 0xea5d,
        "world-map" => 0xea68,
        "pirate-map" => 0xea6b,
        "story-map" => 0xea6d,
        "map-book" => 0xea7a,
        "map-legend" => 0xea85,
        "map-legend-o" => 0xea86,
        "map-options" => 0xea94,
        "map-options-alt" => 0xea95,
        "map-print" => 0xea96,
        "world-map-alt-o" => 0xeab1,
        "flow-map" => 0xeab2,
        "map-stat" => 0xeab3,
        "statistic-map" => 0xeab4,
        "voronoi-map" => 0xeab7,
        "triangle-map" => 0xeab8,
        "phone-map" => 0xeab9,
        "hex-map" => 0xeaba,
        "map-bookmark" => 0xeabd,
        "map-tag" => 0xeabf,
        "map-tags" => 0xeac0,
        "compare-map" => 0xead8,
        "swipe-map-v" => 0xead9,
        "swipe-map-h" => 0xeada,
        "magnify-map" => 0xeadb,
        "map-share" => 0xeae0,
        "map-send" => 0xeae1,
        "map-share-alt" => 0xeae2,
        "map-add" => 0xeae9,
        "map-rm" => 0xeaea,
        "map-time" => 0xeaee,
        "time-map" => 0xeaef,
        "map-play" => 0xeaf5,
        "map-star" => 0xeaf6,
        "map-favorite" => 0xeaf7,
        "map-smiley" => 0xeb00,
        "map-control" => 0xeb02,
        "map-lock" => 0xeb04,
        "map-unlock" => 0xeb05,
        "weather-map" => 0xeb0b,
        "story-map-o" => 0xeb2a,
        "story-maps" => 0xeb2b,
        "map-edit" => 0xeb2c,
        "height-map" => 0xeb40,
        "map-user" => 0xeb4b,
        "map-users" => 0xeb4c,
        "earth" => 0xea22,
        "earth-euro-africa" => 0xea23,
        "earth-atlantic" => 0xea24,
        "earth-america" => 0xea25,
        "earth-pacific" => 0xea26,
        "earth-australia" => 0xea27,
        "earth-asia" => 0xea28,
        "earth-north" => 0xea29,
        "earth-south" => 0xea2a,
        "earth-o" => 0xea2b,
        "earth-euro-africa-o" => 0xea2c,
        "earth-atlantic-o" => 0xea2d,
        "earth-america-o" => 0xea2e,
        "earth-pacific-o" => 0xea2f,
        "earth-australia-o" => 0xea30,
        "earth-asia-o" => 0xea31,
        "earth-north-o" => 0xea32,
        "earth-south-o" => 0xea33,
        "globe" => 0xea36,
        "globe-o" => 0xea37,
        "globe-alt" => 0xea38,
        "globe-alt-o" => 0xea39,
        "globe-poi" => 0xea82,
        "network" => 0xeabb,
        "network-o" => 0xeabc,
        "tag" => 0xeac1,
        "tag-o" => 0xeac2,
        "tags" => 0xeac3,
        "tags-o" => 0xeac4,
        "earth-gear" => 0xead5,
        "globe-earth" => 0xeaf8,
        "globe-earth-alt" => 0xeaf9,
        "globe-favorite" => 0xeafb,
        "globe-options" => 0xeafc,
        "globe-share" => 0xeafd,
        "globe-star" => 0xeafe,
        "globe-smiley" => 0xeaff,
        "globe-user" => 0xeb0c,
        "globe-users" => 0xeb0d,
        "globe-shield" => 0xeb0e,
        "earth-network" => 0xeb0f,
        "earth-network-o" => 0xeb10,
        "globe-gear" => 0xeb11,
        "point" => 0xea01,
        "polyline-pt" => 0xea02,
        "polygon-pt" => 0xea03,
        "polygon-hole-pt" => 0xea04,
        "rectangle-pt" => 0xea05,
        "square-pt" => 0xea06,
        "circle-o" => 0xea07,
        "polyline" => 0xea09,
        "polygon-o" => 0xea0a,
        "polygon-hole-o" => 0xea0b,
        "rectangle-o" => 0xea0c,
        "square-o" => 0xea0d,
        "polygon-hole" => 0xea0e,
        "polygon" => 0xea0f,
        "rectangle" => 0xea10,
        "square" => 0xea11,
        "circle" => 0xea12,
        "multipoint" => 0xea52,
        "bbox-alt" => 0xeaa9,
        "extent-alt" => 0xeaaa,
        "bbox" => 0xeaab,
        "extent" => 0xeaac,
        "map-extent" => 0xeab0,
        "regular-shape-pt" => 0xeaeb,
        "regular-shape-o" => 0xeaec,
        "regular-shape" => 0xeaed,
        "measure" => 0xea08,
        "measure-line" => 0xea13,
        "measure-area" => 0xea14,
        "measure-area-alt" => 0xea15,
        "scale" => 0xeb01,
        "azimuth" => 0xeb53,
        "layer" => 0xea41,
        "layer-o" => 0xea42,
        "layers" => 0xea43,
        "layers-o" => 0xea44,
        "layer-up" => 0xea45,
        "layer-down" => 0xea46,
        "layer-alt" => 0xea47,
        "layer-alt-o" => 0xea48,
        "layer-stack" => 0xea49,
        "layer-stack-o" => 0xea4a,
        "layer-add" => 0xea4b,
        "layer-add-o" => 0xea4c,
        "layer-rm" => 0xea4d,
        "layer-rm-o" => 0xea4e,
        "layer-poi" => 0xea6a,
        "layer-download" => 0xea97,
        "layer-upload" => 0xea98,
        "layer-road" => 0xeaf0,
        "layer-hydro" => 0xeaf1,
        "layer-landcover" => 0xeaf2,
        "layer-contour" => 0xeaf3,
        "layer-stat" => 0xeaf4,
        "layer-stat-alt" => 0xeb29,
        "layer-edit" => 0xeb2d,
        "layer-alt-edit" => 0xeb2e,
        "layer-height" => 0xeb41,
        "layer-2-add-o" => 0xeb46,
        "layer-2-rm-o" => 0xeb47,
        "layer-alt-add-o" => 0xeb48,
        "layer-alt-rm-o" => 0xeb49,
        "layer-alt-x-o" => 0xeb4a,
        "layers-poi" => 0xeb4f,
        "layer-alt-poi" => 0xeb50,
        "mosaic" => 0xeb6c,
        "pyramid" => 0xeb6d,
        "help-larrow" => 0xea3e,
        "help-rarrow" => 0xea3f,
        "home" => 0xeb14,
        "satellite" => 0xeb3a,
        "satellite-earth" => 0xeb3b,
        "drone" => 0xeb3f,
        "poi" => 0xea16,
        "poi-o" => 0xea17,
        "poi-alt" => 0xea18,
        "poi-alt-o" => 0xea19,
        "pin" => 0xea1a,
        "pushpin" => 0xea1b,
        "pois" => 0xea1c,
        "pois-o" => 0xea1d,
        "poi-favorite" => 0xea1e,
        "poi-favorite-o" => 0xea1f,
        "poi-home" => 0xea20,
        "poi-home-o" => 0xea21,
        "poi-earth" => 0xea34,
        "pin-earth" => 0xea35,
        "pirate-poi" => 0xea6c,
        "location-poi" => 0xea83,
        "location-poi-o" => 0xea84,
        "bookmark-poi" => 0xeabe,
        "bookmark-poi-b" => 0xeacf,
        "poi-map" => 0xead6,
        "poi-map-o" => 0xead7,
        "location-man" => 0xeb15,
        "location-man-alt" => 0xeb16,
        "poi-info" => 0xeb1c,
        "poi-info-o" => 0xeb1d,
        "position" => 0xeb22,
        "position-o" => 0xeb23,
        "position-man" => 0xeb24,
        "poi-slash" => 0xeb4d,
        "poi-slash-o" => 0xeb4e,
        "proj-conic" => 0xeb67,
        "proj-geo" => 0xeb68,
        "proj-square" => 0xeb69,
        "proj-stereo" => 0xeb6a,
        "proj-utm" => 0xeb6b,
        "route" => 0xea7b,
        "route-start" => 0xea7c,
        "route-end" => 0xea7d,
        "car" => 0xea7e,
        "bicycle" => 0xea7f,
        "pedestrian" => 0xea80,
        "hiker" => 0xea81,
        "location-arrow" => 0xea87,
        "location-arrow-o" => 0xea88,
        "location" => 0xea89,
        "location-on" => 0xea8a,
        "direct" => 0xea8d,
        "revers" => 0xea8e,
        "timer" => 0xea8f,
        "signpost" => 0xeab5,
        "direction" => 0xeab6,
        "flag" => 0xeac5,
        "flag-o" => 0xeac6,
        "flag-start" => 0xeac7,
        "flag-start-o" => 0xeac8,
        "flag-finish" => 0xeac9,
        "flag-b" => 0xeaca,
        "flab-b-o" => 0xeacb,
        "flag-start-b" => 0xeacc,
        "flag-start-b-o" => 0xeacd,
        "flag-finish-b-o" => 0xeace,
        "start" => 0xead0,
        "start-o" => 0xead1,
        "step" => 0xead2,
        "step-o" => 0xead3,
        "finish" => 0xead4,
        "directions" => 0xeb03,
        "phone-route" => 0xeb08,
        "phone-route-alt" => 0xeb09,
        "phone-route-alt-r" => 0xeb0a,
        "map-search" => 0xea5e,
        "search-map" => 0xea5f,
        "search-poi" => 0xea60,
        "search-globe" => 0xea61,
        "search-home" => 0xea62,
        "search-address" => 0xea63,
        "search-attribtues" => 0xea64,
        "search-propertie" => 0xea65,
        "search-feature" => 0xea66,
        "search-layer" => 0xea67,
        "search-country" => 0xea69,
        "search-globe-alt" => 0xeafa,
        "search-coord" => 0xeb12,
        "search-data" => 0xeb13,
        "zoom-in" => 0xeaa5,
        "zoom-out" => 0xeaa6,
        "full-screen" => 0xeaa7,
        "screen-dub" => 0xeadc,
        "screen-split-h" => 0xeadd,
        "screen-split-v" => 0xeade,
        "screen-mag" => 0xeadf,
        "coord-system" => 0xeb1a,
        "coord-system-3d" => 0xeb1b,
        "coord-system-alt" => 0xeb1e,
        "coord-system-3d-alt" => 0xeb1f,
        "grid" => 0xeb20,
        "cube-3d" => 0xeb21,
        "coord-grid" => 0xeb25,
        "photogrammetry" => 0xeb3c,
        "360" => 0xeb3d,
        "topography" => 0xeb3e,
        "gnss" => 0xeb42,
        "gnss-antenna" => 0xeb43,
        "tacheometer" => 0xeb44,
        "theodolite" => 0xeb45,
        "profile" => 0xeb51,
        "profile-o" => 0xeb52,
        "screen-dub1" => 0xeb54,
        "screen-dub2" => 0xeb55,
        "screen-dub-o" => 0xeb56,
        "screen-mag-o" => 0xeb57,
        "screen-mag-alt" => 0xeb58,
        "polygon-o-dash" => 0xeb6e,
        "polyline-dash" => 0xeb6f,
        _ => return None,
    };
    Some(gis(code))
}

/// `fg-north-arrow`
pub const NORTH_ARROW: Icon = Icon::font(FontId::Icon(GIS), '\u{ea8b}');
/// `fg-north-arrow-n`
pub const NORTH_ARROW_N: Icon = Icon::font(FontId::Icon(GIS), '\u{ea8c}');
/// `fg-compass`
pub const COMPASS: Icon = Icon::font(FontId::Icon(GIS), '\u{ea90}');
/// `fg-compass-needle`
pub const COMPASS_NEEDLE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea91}');
/// `fg-compass-rose`
pub const COMPASS_ROSE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea92}');
/// `fg-compass-rose-n`
pub const COMPASS_ROSE_N: Icon = Icon::font(FontId::Icon(GIS), '\u{ea93}');
/// `fg-compass-alt`
pub const COMPASS_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{eb06}');
/// `fg-compass-alt-o`
pub const COMPASS_ALT_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb07}');
/// `fg-gpx-file`
pub const GPX_FILE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea99}');
/// `fg-geojson-file`
pub const GEOJSON_FILE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea9a}');
/// `fg-kml-file`
pub const KML_FILE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea9b}');
/// `fg-wms`
pub const WMS: Icon = Icon::font(FontId::Icon(GIS), '\u{ea9c}');
/// `fg-wmts`
pub const WMTS: Icon = Icon::font(FontId::Icon(GIS), '\u{ea9d}');
/// `fg-wfs`
pub const WFS: Icon = Icon::font(FontId::Icon(GIS), '\u{ea9e}');
/// `fg-wfs-t`
pub const WFS_T: Icon = Icon::font(FontId::Icon(GIS), '\u{ea9f}');
/// `fg-mvt`
pub const MVT: Icon = Icon::font(FontId::Icon(GIS), '\u{eaa0}');
/// `fg-xyz`
pub const XYZ: Icon = Icon::font(FontId::Icon(GIS), '\u{eaa1}');
/// `fg-shape-file`
pub const SHAPE_FILE: Icon = Icon::font(FontId::Icon(GIS), '\u{eaa2}');
/// `fg-esri-json-file`
pub const ESRI_JSON_FILE: Icon = Icon::font(FontId::Icon(GIS), '\u{eaa3}');
/// `fg-topojson-file`
pub const TOPOJSON_FILE: Icon = Icon::font(FontId::Icon(GIS), '\u{eaa4}');
/// `fg-folder-map`
pub const FOLDER_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{eb2f}');
/// `fg-world-folder-o`
pub const WORLD_FOLDER_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb30}');
/// `fg-world-folder`
pub const WORLD_FOLDER: Icon = Icon::font(FontId::Icon(GIS), '\u{eb31}');
/// `fg-folder-globe`
pub const FOLDER_GLOBE: Icon = Icon::font(FontId::Icon(GIS), '\u{eb32}');
/// `fg-folder-globe-o`
pub const FOLDER_GLOBE_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb33}');
/// `fg-folder-maps`
pub const FOLDER_MAPS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb34}');
/// `fg-folder-poi`
pub const FOLDER_POI: Icon = Icon::font(FontId::Icon(GIS), '\u{eb35}');
/// `fg-folder-poi-o`
pub const FOLDER_POI_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb36}');
/// `fg-folder-pois`
pub const FOLDER_POIS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb37}');
/// `fg-earth-net`
pub const EARTH_NET: Icon = Icon::font(FontId::Icon(GIS), '\u{eb38}');
/// `fg-earth-net-o`
pub const EARTH_NET_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb39}');
/// `fg-wcs`
pub const WCS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb59}');
/// `fg-i3s-file`
pub const I3S_FILE: Icon = Icon::font(FontId::Icon(GIS), '\u{eb5a}');
/// `fg-i3s-web`
pub const I3S_WEB: Icon = Icon::font(FontId::Icon(GIS), '\u{eb5b}');
/// `fg-3dtiles-file`
pub const _3DTILES_FILE: Icon = Icon::font(FontId::Icon(GIS), '\u{eb5c}');
/// `fg-3dtiles-web`
pub const _3DTILES_WEB: Icon = Icon::font(FontId::Icon(GIS), '\u{eb5d}');
/// `fg-wps`
pub const WPS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb5e}');
/// `fg-wcps`
pub const WCPS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb5f}');
/// `fg-openls`
pub const OPENLS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb60}');
/// `fg-wmc`
pub const WMC: Icon = Icon::font(FontId::Icon(GIS), '\u{eb61}');
/// `fg-tjs`
pub const TJS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb62}');
/// `fg-sld`
pub const SLD: Icon = Icon::font(FontId::Icon(GIS), '\u{eb63}');
/// `fg-sos`
pub const SOS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb64}');
/// `fg-sps`
pub const SPS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb65}');
/// `fg-csw`
pub const CSW: Icon = Icon::font(FontId::Icon(GIS), '\u{eb66}');
/// `fg-arrow-o`
pub const ARROW_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea3a}');
/// `fg-arrow`
pub const ARROW: Icon = Icon::font(FontId::Icon(GIS), '\u{ea3b}');
/// `fg-modify-line`
pub const MODIFY_LINE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea3c}');
/// `fg-modify-poly`
pub const MODIFY_POLY: Icon = Icon::font(FontId::Icon(GIS), '\u{ea3d}');
/// `fg-modify-poly-o`
pub const MODIFY_POLY_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea40}');
/// `fg-copy-point`
pub const COPY_POINT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea4f}');
/// `fg-copy-line`
pub const COPY_LINE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea50}');
/// `fg-copy-poly`
pub const COPY_POLY: Icon = Icon::font(FontId::Icon(GIS), '\u{ea51}');
/// `fg-buffer`
pub const BUFFER: Icon = Icon::font(FontId::Icon(GIS), '\u{ea6e}');
/// `fg-difference`
pub const DIFFERENCE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea6f}');
/// `fg-intersection`
pub const INTERSECTION: Icon = Icon::font(FontId::Icon(GIS), '\u{ea70}');
/// `fg-union`
pub const UNION: Icon = Icon::font(FontId::Icon(GIS), '\u{ea71}');
/// `fg-sym-difference`
pub const SYM_DIFFERENCE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea72}');
/// `fg-move`
pub const MOVE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea73}');
/// `fg-move-alt`
pub const MOVE_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea74}');
/// `fg-offset`
pub const OFFSET: Icon = Icon::font(FontId::Icon(GIS), '\u{ea75}');
/// `fg-snap`
pub const SNAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ea76}');
/// `fg-split`
pub const SPLIT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea77}');
/// `fg-split-line`
pub const SPLIT_LINE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea78}');
/// `fg-split-polygon`
pub const SPLIT_POLYGON: Icon = Icon::font(FontId::Icon(GIS), '\u{ea79}');
/// `fg-convex-hull`
pub const CONVEX_HULL: Icon = Icon::font(FontId::Icon(GIS), '\u{eaa8}');
/// `fg-select-extent`
pub const SELECT_EXTENT: Icon = Icon::font(FontId::Icon(GIS), '\u{eaad}');
/// `fg-snap-ortho`
pub const SNAP_ORTHO: Icon = Icon::font(FontId::Icon(GIS), '\u{eaae}');
/// `fg-color`
pub const COLOR: Icon = Icon::font(FontId::Icon(GIS), '\u{eaaf}');
/// `fg-rotate`
pub const ROTATE: Icon = Icon::font(FontId::Icon(GIS), '\u{eae3}');
/// `fg-flip-h`
pub const FLIP_H: Icon = Icon::font(FontId::Icon(GIS), '\u{eae4}');
/// `fg-flip-v`
pub const FLIP_V: Icon = Icon::font(FontId::Icon(GIS), '\u{eae5}');
/// `fg-simplify`
pub const SIMPLIFY: Icon = Icon::font(FontId::Icon(GIS), '\u{eae6}');
/// `fg-proj-point`
pub const PROJ_POINT: Icon = Icon::font(FontId::Icon(GIS), '\u{eae7}');
/// `fg-scale-poly`
pub const SCALE_POLY: Icon = Icon::font(FontId::Icon(GIS), '\u{eae8}');
/// `fg-skeletonize`
pub const SKELETONIZE: Icon = Icon::font(FontId::Icon(GIS), '\u{eb17}');
/// `fg-dilatation`
pub const DILATATION: Icon = Icon::font(FontId::Icon(GIS), '\u{eb18}');
/// `fg-erosion`
pub const EROSION: Icon = Icon::font(FontId::Icon(GIS), '\u{eb19}');
/// `fg-translate`
pub const TRANSLATE: Icon = Icon::font(FontId::Icon(GIS), '\u{eb26}');
/// `fg-translate-x`
pub const TRANSLATE_X: Icon = Icon::font(FontId::Icon(GIS), '\u{eb27}');
/// `fg-translate-y`
pub const TRANSLATE_Y: Icon = Icon::font(FontId::Icon(GIS), '\u{eb28}');
/// `fg-map`
pub const MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ea53}');
/// `fg-map-o`
pub const MAP_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea54}');
/// `fg-map-poi`
pub const MAP_POI: Icon = Icon::font(FontId::Icon(GIS), '\u{ea55}');
/// `fg-world-map-alt`
pub const WORLD_MAP_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea56}');
/// `fg-map-route`
pub const MAP_ROUTE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea57}');
/// `fg-road-map`
pub const ROAD_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ea58}');
/// `fg-cadastre-map`
pub const CADASTRE_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ea59}');
/// `fg-landcover-map`
pub const LANDCOVER_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ea5a}');
/// `fg-bus-map`
pub const BUS_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ea5b}');
/// `fg-contour-map`
pub const CONTOUR_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ea5c}');
/// `fg-hydro-map`
pub const HYDRO_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ea5d}');
/// `fg-world-map`
pub const WORLD_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ea68}');
/// `fg-pirate-map`
pub const PIRATE_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ea6b}');
/// `fg-story-map`
pub const STORY_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ea6d}');
/// `fg-map-book`
pub const MAP_BOOK: Icon = Icon::font(FontId::Icon(GIS), '\u{ea7a}');
/// `fg-map-legend`
pub const MAP_LEGEND: Icon = Icon::font(FontId::Icon(GIS), '\u{ea85}');
/// `fg-map-legend-o`
pub const MAP_LEGEND_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea86}');
/// `fg-map-options`
pub const MAP_OPTIONS: Icon = Icon::font(FontId::Icon(GIS), '\u{ea94}');
/// `fg-map-options-alt`
pub const MAP_OPTIONS_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea95}');
/// `fg-map-print`
pub const MAP_PRINT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea96}');
/// `fg-world-map-alt-o`
pub const WORLD_MAP_ALT_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eab1}');
/// `fg-flow-map`
pub const FLOW_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{eab2}');
/// `fg-map-stat`
pub const MAP_STAT: Icon = Icon::font(FontId::Icon(GIS), '\u{eab3}');
/// `fg-statistic-map`
pub const STATISTIC_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{eab4}');
/// `fg-voronoi-map`
pub const VORONOI_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{eab7}');
/// `fg-triangle-map`
pub const TRIANGLE_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{eab8}');
/// `fg-phone-map`
pub const PHONE_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{eab9}');
/// `fg-hex-map`
pub const HEX_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{eaba}');
/// `fg-map-bookmark`
pub const MAP_BOOKMARK: Icon = Icon::font(FontId::Icon(GIS), '\u{eabd}');
/// `fg-map-tag`
pub const MAP_TAG: Icon = Icon::font(FontId::Icon(GIS), '\u{eabf}');
/// `fg-map-tags`
pub const MAP_TAGS: Icon = Icon::font(FontId::Icon(GIS), '\u{eac0}');
/// `fg-compare-map`
pub const COMPARE_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ead8}');
/// `fg-swipe-map-v`
pub const SWIPE_MAP_V: Icon = Icon::font(FontId::Icon(GIS), '\u{ead9}');
/// `fg-swipe-map-h`
pub const SWIPE_MAP_H: Icon = Icon::font(FontId::Icon(GIS), '\u{eada}');
/// `fg-magnify-map`
pub const MAGNIFY_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{eadb}');
/// `fg-map-share`
pub const MAP_SHARE: Icon = Icon::font(FontId::Icon(GIS), '\u{eae0}');
/// `fg-map-send`
pub const MAP_SEND: Icon = Icon::font(FontId::Icon(GIS), '\u{eae1}');
/// `fg-map-share-alt`
pub const MAP_SHARE_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{eae2}');
/// `fg-map-add`
pub const MAP_ADD: Icon = Icon::font(FontId::Icon(GIS), '\u{eae9}');
/// `fg-map-rm`
pub const MAP_RM: Icon = Icon::font(FontId::Icon(GIS), '\u{eaea}');
/// `fg-map-time`
pub const MAP_TIME: Icon = Icon::font(FontId::Icon(GIS), '\u{eaee}');
/// `fg-time-map`
pub const TIME_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{eaef}');
/// `fg-map-play`
pub const MAP_PLAY: Icon = Icon::font(FontId::Icon(GIS), '\u{eaf5}');
/// `fg-map-star`
pub const MAP_STAR: Icon = Icon::font(FontId::Icon(GIS), '\u{eaf6}');
/// `fg-map-favorite`
pub const MAP_FAVORITE: Icon = Icon::font(FontId::Icon(GIS), '\u{eaf7}');
/// `fg-map-smiley`
pub const MAP_SMILEY: Icon = Icon::font(FontId::Icon(GIS), '\u{eb00}');
/// `fg-map-control`
pub const MAP_CONTROL: Icon = Icon::font(FontId::Icon(GIS), '\u{eb02}');
/// `fg-map-lock`
pub const MAP_LOCK: Icon = Icon::font(FontId::Icon(GIS), '\u{eb04}');
/// `fg-map-unlock`
pub const MAP_UNLOCK: Icon = Icon::font(FontId::Icon(GIS), '\u{eb05}');
/// `fg-weather-map`
pub const WEATHER_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{eb0b}');
/// `fg-story-map-o`
pub const STORY_MAP_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb2a}');
/// `fg-story-maps`
pub const STORY_MAPS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb2b}');
/// `fg-map-edit`
pub const MAP_EDIT: Icon = Icon::font(FontId::Icon(GIS), '\u{eb2c}');
/// `fg-height-map`
pub const HEIGHT_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{eb40}');
/// `fg-map-user`
pub const MAP_USER: Icon = Icon::font(FontId::Icon(GIS), '\u{eb4b}');
/// `fg-map-users`
pub const MAP_USERS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb4c}');
/// `fg-earth`
pub const EARTH: Icon = Icon::font(FontId::Icon(GIS), '\u{ea22}');
/// `fg-earth-euro-africa`
pub const EARTH_EURO_AFRICA: Icon = Icon::font(FontId::Icon(GIS), '\u{ea23}');
/// `fg-earth-atlantic`
pub const EARTH_ATLANTIC: Icon = Icon::font(FontId::Icon(GIS), '\u{ea24}');
/// `fg-earth-america`
pub const EARTH_AMERICA: Icon = Icon::font(FontId::Icon(GIS), '\u{ea25}');
/// `fg-earth-pacific`
pub const EARTH_PACIFIC: Icon = Icon::font(FontId::Icon(GIS), '\u{ea26}');
/// `fg-earth-australia`
pub const EARTH_AUSTRALIA: Icon = Icon::font(FontId::Icon(GIS), '\u{ea27}');
/// `fg-earth-asia`
pub const EARTH_ASIA: Icon = Icon::font(FontId::Icon(GIS), '\u{ea28}');
/// `fg-earth-north`
pub const EARTH_NORTH: Icon = Icon::font(FontId::Icon(GIS), '\u{ea29}');
/// `fg-earth-south`
pub const EARTH_SOUTH: Icon = Icon::font(FontId::Icon(GIS), '\u{ea2a}');
/// `fg-earth-o`
pub const EARTH_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea2b}');
/// `fg-earth-euro-africa-o`
pub const EARTH_EURO_AFRICA_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea2c}');
/// `fg-earth-atlantic-o`
pub const EARTH_ATLANTIC_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea2d}');
/// `fg-earth-america-o`
pub const EARTH_AMERICA_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea2e}');
/// `fg-earth-pacific-o`
pub const EARTH_PACIFIC_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea2f}');
/// `fg-earth-australia-o`
pub const EARTH_AUSTRALIA_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea30}');
/// `fg-earth-asia-o`
pub const EARTH_ASIA_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea31}');
/// `fg-earth-north-o`
pub const EARTH_NORTH_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea32}');
/// `fg-earth-south-o`
pub const EARTH_SOUTH_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea33}');
/// `fg-globe`
pub const GLOBE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea36}');
/// `fg-globe-o`
pub const GLOBE_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea37}');
/// `fg-globe-alt`
pub const GLOBE_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea38}');
/// `fg-globe-alt-o`
pub const GLOBE_ALT_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea39}');
/// `fg-globe-poi`
pub const GLOBE_POI: Icon = Icon::font(FontId::Icon(GIS), '\u{ea82}');
/// `fg-network`
pub const NETWORK: Icon = Icon::font(FontId::Icon(GIS), '\u{eabb}');
/// `fg-network-o`
pub const NETWORK_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eabc}');
/// `fg-tag`
pub const TAG: Icon = Icon::font(FontId::Icon(GIS), '\u{eac1}');
/// `fg-tag-o`
pub const TAG_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eac2}');
/// `fg-tags`
pub const TAGS: Icon = Icon::font(FontId::Icon(GIS), '\u{eac3}');
/// `fg-tags-o`
pub const TAGS_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eac4}');
/// `fg-earth-gear`
pub const EARTH_GEAR: Icon = Icon::font(FontId::Icon(GIS), '\u{ead5}');
/// `fg-globe-earth`
pub const GLOBE_EARTH: Icon = Icon::font(FontId::Icon(GIS), '\u{eaf8}');
/// `fg-globe-earth-alt`
pub const GLOBE_EARTH_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{eaf9}');
/// `fg-globe-favorite`
pub const GLOBE_FAVORITE: Icon = Icon::font(FontId::Icon(GIS), '\u{eafb}');
/// `fg-globe-options`
pub const GLOBE_OPTIONS: Icon = Icon::font(FontId::Icon(GIS), '\u{eafc}');
/// `fg-globe-share`
pub const GLOBE_SHARE: Icon = Icon::font(FontId::Icon(GIS), '\u{eafd}');
/// `fg-globe-star`
pub const GLOBE_STAR: Icon = Icon::font(FontId::Icon(GIS), '\u{eafe}');
/// `fg-globe-smiley`
pub const GLOBE_SMILEY: Icon = Icon::font(FontId::Icon(GIS), '\u{eaff}');
/// `fg-globe-user`
pub const GLOBE_USER: Icon = Icon::font(FontId::Icon(GIS), '\u{eb0c}');
/// `fg-globe-users`
pub const GLOBE_USERS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb0d}');
/// `fg-globe-shield`
pub const GLOBE_SHIELD: Icon = Icon::font(FontId::Icon(GIS), '\u{eb0e}');
/// `fg-earth-network`
pub const EARTH_NETWORK: Icon = Icon::font(FontId::Icon(GIS), '\u{eb0f}');
/// `fg-earth-network-o`
pub const EARTH_NETWORK_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb10}');
/// `fg-globe-gear`
pub const GLOBE_GEAR: Icon = Icon::font(FontId::Icon(GIS), '\u{eb11}');
/// `fg-point`
pub const POINT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea01}');
/// `fg-polyline-pt`
pub const POLYLINE_PT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea02}');
/// `fg-polygon-pt`
pub const POLYGON_PT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea03}');
/// `fg-polygon-hole-pt`
pub const POLYGON_HOLE_PT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea04}');
/// `fg-rectangle-pt`
pub const RECTANGLE_PT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea05}');
/// `fg-square-pt`
pub const SQUARE_PT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea06}');
/// `fg-circle-o`
pub const CIRCLE_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea07}');
/// `fg-polyline`
pub const POLYLINE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea09}');
/// `fg-polygon-o`
pub const POLYGON_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea0a}');
/// `fg-polygon-hole-o`
pub const POLYGON_HOLE_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea0b}');
/// `fg-rectangle-o`
pub const RECTANGLE_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea0c}');
/// `fg-square-o`
pub const SQUARE_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea0d}');
/// `fg-polygon-hole`
pub const POLYGON_HOLE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea0e}');
/// `fg-polygon`
pub const POLYGON: Icon = Icon::font(FontId::Icon(GIS), '\u{ea0f}');
/// `fg-rectangle`
pub const RECTANGLE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea10}');
/// `fg-square`
pub const SQUARE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea11}');
/// `fg-circle`
pub const CIRCLE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea12}');
/// `fg-multipoint`
pub const MULTIPOINT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea52}');
/// `fg-bbox-alt`
pub const BBOX_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{eaa9}');
/// `fg-extent-alt`
pub const EXTENT_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{eaaa}');
/// `fg-bbox`
pub const BBOX: Icon = Icon::font(FontId::Icon(GIS), '\u{eaab}');
/// `fg-extent`
pub const EXTENT: Icon = Icon::font(FontId::Icon(GIS), '\u{eaac}');
/// `fg-map-extent`
pub const MAP_EXTENT: Icon = Icon::font(FontId::Icon(GIS), '\u{eab0}');
/// `fg-regular-shape-pt`
pub const REGULAR_SHAPE_PT: Icon = Icon::font(FontId::Icon(GIS), '\u{eaeb}');
/// `fg-regular-shape-o`
pub const REGULAR_SHAPE_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eaec}');
/// `fg-regular-shape`
pub const REGULAR_SHAPE: Icon = Icon::font(FontId::Icon(GIS), '\u{eaed}');
/// `fg-measure`
pub const MEASURE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea08}');
/// `fg-measure-line`
pub const MEASURE_LINE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea13}');
/// `fg-measure-area`
pub const MEASURE_AREA: Icon = Icon::font(FontId::Icon(GIS), '\u{ea14}');
/// `fg-measure-area-alt`
pub const MEASURE_AREA_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea15}');
/// `fg-scale`
pub const SCALE: Icon = Icon::font(FontId::Icon(GIS), '\u{eb01}');
/// `fg-azimuth`
pub const AZIMUTH: Icon = Icon::font(FontId::Icon(GIS), '\u{eb53}');
/// `fg-layer`
pub const LAYER: Icon = Icon::font(FontId::Icon(GIS), '\u{ea41}');
/// `fg-layer-o`
pub const LAYER_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea42}');
/// `fg-layers`
pub const LAYERS: Icon = Icon::font(FontId::Icon(GIS), '\u{ea43}');
/// `fg-layers-o`
pub const LAYERS_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea44}');
/// `fg-layer-up`
pub const LAYER_UP: Icon = Icon::font(FontId::Icon(GIS), '\u{ea45}');
/// `fg-layer-down`
pub const LAYER_DOWN: Icon = Icon::font(FontId::Icon(GIS), '\u{ea46}');
/// `fg-layer-alt`
pub const LAYER_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea47}');
/// `fg-layer-alt-o`
pub const LAYER_ALT_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea48}');
/// `fg-layer-stack`
pub const LAYER_STACK: Icon = Icon::font(FontId::Icon(GIS), '\u{ea49}');
/// `fg-layer-stack-o`
pub const LAYER_STACK_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea4a}');
/// `fg-layer-add`
pub const LAYER_ADD: Icon = Icon::font(FontId::Icon(GIS), '\u{ea4b}');
/// `fg-layer-add-o`
pub const LAYER_ADD_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea4c}');
/// `fg-layer-rm`
pub const LAYER_RM: Icon = Icon::font(FontId::Icon(GIS), '\u{ea4d}');
/// `fg-layer-rm-o`
pub const LAYER_RM_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea4e}');
/// `fg-layer-poi`
pub const LAYER_POI: Icon = Icon::font(FontId::Icon(GIS), '\u{ea6a}');
/// `fg-layer-download`
pub const LAYER_DOWNLOAD: Icon = Icon::font(FontId::Icon(GIS), '\u{ea97}');
/// `fg-layer-upload`
pub const LAYER_UPLOAD: Icon = Icon::font(FontId::Icon(GIS), '\u{ea98}');
/// `fg-layer-road`
pub const LAYER_ROAD: Icon = Icon::font(FontId::Icon(GIS), '\u{eaf0}');
/// `fg-layer-hydro`
pub const LAYER_HYDRO: Icon = Icon::font(FontId::Icon(GIS), '\u{eaf1}');
/// `fg-layer-landcover`
pub const LAYER_LANDCOVER: Icon = Icon::font(FontId::Icon(GIS), '\u{eaf2}');
/// `fg-layer-contour`
pub const LAYER_CONTOUR: Icon = Icon::font(FontId::Icon(GIS), '\u{eaf3}');
/// `fg-layer-stat`
pub const LAYER_STAT: Icon = Icon::font(FontId::Icon(GIS), '\u{eaf4}');
/// `fg-layer-stat-alt`
pub const LAYER_STAT_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{eb29}');
/// `fg-layer-edit`
pub const LAYER_EDIT: Icon = Icon::font(FontId::Icon(GIS), '\u{eb2d}');
/// `fg-layer-alt-edit`
pub const LAYER_ALT_EDIT: Icon = Icon::font(FontId::Icon(GIS), '\u{eb2e}');
/// `fg-layer-height`
pub const LAYER_HEIGHT: Icon = Icon::font(FontId::Icon(GIS), '\u{eb41}');
/// `fg-layer-2-add-o`
pub const LAYER_2_ADD_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb46}');
/// `fg-layer-2-rm-o`
pub const LAYER_2_RM_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb47}');
/// `fg-layer-alt-add-o`
pub const LAYER_ALT_ADD_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb48}');
/// `fg-layer-alt-rm-o`
pub const LAYER_ALT_RM_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb49}');
/// `fg-layer-alt-x-o`
pub const LAYER_ALT_X_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb4a}');
/// `fg-layers-poi`
pub const LAYERS_POI: Icon = Icon::font(FontId::Icon(GIS), '\u{eb4f}');
/// `fg-layer-alt-poi`
pub const LAYER_ALT_POI: Icon = Icon::font(FontId::Icon(GIS), '\u{eb50}');
/// `fg-mosaic`
pub const MOSAIC: Icon = Icon::font(FontId::Icon(GIS), '\u{eb6c}');
/// `fg-pyramid`
pub const PYRAMID: Icon = Icon::font(FontId::Icon(GIS), '\u{eb6d}');
/// `fg-help-larrow`
pub const HELP_LARROW: Icon = Icon::font(FontId::Icon(GIS), '\u{ea3e}');
/// `fg-help-rarrow`
pub const HELP_RARROW: Icon = Icon::font(FontId::Icon(GIS), '\u{ea3f}');
/// `fg-home`
pub const HOME: Icon = Icon::font(FontId::Icon(GIS), '\u{eb14}');
/// `fg-satellite`
pub const SATELLITE: Icon = Icon::font(FontId::Icon(GIS), '\u{eb3a}');
/// `fg-satellite-earth`
pub const SATELLITE_EARTH: Icon = Icon::font(FontId::Icon(GIS), '\u{eb3b}');
/// `fg-drone`
pub const DRONE: Icon = Icon::font(FontId::Icon(GIS), '\u{eb3f}');
/// `fg-poi`
pub const POI: Icon = Icon::font(FontId::Icon(GIS), '\u{ea16}');
/// `fg-poi-o`
pub const POI_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea17}');
/// `fg-poi-alt`
pub const POI_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea18}');
/// `fg-poi-alt-o`
pub const POI_ALT_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea19}');
/// `fg-pin`
pub const PIN: Icon = Icon::font(FontId::Icon(GIS), '\u{ea1a}');
/// `fg-pushpin`
pub const PUSHPIN: Icon = Icon::font(FontId::Icon(GIS), '\u{ea1b}');
/// `fg-pois`
pub const POIS: Icon = Icon::font(FontId::Icon(GIS), '\u{ea1c}');
/// `fg-pois-o`
pub const POIS_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea1d}');
/// `fg-poi-favorite`
pub const POI_FAVORITE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea1e}');
/// `fg-poi-favorite-o`
pub const POI_FAVORITE_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea1f}');
/// `fg-poi-home`
pub const POI_HOME: Icon = Icon::font(FontId::Icon(GIS), '\u{ea20}');
/// `fg-poi-home-o`
pub const POI_HOME_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea21}');
/// `fg-poi-earth`
pub const POI_EARTH: Icon = Icon::font(FontId::Icon(GIS), '\u{ea34}');
/// `fg-pin-earth`
pub const PIN_EARTH: Icon = Icon::font(FontId::Icon(GIS), '\u{ea35}');
/// `fg-pirate-poi`
pub const PIRATE_POI: Icon = Icon::font(FontId::Icon(GIS), '\u{ea6c}');
/// `fg-location-poi`
pub const LOCATION_POI: Icon = Icon::font(FontId::Icon(GIS), '\u{ea83}');
/// `fg-location-poi-o`
pub const LOCATION_POI_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea84}');
/// `fg-bookmark-poi`
pub const BOOKMARK_POI: Icon = Icon::font(FontId::Icon(GIS), '\u{eabe}');
/// `fg-bookmark-poi-b`
pub const BOOKMARK_POI_B: Icon = Icon::font(FontId::Icon(GIS), '\u{eacf}');
/// `fg-poi-map`
pub const POI_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ead6}');
/// `fg-poi-map-o`
pub const POI_MAP_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ead7}');
/// `fg-location-man`
pub const LOCATION_MAN: Icon = Icon::font(FontId::Icon(GIS), '\u{eb15}');
/// `fg-location-man-alt`
pub const LOCATION_MAN_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{eb16}');
/// `fg-poi-info`
pub const POI_INFO: Icon = Icon::font(FontId::Icon(GIS), '\u{eb1c}');
/// `fg-poi-info-o`
pub const POI_INFO_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb1d}');
/// `fg-position`
pub const POSITION: Icon = Icon::font(FontId::Icon(GIS), '\u{eb22}');
/// `fg-position-o`
pub const POSITION_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb23}');
/// `fg-position-man`
pub const POSITION_MAN: Icon = Icon::font(FontId::Icon(GIS), '\u{eb24}');
/// `fg-poi-slash`
pub const POI_SLASH: Icon = Icon::font(FontId::Icon(GIS), '\u{eb4d}');
/// `fg-poi-slash-o`
pub const POI_SLASH_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb4e}');
/// `fg-proj-conic`
pub const PROJ_CONIC: Icon = Icon::font(FontId::Icon(GIS), '\u{eb67}');
/// `fg-proj-geo`
pub const PROJ_GEO: Icon = Icon::font(FontId::Icon(GIS), '\u{eb68}');
/// `fg-proj-square`
pub const PROJ_SQUARE: Icon = Icon::font(FontId::Icon(GIS), '\u{eb69}');
/// `fg-proj-stereo`
pub const PROJ_STEREO: Icon = Icon::font(FontId::Icon(GIS), '\u{eb6a}');
/// `fg-proj-utm`
pub const PROJ_UTM: Icon = Icon::font(FontId::Icon(GIS), '\u{eb6b}');
/// `fg-route`
pub const ROUTE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea7b}');
/// `fg-route-start`
pub const ROUTE_START: Icon = Icon::font(FontId::Icon(GIS), '\u{ea7c}');
/// `fg-route-end`
pub const ROUTE_END: Icon = Icon::font(FontId::Icon(GIS), '\u{ea7d}');
/// `fg-car`
pub const CAR: Icon = Icon::font(FontId::Icon(GIS), '\u{ea7e}');
/// `fg-bicycle`
pub const BICYCLE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea7f}');
/// `fg-pedestrian`
pub const PEDESTRIAN: Icon = Icon::font(FontId::Icon(GIS), '\u{ea80}');
/// `fg-hiker`
pub const HIKER: Icon = Icon::font(FontId::Icon(GIS), '\u{ea81}');
/// `fg-location-arrow`
pub const LOCATION_ARROW: Icon = Icon::font(FontId::Icon(GIS), '\u{ea87}');
/// `fg-location-arrow-o`
pub const LOCATION_ARROW_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ea88}');
/// `fg-location`
pub const LOCATION: Icon = Icon::font(FontId::Icon(GIS), '\u{ea89}');
/// `fg-location-on`
pub const LOCATION_ON: Icon = Icon::font(FontId::Icon(GIS), '\u{ea8a}');
/// `fg-direct`
pub const DIRECT: Icon = Icon::font(FontId::Icon(GIS), '\u{ea8d}');
/// `fg-revers`
pub const REVERS: Icon = Icon::font(FontId::Icon(GIS), '\u{ea8e}');
/// `fg-timer`
pub const TIMER: Icon = Icon::font(FontId::Icon(GIS), '\u{ea8f}');
/// `fg-signpost`
pub const SIGNPOST: Icon = Icon::font(FontId::Icon(GIS), '\u{eab5}');
/// `fg-direction`
pub const DIRECTION: Icon = Icon::font(FontId::Icon(GIS), '\u{eab6}');
/// `fg-flag`
pub const FLAG: Icon = Icon::font(FontId::Icon(GIS), '\u{eac5}');
/// `fg-flag-o`
pub const FLAG_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eac6}');
/// `fg-flag-start`
pub const FLAG_START: Icon = Icon::font(FontId::Icon(GIS), '\u{eac7}');
/// `fg-flag-start-o`
pub const FLAG_START_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eac8}');
/// `fg-flag-finish`
pub const FLAG_FINISH: Icon = Icon::font(FontId::Icon(GIS), '\u{eac9}');
/// `fg-flag-b`
pub const FLAG_B: Icon = Icon::font(FontId::Icon(GIS), '\u{eaca}');
/// `fg-flab-b-o`
pub const FLAB_B_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eacb}');
/// `fg-flag-start-b`
pub const FLAG_START_B: Icon = Icon::font(FontId::Icon(GIS), '\u{eacc}');
/// `fg-flag-start-b-o`
pub const FLAG_START_B_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eacd}');
/// `fg-flag-finish-b-o`
pub const FLAG_FINISH_B_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eace}');
/// `fg-start`
pub const START: Icon = Icon::font(FontId::Icon(GIS), '\u{ead0}');
/// `fg-start-o`
pub const START_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ead1}');
/// `fg-step`
pub const STEP: Icon = Icon::font(FontId::Icon(GIS), '\u{ead2}');
/// `fg-step-o`
pub const STEP_O: Icon = Icon::font(FontId::Icon(GIS), '\u{ead3}');
/// `fg-finish`
pub const FINISH: Icon = Icon::font(FontId::Icon(GIS), '\u{ead4}');
/// `fg-directions`
pub const DIRECTIONS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb03}');
/// `fg-phone-route`
pub const PHONE_ROUTE: Icon = Icon::font(FontId::Icon(GIS), '\u{eb08}');
/// `fg-phone-route-alt`
pub const PHONE_ROUTE_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{eb09}');
/// `fg-phone-route-alt-r`
pub const PHONE_ROUTE_ALT_R: Icon = Icon::font(FontId::Icon(GIS), '\u{eb0a}');
/// `fg-map-search`
pub const MAP_SEARCH: Icon = Icon::font(FontId::Icon(GIS), '\u{ea5e}');
/// `fg-search-map`
pub const SEARCH_MAP: Icon = Icon::font(FontId::Icon(GIS), '\u{ea5f}');
/// `fg-search-poi`
pub const SEARCH_POI: Icon = Icon::font(FontId::Icon(GIS), '\u{ea60}');
/// `fg-search-globe`
pub const SEARCH_GLOBE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea61}');
/// `fg-search-home`
pub const SEARCH_HOME: Icon = Icon::font(FontId::Icon(GIS), '\u{ea62}');
/// `fg-search-address`
pub const SEARCH_ADDRESS: Icon = Icon::font(FontId::Icon(GIS), '\u{ea63}');
/// `fg-search-attribtues`
pub const SEARCH_ATTRIBTUES: Icon = Icon::font(FontId::Icon(GIS), '\u{ea64}');
/// `fg-search-propertie`
pub const SEARCH_PROPERTIE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea65}');
/// `fg-search-feature`
pub const SEARCH_FEATURE: Icon = Icon::font(FontId::Icon(GIS), '\u{ea66}');
/// `fg-search-layer`
pub const SEARCH_LAYER: Icon = Icon::font(FontId::Icon(GIS), '\u{ea67}');
/// `fg-search-country`
pub const SEARCH_COUNTRY: Icon = Icon::font(FontId::Icon(GIS), '\u{ea69}');
/// `fg-search-globe-alt`
pub const SEARCH_GLOBE_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{eafa}');
/// `fg-search-coord`
pub const SEARCH_COORD: Icon = Icon::font(FontId::Icon(GIS), '\u{eb12}');
/// `fg-search-data`
pub const SEARCH_DATA: Icon = Icon::font(FontId::Icon(GIS), '\u{eb13}');
/// `fg-zoom-in`
pub const ZOOM_IN: Icon = Icon::font(FontId::Icon(GIS), '\u{eaa5}');
/// `fg-zoom-out`
pub const ZOOM_OUT: Icon = Icon::font(FontId::Icon(GIS), '\u{eaa6}');
/// `fg-full-screen`
pub const FULL_SCREEN: Icon = Icon::font(FontId::Icon(GIS), '\u{eaa7}');
/// `fg-screen-dub`
pub const SCREEN_DUB: Icon = Icon::font(FontId::Icon(GIS), '\u{eadc}');
/// `fg-screen-split-h`
pub const SCREEN_SPLIT_H: Icon = Icon::font(FontId::Icon(GIS), '\u{eadd}');
/// `fg-screen-split-v`
pub const SCREEN_SPLIT_V: Icon = Icon::font(FontId::Icon(GIS), '\u{eade}');
/// `fg-screen-mag`
pub const SCREEN_MAG: Icon = Icon::font(FontId::Icon(GIS), '\u{eadf}');
/// `fg-coord-system`
pub const COORD_SYSTEM: Icon = Icon::font(FontId::Icon(GIS), '\u{eb1a}');
/// `fg-coord-system-3d`
pub const COORD_SYSTEM_3D: Icon = Icon::font(FontId::Icon(GIS), '\u{eb1b}');
/// `fg-coord-system-alt`
pub const COORD_SYSTEM_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{eb1e}');
/// `fg-coord-system-3d-alt`
pub const COORD_SYSTEM_3D_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{eb1f}');
/// `fg-grid`
pub const GRID: Icon = Icon::font(FontId::Icon(GIS), '\u{eb20}');
/// `fg-cube-3d`
pub const CUBE_3D: Icon = Icon::font(FontId::Icon(GIS), '\u{eb21}');
/// `fg-coord-grid`
pub const COORD_GRID: Icon = Icon::font(FontId::Icon(GIS), '\u{eb25}');
/// `fg-photogrammetry`
pub const PHOTOGRAMMETRY: Icon = Icon::font(FontId::Icon(GIS), '\u{eb3c}');
/// `fg-360`
pub const _360: Icon = Icon::font(FontId::Icon(GIS), '\u{eb3d}');
/// `fg-topography`
pub const TOPOGRAPHY: Icon = Icon::font(FontId::Icon(GIS), '\u{eb3e}');
/// `fg-gnss`
pub const GNSS: Icon = Icon::font(FontId::Icon(GIS), '\u{eb42}');
/// `fg-gnss-antenna`
pub const GNSS_ANTENNA: Icon = Icon::font(FontId::Icon(GIS), '\u{eb43}');
/// `fg-tacheometer`
pub const TACHEOMETER: Icon = Icon::font(FontId::Icon(GIS), '\u{eb44}');
/// `fg-theodolite`
pub const THEODOLITE: Icon = Icon::font(FontId::Icon(GIS), '\u{eb45}');
/// `fg-profile`
pub const PROFILE: Icon = Icon::font(FontId::Icon(GIS), '\u{eb51}');
/// `fg-profile-o`
pub const PROFILE_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb52}');
/// `fg-screen-dub1`
pub const SCREEN_DUB1: Icon = Icon::font(FontId::Icon(GIS), '\u{eb54}');
/// `fg-screen-dub2`
pub const SCREEN_DUB2: Icon = Icon::font(FontId::Icon(GIS), '\u{eb55}');
/// `fg-screen-dub-o`
pub const SCREEN_DUB_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb56}');
/// `fg-screen-mag-o`
pub const SCREEN_MAG_O: Icon = Icon::font(FontId::Icon(GIS), '\u{eb57}');
/// `fg-screen-mag-alt`
pub const SCREEN_MAG_ALT: Icon = Icon::font(FontId::Icon(GIS), '\u{eb58}');
/// `fg-polygon-o-dash`
pub const POLYGON_O_DASH: Icon = Icon::font(FontId::Icon(GIS), '\u{eb6e}');
/// `fg-polyline-dash`
pub const POLYLINE_DASH: Icon = Icon::font(FontId::Icon(GIS), '\u{eb6f}');
