use feature_api::{
    BackgroundRequirement, RuntimeMode, StartupPolicy, StorageClass, SupportedPlatform,
};
use proptest::prelude::*;
use qbit_hotkeys::application::{ConflictCode, analyze, analyze_candidate, compile};
use qbit_hotkeys::domain::{
    Action, Chord, DomainError, KeyIdentity, KeyToken, Mapping, MappingBehavior, MappingMetadata,
    Modifier, Scope, Trigger,
};
use uuid::Uuid;

fn chord(key: &str) -> Chord {
    Chord::new([], KeyIdentity::Logical(KeyToken::new(key).unwrap())).unwrap()
}

fn mapping(id: u128, enabled: bool, trigger: &str, action: Action) -> Mapping {
    Mapping::with_id(
        Uuid::from_u128(id),
        enabled,
        Trigger::Chord {
            chord: chord(trigger),
        },
        action,
        Scope::Global,
        MappingBehavior::default(),
        MappingMetadata::default(),
    )
}

fn four_item_permutation(mut selector: usize) -> [usize; 4] {
    let mut remaining = vec![0, 1, 2, 3];
    let first = remaining.remove(selector / 6);
    selector %= 6;
    let second = remaining.remove(selector / 2);
    selector %= 2;
    let third = remaining.remove(selector);
    [first, second, third, remaining[0]]
}

