//! Ported from `packages/engine/Source/Scene/InstanceAttributeSemantic.js`.

/// Instance attribute semantic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum InstanceAttributeSemantic {
    /// Position.
    Position = 0,
    /// Rotation.
    Rotation = 1,
    /// Scale.
    Scale = 2,
    /// Translation.
    Translation = 3,
}
