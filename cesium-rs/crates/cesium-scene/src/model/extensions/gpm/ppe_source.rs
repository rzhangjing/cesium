//! Ported from `packages/engine/Source/Scene/Model/Extensions/Gpm/PpeSource.js`.

/// An enum of per-point error sources.
///
/// This reflects the `ppeMetadata.source` definition of the
/// NGA_gpm_local glTF extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PpeSource {
    /// The PPE standard deviation of error in the x dimension of the MCS
    /// (sigma x). Value will be squared and used to populate the (1,1)
    /// element in the PPE covariance matrix.
    Sigx,
    /// The PPE standard deviation of error in the y dimension of the MCS
    /// (sigma y). Value will be squared and used to populate the (2,2)
    /// element in the PPE covariance matrix.
    Sigy,
    /// The PPE standard deviation of error in the z dimension of the MCS
    /// (sigma z). Value will be squared and used to populate the (3,3)
    /// element in the PPE covariance matrix.
    Sigz,
    /// The PPE variance of error in the x dimension of the MCS (sigma x2).
    /// Value will be used to populate the (1,1) element in the PPE
    /// covariance matrix.
    Varx,
    /// The PPE variance of error in the y dimension of the MCS (sigma y2).
    /// Value will be used to populate the (2,2) element in the PPE
    /// covariance matrix.
    Vary,
    /// The PPE variance of error in the z dimension of the MCS (sigma z2).
    /// Value will be used to populate the (3,3) element in the PPE
    /// covariance matrix.
    Varz,
    /// The PPE radial error in the horizontal dimension (x-y) of the MCS
    /// (sigma radial). Value will be squared and used to populate the
    /// (1,1) and (2,2) element in the PPE covariance matrix.
    Sigr,
}

impl PpeSource {
    /// Returns the string representation, mirroring the frozen string
    /// constants of the JS enum.
    ///
    /// DEVIATION: mirrors CesiumJS verbatim, where the `SIGR` constant is
    /// defined with the value `"VARZ"` (identical to `Varz`) in the
    /// original `PpeSource.js`.
    pub fn as_str(self) -> &'static str {
        match self {
            PpeSource::Sigx => "SIGX",
            PpeSource::Sigy => "SIGY",
            PpeSource::Sigz => "SIGZ",
            PpeSource::Varx => "VARX",
            PpeSource::Vary => "VARY",
            PpeSource::Varz => "VARZ",
            PpeSource::Sigr => "VARZ",
        }
    }

    /// Parses a PPE source from its glTF JSON string representation.
    ///
    /// DEVIATION: mirrors CesiumJS verbatim: `"VARZ"` maps to `Varz`
    /// (there is no way to distinguish it from `Sigr`, which the original
    /// JS defines with the same value).
    pub fn from_str(value: &str) -> Option<PpeSource> {
        match value {
            "SIGX" => Some(PpeSource::Sigx),
            "SIGY" => Some(PpeSource::Sigy),
            "SIGZ" => Some(PpeSource::Sigz),
            "VARX" => Some(PpeSource::Varx),
            "VARY" => Some(PpeSource::Vary),
            "VARZ" => Some(PpeSource::Varz),
            _ => None,
        }
    }
}
