#![deny(unsafe_code)]

//! Typed metadata contracts for Qbit features.

use core::fmt;
use core::str::FromStr;

/// A validated, namespaced identifier for a Qbit feature.
///
/// Valid identifiers have the form `qbit.<slug>`, where the slug starts with
/// an ASCII letter and contains only lowercase ASCII letters, digits, and
/// hyphens. It cannot end with a hyphen.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FeatureId(String);

impl FeatureId {
    pub const PREFIX: &'static str = "qbit.";

    pub fn new(value: impl AsRef<str>) -> Result<Self, FeatureIdError> {
        let value = value.as_ref();
        let Some(slug) = value.strip_prefix(Self::PREFIX) else {
            return Err(FeatureIdError::MissingPrefix);
        };

        if slug.is_empty() {
            return Err(FeatureIdError::EmptySlug);
        }

        let bytes = slug.as_bytes();
        if !bytes[0].is_ascii_lowercase() {
            return Err(FeatureIdError::InvalidStart);
        }
        if bytes.last() == Some(&b'-') {
            return Err(FeatureIdError::TrailingHyphen);
        }
        if bytes
            .iter()
            .any(|byte| !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && *byte != b'-')
        {
            return Err(FeatureIdError::InvalidCharacter);
        }

        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for FeatureId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for FeatureId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for FeatureId {
    type Err = FeatureIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

/// Reasons a feature identifier failed validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeatureIdError {
    MissingPrefix,
    EmptySlug,
    InvalidStart,
    TrailingHyphen,
    InvalidCharacter,
}

impl fmt::Display for FeatureIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingPrefix => "feature id must start with `qbit.`",
            Self::EmptySlug => "feature id slug cannot be empty",
            Self::InvalidStart => "feature id slug must start with a lowercase ASCII letter",
            Self::TrailingHyphen => "feature id slug cannot end with a hyphen",
            Self::InvalidCharacter => {
                "feature id slug may contain only lowercase ASCII letters, digits, and hyphens"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for FeatureIdError {}

/// Operating systems on which a feature may be supported.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SupportedPlatform {
    Windows,
    MacOs,
    Linux,
}

/// The execution environment a feature expects.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeMode {
    EmbeddedBackground,
    EmbeddedOnDemand,
    IsolatedWorker,
}

/// When a feature should start.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StartupPolicy {
    Manual,
    OnApplicationStart,
}

/// The observable lifecycle state of a feature.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FeatureLifecycleState {
    Unavailable,
    Disabled,
    Starting,
    Running,
    Degraded,
    Stopping,
    Failed,
}

/// A declarative capability required by a feature.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CapabilityRequirement {
    pub name: &'static str,
    pub required: bool,
}

/// A declarative storage need for a feature.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StorageRequirement {
    pub class: StorageClass,
    pub required: bool,
    pub purpose: &'static str,
}

/// The architecture-defined class of storage a feature may require.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StorageClass {
    CoreRelationalState,
    FeatureRelationalState,
    FeatureBlobData,
    Cache,
    Secrets,
}

/// Version metadata for a feature implementation and its persisted schema.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FeatureVersion {
    pub implementation: &'static str,
    pub schema: Option<u32>,
}

/// The degree to which a feature needs background execution.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BackgroundRequirement {
    None,
    OnDemand,
    Continuous,
}

/// The kind of contract a feature dependency represents.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FeatureDependencyKind {
    CoreCapability,
    FeatureContract,
}

/// A contract or capability required by a feature.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FeatureDependency {
    pub kind: FeatureDependencyKind,
    pub id: &'static str,
}

/// UI integration metadata for a feature.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FeatureUiMetadata {
    pub route: &'static str,
}

/// Diagnostics integration metadata for a feature.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FeatureDiagnosticsMetadata {
    pub component: &'static str,
}

