//! Platform-neutral, validated Hotkeys data model.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

/// Domain construction errors. Labels are limited to 120 Unicode scalar values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainError {
    InvalidKeyToken,
    KeyTokenTooLong,
    InvalidLabel,
    LabelTooLong,
    DuplicateModifier(Modifier),
    AmbiguousModifierFamily(ModifierFamily),
    ModifierAsPrimaryKey,
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "hotkeys domain invariant failed: {self:?}")
    }
}

impl std::error::Error for DomainError {}

/// An ASCII-safe canonical token, such as `Enter`, `KeyA`, or `NumpadEnter`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct KeyToken(String);

impl KeyToken {
    pub const MAX_BYTES: usize = 64;

    pub fn new(value: impl AsRef<str>) -> Result<Self, DomainError> {
        let value = value.as_ref();
        if value.is_empty()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(DomainError::InvalidKeyToken);
        }
        if value.len() > Self::MAX_BYTES {
            return Err(DomainError::KeyTokenTooLong);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for KeyToken {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Logical and physical identities deliberately remain distinct even with equal tokens.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", content = "token", rename_all = "snake_case")]
pub enum KeyIdentity {
    Logical(KeyToken),
    Physical(KeyToken),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    Ctrl,
    LeftCtrl,
    RightCtrl,
    Shift,
    LeftShift,
    RightShift,
    Alt,
    LeftAlt,
    RightAlt,
    Meta,
    LeftMeta,
    RightMeta,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModifierFamily {
    Ctrl,
    Shift,
    Alt,
    Meta,
}

impl Modifier {
    fn family(self) -> ModifierFamily {
        match self {
            Self::Ctrl | Self::LeftCtrl | Self::RightCtrl => ModifierFamily::Ctrl,
            Self::Shift | Self::LeftShift | Self::RightShift => ModifierFamily::Shift,
            Self::Alt | Self::LeftAlt | Self::RightAlt => ModifierFamily::Alt,
            Self::Meta | Self::LeftMeta | Self::RightMeta => ModifierFamily::Meta,
        }
    }
    fn is_generic(self) -> bool {
        matches!(self, Self::Ctrl | Self::Shift | Self::Alt | Self::Meta)
    }
    fn token(self) -> &'static str {
        match self {
            Self::Ctrl => "Ctrl",
            Self::LeftCtrl => "LeftCtrl",
            Self::RightCtrl => "RightCtrl",
            Self::Shift => "Shift",
            Self::LeftShift => "LeftShift",
            Self::RightShift => "RightShift",
            Self::Alt => "Alt",
            Self::LeftAlt => "LeftAlt",
            Self::RightAlt => "RightAlt",
            Self::Meta => "Meta",
            Self::LeftMeta => "LeftMeta",
            Self::RightMeta => "RightMeta",
        }
    }
}

/// A normalized chord. Empty modifiers are valid: a future runtime may support single keys.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Chord {
    modifiers: Vec<Modifier>,
    primary: KeyIdentity,
}

impl Chord {
    pub fn new(
        modifiers: impl IntoIterator<Item = Modifier>,
        primary: KeyIdentity,
    ) -> Result<Self, DomainError> {
        let mut normalized = BTreeSet::new();
        for modifier in modifiers {
            if !normalized.insert(modifier) {
                return Err(DomainError::DuplicateModifier(modifier));
            }
        }
        for family in [
            ModifierFamily::Ctrl,
            ModifierFamily::Shift,
            ModifierFamily::Alt,
            ModifierFamily::Meta,
        ] {
            let modifiers: Vec<_> = normalized
                .iter()
                .copied()
                .filter(|modifier| modifier.family() == family)
                .collect();
            if modifiers.len() > 1 && modifiers.iter().any(|modifier| modifier.is_generic()) {
                return Err(DomainError::AmbiguousModifierFamily(family));
            }
        }
        let token = match &primary {
            KeyIdentity::Logical(token) | KeyIdentity::Physical(token) => token,
        };
        if normalized
            .iter()
            .any(|modifier| modifier.token() == token.as_str())
        {
            return Err(DomainError::ModifierAsPrimaryKey);
        }
        Ok(Self {
            modifiers: normalized.into_iter().collect(),
            primary,
        })
    }

    pub fn modifiers(&self) -> &[Modifier] {
        &self.modifiers
    }

    pub fn primary(&self) -> &KeyIdentity {
        &self.primary
    }
}

#[derive(Deserialize)]
struct ChordDto {
    modifiers: Vec<Modifier>,
    primary: KeyIdentity,
}
impl<'de> Deserialize<'de> for Chord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let dto = ChordDto::deserialize(deserializer)?;
        Self::new(dto.modifiers, dto.primary).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Trigger {
    Chord { chord: Chord },
}
impl Trigger {
    pub fn chord(&self) -> &Chord {
        match self {
            Self::Chord { chord } => chord,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    EmitShortcut { chord: Chord },
    Disable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Global,
}

/// Reserved for future behavior semantics; v1 intentionally has no policies.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MappingBehavior {}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct MappingMetadata {
    label: Option<String>,
}
impl MappingMetadata {
    pub const MAX_LABEL_SCALARS: usize = 120;
    pub fn new(label: Option<String>) -> Result<Self, DomainError> {
        if let Some(label) = &label
            && (label.is_empty() || label.chars().count() > Self::MAX_LABEL_SCALARS)
        {
            return Err(if label.is_empty() {
                DomainError::InvalidLabel
            } else {
                DomainError::LabelTooLong
            });
        }
        Ok(Self { label })
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

impl<'de> Deserialize<'de> for MappingMetadata {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct MappingMetadataDto {
            label: Option<String>,
        }

        let dto = MappingMetadataDto::deserialize(deserializer)?;
        Self::new(dto.label).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mapping {
    pub id: Uuid,
    pub enabled: bool,
    pub trigger: Trigger,
    pub action: Action,
    pub scope: Scope,
    pub behavior: MappingBehavior,
    pub metadata: MappingMetadata,
}
impl Mapping {
    pub fn new(
        enabled: bool,
        trigger: Trigger,
        action: Action,
        scope: Scope,
        behavior: MappingBehavior,
        metadata: MappingMetadata,
    ) -> Self {
        Self::with_id(
            Uuid::new_v4(),
            enabled,
            trigger,
            action,
            scope,
            behavior,
            metadata,
        )
    }
    pub fn with_id(
        id: Uuid,
        enabled: bool,
        trigger: Trigger,
        action: Action,
        scope: Scope,
        behavior: MappingBehavior,
        metadata: MappingMetadata,
    ) -> Self {
        Self {
            id,
            enabled,
            trigger,
            action,
            scope,
            behavior,
            metadata,
        }
    }
}
