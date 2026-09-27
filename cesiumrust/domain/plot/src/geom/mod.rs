//! Pure geometric algorithms (plan §6): screen-space hit testing (M3) and the
//! sampling / tessellation the face + conic primitives need (M4+).
//!
//! Everything here works in **screen pixels** (`[f64; 2]`, top-left origin) so
//! it is projection-agnostic: the bridge projects a geometry's `GeoPoint`s to
//! screen with the active camera and then calls these. That is what makes hit
//! testing "与像素尺度无关、2D/3D 同构" and unit-testable with no engine.

pub mod hit;
pub mod sample;
pub mod tessellate;