/// Static metadata describing a feature and its runtime requirements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureDescriptor {
    pub id: FeatureId,
    pub display_name_key: &'static str,
    pub description_key: &'static str,
    pub version: Option<FeatureVersion>,
    pub supported_platforms: &'static [SupportedPlatform],
    pub runtime_mode: RuntimeMode,
    pub startup_policy: StartupPolicy,
    pub capabilities: &'static [CapabilityRequirement],
    pub storage: &'static [StorageRequirement],
    pub background_requirement: BackgroundRequirement,
    pub dependencies: &'static [FeatureDependency],
    pub ui: Option<FeatureUiMetadata>,
    pub diagnostics: FeatureDiagnosticsMetadata,
}

impl FeatureDescriptor {
    pub fn supports(&self, platform: SupportedPlatform) -> bool {
        self.supported_platforms.contains(&platform)
    }

    pub fn requires_capability(&self, name: &str) -> bool {
        self.capabilities
            .iter()
            .any(|capability| capability.required && capability.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_feature_ids() {
        assert_eq!(
            FeatureId::new("qbit.clipboard-history").unwrap().as_str(),
            "qbit.clipboard-history"
        );
        assert!(FeatureId::new("qbit.a1-b2").is_ok());
    }

    #[test]
    fn rejects_invalid_feature_ids_with_typed_errors() {
        assert_eq!(
            FeatureId::new("other.feature"),
            Err(FeatureIdError::MissingPrefix)
        );
        assert_eq!(FeatureId::new("qbit."), Err(FeatureIdError::EmptySlug));
        assert_eq!(
            FeatureId::new("qbit.1alpha"),
            Err(FeatureIdError::InvalidStart)
        );
        assert_eq!(
            FeatureId::new("qbit.alpha-"),
            Err(FeatureIdError::TrailingHyphen)
        );
        assert_eq!(
            FeatureId::new("qbit.Alpha"),
            Err(FeatureIdError::InvalidStart)
        );
        assert_eq!(
            FeatureId::new("qbit.alpha_beta"),
            Err(FeatureIdError::InvalidCharacter)
        );
    }

    #[test]
    fn descriptor_reports_platform_and_required_capabilities() {
        let descriptor = FeatureDescriptor {
            id: FeatureId::new("qbit.example").unwrap(),
            display_name_key: "features.example.display-name",
            description_key: "features.example.description",
            version: Some(FeatureVersion {
                implementation: "1.2.3",
                schema: Some(2),
            }),
            supported_platforms: &[SupportedPlatform::Windows, SupportedPlatform::Linux],
            runtime_mode: RuntimeMode::EmbeddedBackground,
            startup_policy: StartupPolicy::OnApplicationStart,
            capabilities: &[
                CapabilityRequirement {
                    name: "clipboard.read",
                    required: true,
                },
                CapabilityRequirement {
                    name: "notifications",
                    required: false,
                },
            ],
            storage: &[StorageRequirement {
                class: StorageClass::FeatureRelationalState,
                required: true,
                purpose: "settings",
            }],
            background_requirement: BackgroundRequirement::Continuous,
            dependencies: &[FeatureDependency {
                kind: FeatureDependencyKind::CoreCapability,
                id: "clipboard.read",
            }],
            ui: Some(FeatureUiMetadata {
                route: "/features/example",
            }),
            diagnostics: FeatureDiagnosticsMetadata {
                component: "feature.example",
            },
        };

        assert_eq!(descriptor.display_name_key, "features.example.display-name");
        assert_eq!(descriptor.description_key, "features.example.description");
        assert_eq!(descriptor.version.unwrap().implementation, "1.2.3");
        assert_eq!(descriptor.version.unwrap().schema, Some(2));
        assert!(descriptor.supports(SupportedPlatform::Windows));
        assert!(!descriptor.supports(SupportedPlatform::MacOs));
        assert!(descriptor.requires_capability("clipboard.read"));
        assert!(!descriptor.requires_capability("notifications"));
        assert_eq!(
            descriptor.background_requirement,
            BackgroundRequirement::Continuous
        );
        assert_eq!(
            descriptor.dependencies[0],
            FeatureDependency {
                kind: FeatureDependencyKind::CoreCapability,
                id: "clipboard.read",
            }
        );
        assert_eq!(descriptor.ui.unwrap().route, "/features/example");
        assert_eq!(descriptor.diagnostics.component, "feature.example");
    }
}
