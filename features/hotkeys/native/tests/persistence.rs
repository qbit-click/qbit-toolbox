use persistence::{DurabilityProfile, open_feature_database};
use qbit_hotkeys::domain::{
    Action, Chord, KeyIdentity, KeyToken, Mapping, MappingBehavior, MappingMetadata, Scope, Trigger,
};
use qbit_hotkeys::persistence::{
    HotkeysRepository, PAYLOAD_FORMAT_VERSION, RepositoryError, SCHEMA_VERSION,
};
use qbit_hotkeys::{FEATURE_ID, FEATURE_SCHEMA_VERSION};
use tempfile::tempdir;
use uuid::Uuid;

fn mapping(id: u128, enabled: bool, key: &str) -> Mapping {
    Mapping::with_id(
        Uuid::from_u128(id),
        enabled,
        Trigger::Chord {
            chord: Chord::new([], KeyIdentity::Logical(KeyToken::new(key).unwrap())).unwrap(),
        },
        Action::Disable,
        Scope::Global,
        MappingBehavior::default(),
        MappingMetadata::default(),
    )
}

#[test]
fn creates_the_canonical_path_and_records_v1_once() {
    let directory = tempdir().unwrap();
    let repository = HotkeysRepository::open(directory.path()).unwrap();
    assert_eq!(repository.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(SCHEMA_VERSION, FEATURE_SCHEMA_VERSION as i64);
    assert_eq!(
        repository.database_path(),
        directory.path().join("data/features/qbit.hotkeys/data.db")
    );
    drop(repository);

    let database =
        open_feature_database(directory.path(), FEATURE_ID, DurabilityProfile::Critical).unwrap();
    let connection = database.into_connection();
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn crud_enablement_ordering_and_reopen_are_durable() {
    let directory = tempdir().unwrap();
    let first = mapping(1, true, "KeyA");
    let second = mapping(2, true, "KeyB");
    {
        let mut repository = HotkeysRepository::open(directory.path()).unwrap();
        repository.insert_mapping(&second).unwrap();
        repository.insert_mapping(&first).unwrap();
        assert_eq!(
            repository
                .list_mappings()
                .unwrap()
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![second.id, first.id]
        );
        repository.set_mapping_enabled(first.id, false).unwrap();
        assert!(!repository.get_mapping(first.id).unwrap().unwrap().enabled);
        let mut replacement = second.clone();
        replacement.enabled = false;
        repository.replace_mapping(&replacement).unwrap();
        assert!(repository.delete_mapping(first.id).unwrap());
        assert!(!repository.delete_mapping(first.id).unwrap());
    }
    let repository = HotkeysRepository::open(directory.path()).unwrap();
    assert_eq!(
        repository.list_mappings().unwrap(),
        vec![Mapping {
            enabled: false,
            ..second
        }]
    );
}

#[test]
fn duplicate_insert_and_replace_missing_are_typed() {
    let directory = tempdir().unwrap();
    let existing = mapping(3, true, "KeyC");
    let mut repository = HotkeysRepository::open(directory.path()).unwrap();
    repository.insert_mapping(&existing).unwrap();
    assert!(
        matches!(repository.insert_mapping(&existing), Err(RepositoryError::DuplicateMapping { id }) if id == existing.id)
    );
    let missing = mapping(4, true, "KeyD");
    assert!(
        matches!(repository.replace_mapping(&missing), Err(RepositoryError::NotFound { id }) if id == missing.id)
    );
    assert_eq!(repository.get_mapping(missing.id).unwrap(), None);
}

#[test]
fn future_schema_is_rejected() {
    let directory = tempdir().unwrap();
    HotkeysRepository::open(directory.path()).unwrap();
    let database =
        open_feature_database(directory.path(), FEATURE_ID, DurabilityProfile::Critical).unwrap();
    let connection = database.into_connection();
    connection
        .execute(
            "INSERT INTO schema_migrations (version, name) VALUES (2, 'future')",
            [],
        )
        .unwrap();
    drop(connection);
    assert!(matches!(
        HotkeysRepository::open(directory.path()),
        Err(RepositoryError::Shared(
            persistence::PersistenceError::FutureSchema { .. }
        ))
    ));
}

#[test]
fn unsupported_and_corrupt_payloads_do_not_reset_the_database() {
    let directory = tempdir().unwrap();
    let valid = mapping(5, true, "KeyE");
    let unsupported = mapping(6, true, "KeyF");
    let corrupt = mapping(7, true, "KeyG");
    let mut repository = HotkeysRepository::open(directory.path()).unwrap();
    repository.insert_mapping(&valid).unwrap();
    repository.insert_mapping(&unsupported).unwrap();
    repository.insert_mapping(&corrupt).unwrap();
    let database_path = repository.database_path().to_owned();
    drop(repository);

    let database =
        open_feature_database(directory.path(), FEATURE_ID, DurabilityProfile::Critical).unwrap();
    let connection = database.into_connection();
    connection
        .execute(
            "UPDATE mappings SET payload_version = ?2 WHERE id = ?1",
            (unsupported.id.to_string(), PAYLOAD_FORMAT_VERSION + 1),
        )
        .unwrap();
    connection
        .execute(
            "UPDATE mappings SET payload = 'not json' WHERE id = ?1",
            [corrupt.id.to_string()],
        )
        .unwrap();
    drop(connection);

    let repository = HotkeysRepository::open(directory.path()).unwrap();
    assert_eq!(repository.get_mapping(valid.id).unwrap(), Some(valid));
    assert!(
        matches!(repository.get_mapping(unsupported.id), Err(RepositoryError::UnsupportedPayloadVersion { id, version }) if id == unsupported.id && version == PAYLOAD_FORMAT_VERSION + 1)
    );
    assert!(matches!(
        repository.get_mapping(corrupt.id),
        Err(RepositoryError::InvalidPayload { .. })
    ));
    assert!(database_path.is_file());
}

#[test]
fn set_enabled_refuses_unsupported_or_corrupt_persisted_payloads() {
    let directory = tempdir().unwrap();
    let unsupported = mapping(8, true, "KeyH");
    let corrupt = mapping(9, true, "KeyI");
    let mut repository = HotkeysRepository::open(directory.path()).unwrap();
    repository.insert_mapping(&unsupported).unwrap();
    repository.insert_mapping(&corrupt).unwrap();
    let database_path = repository.database_path().to_owned();
    drop(repository);

    let database =
        open_feature_database(directory.path(), FEATURE_ID, DurabilityProfile::Critical).unwrap();
    let connection = database.into_connection();
    connection
        .execute(
            "UPDATE mappings SET payload_version = ?2 WHERE id = ?1",
            (unsupported.id.to_string(), PAYLOAD_FORMAT_VERSION + 1),
        )
        .unwrap();
    connection
        .execute(
            "UPDATE mappings SET payload = 'not json' WHERE id = ?1",
            [corrupt.id.to_string()],
        )
        .unwrap();
    drop(connection);

    let mut repository = HotkeysRepository::open(directory.path()).unwrap();
    assert!(matches!(
        repository.set_mapping_enabled(unsupported.id, false),
        Err(RepositoryError::UnsupportedPayloadVersion { id, version }) if id == unsupported.id && version == PAYLOAD_FORMAT_VERSION + 1
    ));
    assert!(matches!(
        repository.set_mapping_enabled(corrupt.id, false),
        Err(RepositoryError::InvalidPayload { id, .. }) if id == corrupt.id
    ));
    assert!(database_path.is_file());
}

#[test]
fn malformed_stored_id_is_reported_without_resetting_the_database() {
    let directory = tempdir().unwrap();
    let valid = mapping(10, true, "KeyJ");
    let malformed = mapping(11, true, "KeyK");
    let mut repository = HotkeysRepository::open(directory.path()).unwrap();
    repository.insert_mapping(&valid).unwrap();
    repository.insert_mapping(&malformed).unwrap();
    let database_path = repository.database_path().to_owned();
    drop(repository);

    let database =
        open_feature_database(directory.path(), FEATURE_ID, DurabilityProfile::Critical).unwrap();
    let connection = database.into_connection();
    connection
        .execute(
            "UPDATE mappings SET id = 'not-a-uuid' WHERE id = ?1",
            [malformed.id.to_string()],
        )
        .unwrap();
    drop(connection);

    let repository = HotkeysRepository::open(directory.path()).unwrap();
    assert_eq!(repository.get_mapping(valid.id).unwrap(), Some(valid));
    assert!(matches!(
        repository.list_mappings(),
        Err(RepositoryError::InvalidStoredMappingId { stored_id, .. }) if stored_id == "not-a-uuid"
    ));
    assert!(database_path.is_file());
}