#[test]
fn domain_invariants_and_serde_validation_hold() {
    for invalid in ["", " ", "Key A", "\n", "é"] {
        assert!(matches!(
            KeyToken::new(invalid),
            Err(DomainError::InvalidKeyToken)
        ));
    }
    assert!(matches!(
        KeyToken::new("a".repeat(65)),
        Err(DomainError::KeyTokenTooLong)
    ));
    assert_ne!(
        KeyIdentity::Logical(KeyToken::new("KeyA").unwrap()),
        KeyIdentity::Physical(KeyToken::new("KeyA").unwrap())
    );
    let zero_modifier = chord("KeyA");
    assert!(zero_modifier.modifiers().is_empty());
    assert_eq!(
        Chord::new(
            [Modifier::RightCtrl, Modifier::LeftShift],
            KeyIdentity::Logical(KeyToken::new("KeyA").unwrap())
        )
        .unwrap()
        .modifiers(),
        vec![Modifier::RightCtrl, Modifier::LeftShift]
    );
    assert!(matches!(
        Chord::new(
            [Modifier::Ctrl, Modifier::Ctrl],
            KeyIdentity::Logical(KeyToken::new("KeyA").unwrap())
        ),
        Err(DomainError::DuplicateModifier(Modifier::Ctrl))
    ));
    assert!(matches!(
        Chord::new(
            [Modifier::Ctrl, Modifier::LeftCtrl],
            KeyIdentity::Logical(KeyToken::new("KeyA").unwrap())
        ),
        Err(DomainError::AmbiguousModifierFamily(_))
    ));
    let ctrl_primary =
        Chord::new([], KeyIdentity::Logical(KeyToken::new("Ctrl").unwrap())).unwrap();
    assert!(ctrl_primary.modifiers().is_empty());
    assert_eq!(
        ctrl_primary.primary(),
        &KeyIdentity::Logical(KeyToken::new("Ctrl").unwrap())
    );
    assert!(matches!(
        Chord::new(
            [Modifier::Ctrl],
            KeyIdentity::Logical(KeyToken::new("Ctrl").unwrap())
        ),
        Err(DomainError::ModifierAsPrimaryKey)
    ));
    assert!(serde_json::from_str::<MappingMetadata>(r#"{"label":""}"#).is_err());
    assert!(
        serde_json::from_str::<MappingMetadata>(&format!(r#"{{"label":"{}"}}"#, "x".repeat(121)))
            .is_err()
    );
}

#[test]
fn conflicts_are_stable_and_report_complete_sccs() {
    let first = mapping(
        1,
        true,
        "KeyA",
        Action::EmitShortcut {
            chord: chord("KeyB"),
        },
    );
    let second = mapping(
        2,
        true,
        "KeyB",
        Action::EmitShortcut {
            chord: chord("KeyC"),
        },
    );
    let third = mapping(
        3,
        true,
        "KeyC",
        Action::EmitShortcut {
            chord: chord("KeyA"),
        },
    );
    let conflicts = analyze(&[third.clone(), first.clone(), second.clone()]);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].code, ConflictCode::EmitShortcutCycle);
    assert_eq!(
        conflicts[0].mapping_ids,
        vec![first.id, second.id, third.id]
    );
    assert_eq!(conflicts, analyze(&[first, second, third]));
}

#[test]
fn disabled_and_disable_actions_do_not_create_edges() {
    let enabled = mapping(
        1,
        true,
        "KeyA",
        Action::EmitShortcut {
            chord: chord("KeyB"),
        },
    );
    let disabled = mapping(
        2,
        false,
        "KeyB",
        Action::EmitShortcut {
            chord: chord("KeyA"),
        },
    );
    let disable = mapping(3, true, "KeyB", Action::Disable);
    assert!(analyze(&[enabled.clone(), disabled]).is_empty());
    assert!(analyze(&[enabled, disable]).is_empty());
}

#[test]
fn self_maps_duplicates_candidates_and_compilation_are_blocking() {
    let self_map = mapping(
        1,
        true,
        "KeyA",
        Action::EmitShortcut {
            chord: chord("KeyA"),
        },
    );
    assert!(matches!(
        analyze(std::slice::from_ref(&self_map)).as_slice(),
        [conflict] if conflict.code == ConflictCode::DirectSelfMap
    ));
    assert!(compile(&[self_map]).is_err());

    let existing = mapping(2, true, "KeyB", Action::Disable);
    let candidate = mapping(2, false, "KeyB", Action::Disable);
    assert!(matches!(
        analyze_candidate(&[existing], &candidate).as_slice(),
        []
    ));
}

#[test]
fn duplicate_enabled_triggers_are_blocking_and_ignore_disabled_mappings() {
    let first = mapping(1, true, "KeyA", Action::Disable);
    let second = mapping(
        2,
        true,
        "KeyA",
        Action::EmitShortcut {
            chord: chord("KeyB"),
        },
    );
    let disabled = mapping(3, false, "KeyA", Action::Disable);

    assert_eq!(
        analyze(&[second.clone(), disabled.clone(), first.clone()]),
        vec![qbit_hotkeys::application::Conflict {
            code: ConflictCode::DuplicateEnabledTrigger,
            severity: qbit_hotkeys::application::ConflictSeverity::Error,
            mapping_ids: vec![first.id, second.id],
        }]
    );
    assert!(compile(&[first.clone(), second]).is_err());
    assert!(analyze(&[first, disabled]).is_empty());
}

#[test]
fn compiler_rejects_two_node_emit_shortcut_cycles() {
    let first = mapping(
        1,
        true,
        "KeyA",
        Action::EmitShortcut {
            chord: chord("KeyB"),
        },
    );
    let second = mapping(
        2,
        true,
        "KeyB",
        Action::EmitShortcut {
            chord: chord("KeyA"),
        },
    );

    assert!(matches!(
        analyze(&[first.clone(), second.clone()]).as_slice(),
        [conflict] if conflict.code == ConflictCode::EmitShortcutCycle
            && conflict.mapping_ids == vec![first.id, second.id]
    ));
    assert!(compile(&[first, second]).is_err());
}

#[test]
fn complex_emit_shortcut_scc_is_reported_once_with_sorted_ids() {
    let first = mapping(
        1,
        true,
        "KeyA",
        Action::EmitShortcut {
            chord: chord("KeyB"),
        },
    );
    let second = mapping(
        2,
        true,
        "KeyB",
        Action::EmitShortcut {
            chord: chord("KeyC"),
        },
    );
    let third = mapping(
        3,
        true,
        "KeyC",
        Action::EmitShortcut {
            chord: chord("KeyA"),
        },
    );
    let fourth = mapping(
        4,
        true,
        "KeyB",
        Action::EmitShortcut {
            chord: chord("KeyA"),
        },
    );
    // The duplicate KeyB trigger creates KeyA -> {second, fourth}, giving multiple
    // routes through one four-node SCC.
    let conflicts = analyze(&[third.clone(), first.clone(), fourth.clone(), second.clone()]);

    let cycles: Vec<_> = conflicts
        .iter()
        .filter(|conflict| conflict.code == ConflictCode::EmitShortcutCycle)
        .collect();
    assert_eq!(cycles.len(), 1);
    assert_eq!(
        cycles[0].mapping_ids,
        vec![first.id, second.id, third.id, fourth.id]
    );
}

#[test]
fn analyze_candidate_replaces_same_id_and_enables_candidate_for_conflict_checks() {
    let existing = mapping(2, true, "KeyB", Action::Disable);
    let replacement = mapping(2, false, "KeyB", Action::Disable);
    assert!(analyze_candidate(&[existing], &replacement).is_empty());

    let active = mapping(1, true, "KeyA", Action::Disable);
    let disabled_candidate = mapping(3, false, "KeyA", Action::Disable);
    assert!(matches!(
        analyze_candidate(std::slice::from_ref(&active), &disabled_candidate).as_slice(),
        [conflict] if conflict.code == ConflictCode::DuplicateEnabledTrigger
            && conflict.mapping_ids == vec![active.id, disabled_candidate.id]
    ));
}

#[test]
fn large_acyclic_chain_does_not_depend_on_call_stack_depth() {
    const MAPPING_COUNT: u128 = 4096;
    let mappings: Vec<_> = (0..MAPPING_COUNT)
        .map(|index| {
            let trigger = format!("K{index}");
            let action = if index + 1 == MAPPING_COUNT {
                Action::Disable
            } else {
                Action::EmitShortcut {
                    chord: chord(&format!("K{}", index + 1)),
                }
            };
            mapping(index + 1, true, &trigger, action)
        })
        .collect();

    assert!(analyze(&mappings).is_empty());
}

#[test]
fn representative_mapping_serde_round_trips_preserve_all_fields() {
    let physical_emit = Mapping::with_id(
        Uuid::from_u128(0x11111111111111111111111111111111),
        true,
        Trigger::Chord {
            chord: Chord::new(
                [Modifier::LeftCtrl, Modifier::RightShift],
                KeyIdentity::Physical(KeyToken::new("KeyA").unwrap()),
            )
            .unwrap(),
        },
        Action::EmitShortcut {
            chord: Chord::new(
                [Modifier::LeftAlt, Modifier::RightMeta],
                KeyIdentity::Logical(KeyToken::new("KeyB").unwrap()),
            )
            .unwrap(),
        },
        Scope::Global,
        MappingBehavior::default(),
        MappingMetadata::new(Some("Physical emit".to_owned())).unwrap(),
    );
    let disable = mapping(
        0x22222222222222222222222222222222,
        true,
        "KeyC",
        Action::Disable,
    );

    for mapping in [physical_emit, disable] {
        let encoded = serde_json::to_string(&mapping).unwrap();
        assert_eq!(serde_json::from_str::<Mapping>(&encoded).unwrap(), mapping);
    }
}

#[test]
fn descriptor_matches_hotkeys_feature_contract() {
    let descriptor = qbit_hotkeys::descriptor();
    assert_eq!(descriptor.id.as_str(), qbit_hotkeys::FEATURE_ID);
    assert!(descriptor.supports(SupportedPlatform::Windows));
    assert!(!descriptor.supports(SupportedPlatform::MacOs));
    assert!(!descriptor.supports(SupportedPlatform::Linux));
    assert_eq!(descriptor.runtime_mode, RuntimeMode::EmbeddedBackground);
    assert_eq!(descriptor.startup_policy, StartupPolicy::OnApplicationStart);
    assert_eq!(
        descriptor.background_requirement,
        BackgroundRequirement::Continuous
    );
    assert_eq!(
        descriptor.version.unwrap().implementation,
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(
        descriptor.version.unwrap().schema,
        Some(qbit_hotkeys::FEATURE_SCHEMA_VERSION)
    );
    assert!(descriptor.storage.iter().any(|storage| {
        storage.class == StorageClass::FeatureRelationalState && storage.required
    }));
    assert_eq!(descriptor.ui.unwrap().route, "/features/hotkeys");
    assert_eq!(descriptor.diagnostics.component, "feature.hotkeys");
    assert!(descriptor.requires_capability("keyboard.global-interception"));
    assert!(descriptor.requires_capability("input.emission"));
    assert!(descriptor.dependencies.is_empty());
}

#[test]
fn compiler_excludes_disabled_and_uses_chord_lookup() {
    let enabled = mapping(1, true, "KeyA", Action::Disable);
    let disabled = mapping(2, false, "KeyB", Action::Disable);
    let compiled = compile(&[enabled.clone(), disabled]).unwrap();
    assert_eq!(compiled.len(), 1);
    assert!(!compiled.is_empty());
    assert_eq!(
        compiled.lookup(&chord("KeyA")).unwrap().mapping_id,
        enabled.id
    );
    assert!(compiled.lookup(&chord("KeyB")).is_none());
}

proptest! {
    #[test]
    fn modifier_normalization_is_permutation_independent(
        families in prop::collection::vec(0usize..5, 4)
    ) {
        let family_modifiers = [
            [Modifier::Ctrl, Modifier::LeftCtrl, Modifier::RightCtrl],
            [Modifier::Shift, Modifier::LeftShift, Modifier::RightShift],
            [Modifier::Alt, Modifier::LeftAlt, Modifier::RightAlt],
            [Modifier::Meta, Modifier::LeftMeta, Modifier::RightMeta],
        ];
        let modifiers: Vec<_> = families
            .into_iter()
            .zip(family_modifiers)
            .flat_map(|(selection, modifiers)| match selection {
                0 => Vec::new(),
                1 => vec![modifiers[0]],
                2 => vec![modifiers[1]],
                3 => vec![modifiers[2]],
                4 => vec![modifiers[1], modifiers[2]],
                _ => unreachable!(),
            })
            .collect();
        let primary = KeyIdentity::Logical(KeyToken::new("KeyA").unwrap());
        let first = Chord::new(modifiers.clone(), primary.clone()).unwrap();
        let mut reversed = modifiers;
        reversed.reverse();
        prop_assert_eq!(first, Chord::new(reversed, primary).unwrap());
    }

    #[test]
    fn mapping_serde_round_trip_preserves_valid_mapping(id in any::<u128>(), enabled in any::<bool>()) {
        let mapping = mapping(id, enabled, "KeyA", Action::EmitShortcut { chord: chord("KeyB") });
        let encoded = serde_json::to_string(&mapping).unwrap();
        prop_assert_eq!(serde_json::from_str::<Mapping>(&encoded).unwrap(), mapping);
    }

    #[test]
    fn conflicts_are_invariant_under_mapping_insertion_order(selector in 0usize..24) {
        let mappings = [
            mapping(1, true, "KeyA", Action::EmitShortcut { chord: chord("KeyB") }),
            mapping(2, true, "KeyB", Action::EmitShortcut { chord: chord("KeyC") }),
            mapping(3, true, "KeyC", Action::EmitShortcut { chord: chord("KeyA") }),
            mapping(4, true, "KeyD", Action::Disable),
        ];
        let canonical = analyze(&mappings);
        let order = four_item_permutation(selector);
        let reordered: Vec<_> = order.into_iter().map(|index| mappings[index].clone()).collect();
        prop_assert_eq!(analyze(&reordered), canonical);
    }
}
