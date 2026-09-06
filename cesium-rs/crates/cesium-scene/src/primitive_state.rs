//! Ported from `packages/engine/Source/Scene/PrimitiveState.js`.

/// The states that describe the lifecycle of a `Primitive`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PrimitiveState {
    /// The initial state of a primitive.
    Ready = 0,
    /// The process of creating the primitive geometry is ongoing.
    Creating = 1,
    /// The geometry for the primitive has been created.
    Created = 2,
    /// The asynchronous combining of geometry is ongoing.
    Combining = 3,
    /// The geometry data is in a form that can be uploaded to the GPU.
    Combined = 4,
    /// The geometry has been created and uploaded to the GPU.
    Complete = 5,
    /// The creation of the primitive failed.
    Failed = 6,
}

impl PrimitiveState {
    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Creates from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Ready),
            1 => Some(Self::Creating),
            2 => Some(Self::Created),
            3 => Some(Self::Combining),
            4 => Some(Self::Combined),
            5 => Some(Self::Complete),
            6 => Some(Self::Failed),
            _ => None,
        }
    }
}
