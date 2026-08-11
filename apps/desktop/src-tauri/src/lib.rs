use std::collections::BTreeMap;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::SystemTime;

use app_runtime::{ActivationIntent, FeatureRegistry, FeatureSummary};
use diagnostics::{DiagnosticEvent, DiagnosticLevel, DiagnosticsRecorder};
use feature_api::{FeatureLifecycleState, RuntimeMode, StartupPolicy};
use ipc_contracts::{
    CommandErrorCategoryDto, CommandErrorDto, CoreRuntimeStateDto, CoreStatusDto,
    FeatureLifecycleStateDto, FeatureSummaryDto, RuntimeModeDto, StartupPolicyDto,
};
use persistence::{CoreStore, DurabilityProfile, PersistenceError};
use tauri::{
    AppHandle, Manager, WebviewUrl, WebviewWindowBuilder,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

const CONTROL_WINDOW_LABEL: &str = "control";
const OPEN_MENU_ID: &str = "open";
const QUIT_MENU_ID: &str = "quit";
const DIAGNOSTICS_CAPACITY: usize = 256;

struct ApplicationState {
    app_version: String,
    platform: String,
    runtime_state: CoreRuntimeStateDto,
    schema_version: Option<u32>,
    startup_error: Option<CommandErrorDto>,
    registry: Mutex<FeatureRegistry>,
    _core_store: Mutex<Option<CoreStore>>,
    exit_requested: AtomicBool,
}

struct DiagnosticsState(Mutex<DiagnosticsRecorder>);

struct CoreBootstrap {
    runtime_state: CoreRuntimeStateDto,
    schema_version: Option<u32>,
    startup_error: Option<CommandErrorDto>,
    core_store: Option<CoreStore>,
}

#[tauri::command]
fn get_core_status(
    state: tauri::State<'_, ApplicationState>,
) -> Result<CoreStatusDto, CommandErrorDto> {
    let registry = state
        .registry
        .lock()
        .map_err(|_| internal_command_error())?;

    Ok(CoreStatusDto {
        app_version: state.app_version.clone(),
        platform: state.platform.clone(),
        runtime_state: state.runtime_state,
        schema_version: state.schema_version,
        startup_error: state.startup_error.clone(),
        features: registry
            .summaries()
            .iter()
            .map(feature_summary_dto)
            .collect(),
    })
}

fn feature_summary_dto(summary: &FeatureSummary) -> FeatureSummaryDto {
    FeatureSummaryDto {
        id: summary.id.to_string(),
        display_name_key: summary.display_name_key.to_owned(),
        description_key: summary.description_key.to_owned(),
        runtime_mode: match summary.runtime_mode {
            RuntimeMode::EmbeddedBackground => RuntimeModeDto::EmbeddedBackground,
            RuntimeMode::EmbeddedOnDemand => RuntimeModeDto::EmbeddedOnDemand,
            RuntimeMode::IsolatedWorker => RuntimeModeDto::IsolatedWorker,
        },
        startup_policy: match summary.startup_policy {
            StartupPolicy::Manual => StartupPolicyDto::Manual,
            StartupPolicy::OnApplicationStart => StartupPolicyDto::OnApplicationStart,
        },
        lifecycle_state: match summary.lifecycle_state {
            FeatureLifecycleState::Unavailable => FeatureLifecycleStateDto::Unavailable,
            FeatureLifecycleState::Disabled => FeatureLifecycleStateDto::Disabled,
            FeatureLifecycleState::Starting => FeatureLifecycleStateDto::Starting,
            FeatureLifecycleState::Running => FeatureLifecycleStateDto::Running,
            FeatureLifecycleState::Degraded => FeatureLifecycleStateDto::Degraded,
            FeatureLifecycleState::Stopping => FeatureLifecycleStateDto::Stopping,
            FeatureLifecycleState::Failed => FeatureLifecycleStateDto::Failed,
        },
    }
}

fn internal_command_error() -> CommandErrorDto {
    command_error(
        "core_status_unavailable",
        CommandErrorCategoryDto::Internal,
        "error.internal",
    )
}

fn command_error(
    code: &'static str,
    category: CommandErrorCategoryDto,
    message_key: &'static str,
) -> CommandErrorDto {
    CommandErrorDto {
        code: code.to_owned(),
        category,
        message_key: message_key.to_owned(),
        recoverable: true,
        context: BTreeMap::new(),
    }
}

fn persistence_command_error(error: &PersistenceError) -> CommandErrorDto {
    match error {
        PersistenceError::Migration { .. } => command_error(
            "core_migration_failed",
            CommandErrorCategoryDto::Migration,
            "errors.core.migration_failed",
        ),
        PersistenceError::InvalidMigrationPlan { .. } => command_error(
            "core_migration_plan_invalid",
            CommandErrorCategoryDto::Migration,
            "errors.core.migration_failed",
        ),
        PersistenceError::FutureSchema { .. } => command_error(
            "core_schema_unsupported",
            CommandErrorCategoryDto::Migration,
            "errors.core.schema_unsupported",
        ),
        PersistenceError::InconsistentSchema { .. } => command_error(
            "core_schema_inconsistent",
            CommandErrorCategoryDto::Migration,
            "errors.core.schema_inconsistent",
        ),
        PersistenceError::Io { .. } => command_error(
            "core_persistence_io_failed",
            CommandErrorCategoryDto::Persistence,
            "errors.core.persistence_unavailable",
        ),
        PersistenceError::Sqlite { .. } => command_error(
            "core_persistence_database_failed",
            CommandErrorCategoryDto::Persistence,
            "errors.core.persistence_unavailable",
        ),
        PersistenceError::InvalidDurabilityProfile { .. } => command_error(
            "core_persistence_profile_invalid",
            CommandErrorCategoryDto::Persistence,
            "errors.core.persistence_unavailable",
        ),
        PersistenceError::InvalidFeatureId { .. } => command_error(
            "core_persistence_feature_id_invalid",
            CommandErrorCategoryDto::Persistence,
            "errors.core.persistence_unavailable",
        ),
    }
}

fn app_data_command_error() -> CommandErrorDto {
    command_error(
        "core_app_data_unavailable",
        CommandErrorCategoryDto::Persistence,
        "errors.core.persistence_unavailable",
    )
}

fn schema_version_command_error() -> CommandErrorDto {
    command_error(
        "core_schema_version_invalid",
        CommandErrorCategoryDto::Migration,
        "errors.core.schema_invalid",
    )
}

fn recover(error: CommandErrorDto) -> CoreBootstrap {
    CoreBootstrap {
        runtime_state: CoreRuntimeStateDto::RecoveryRequired,
        schema_version: None,
        startup_error: Some(error),
        core_store: None,
    }
}

fn open_core_store<R: tauri::Runtime>(app: &tauri::App<R>) -> CoreBootstrap {
    let app_data_dir = match app.path().app_data_dir() {
        Ok(directory) => directory,
        Err(_) => return recover(app_data_command_error()),
    };

    let store = match CoreStore::open(app_data_dir, DurabilityProfile::Critical) {
        Ok(store) => store,
        Err(error) => return recover(persistence_command_error(&error)),
    };

    let schema_version = match store.schema_version() {
        Ok(version) => match u32::try_from(version) {
            Ok(version) => version,
            Err(_) => return recover(schema_version_command_error()),
        },
        Err(error) => return recover(persistence_command_error(&error)),
    };

    CoreBootstrap {
        runtime_state: CoreRuntimeStateDto::Running,
        schema_version: Some(schema_version),
        startup_error: None,
        core_store: Some(store),
    }
}

fn record_diagnostic(
    app: &AppHandle,
    level: DiagnosticLevel,
    event: &'static str,
    error_code: Option<&'static str>,
) {
    if let Some(diagnostics_state) = app.try_state::<DiagnosticsState>()
        && let Ok(mut diagnostics) = diagnostics_state.0.lock()
    {
        diagnostics.record(DiagnosticEvent {
            timestamp: SystemTime::now(),
            level,
            component: "desktop_host",
            event,
            error_code,
        });
    }
}

fn startup_error_code(error: &CommandErrorDto) -> &'static str {
    match error.code.as_str() {
        "core_migration_failed" => "CORE_MIGRATION_FAILED",
        "core_migration_plan_invalid" => "CORE_MIGRATION_PLAN_INVALID",
        "core_schema_unsupported" => "CORE_SCHEMA_UNSUPPORTED",
        "core_schema_inconsistent" => "CORE_SCHEMA_INCONSISTENT",
        "core_schema_version_invalid" => "CORE_SCHEMA_VERSION_INVALID",
        "core_app_data_unavailable" => "CORE_APP_DATA_UNAVAILABLE",
        "core_persistence_io_failed" => "CORE_PERSISTENCE_IO_FAILED",
        "core_persistence_database_failed" => "CORE_PERSISTENCE_DATABASE_FAILED",
        "core_persistence_feature_id_invalid" => "CORE_PERSISTENCE_FEATURE_ID_INVALID",
        "core_persistence_profile_invalid" => "CORE_PERSISTENCE_PROFILE_INVALID",
        _ => "CORE_PERSISTENCE_RECOVERY",
    }
}

fn ensure_control_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(CONTROL_WINDOW_LABEL) {
        window.show()?;
        window.unminimize()?;
        return window.set_focus();
    }

    let window = WebviewWindowBuilder::new(
        app,
        CONTROL_WINDOW_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("Qbit Toolbox")
    .inner_size(1100.0, 760.0)
    .min_inner_size(800.0, 560.0)
    .build()?;
    window.show()?;
    window.set_focus()
}

fn handle_activation(app: &AppHandle, intent: ActivationIntent) {
    match intent {
        ActivationIntent::OpenControlWindow => {
            if ensure_control_window(app).is_err() {
                record_diagnostic(
                    app,
                    DiagnosticLevel::Error,
                    "control_window_activation_failed",
                    Some("CONTROL_WINDOW_ACTIVATION_FAILED"),
                );
            }
        }
    }
}

fn quit_application(app: &AppHandle) {
    if let Some(state) = app.try_state::<ApplicationState>() {
        state.exit_requested.store(true, Ordering::Relaxed);
    }
    app.exit(0);
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let application = tauri::Builder::default()
        .manage(DiagnosticsState(Mutex::new(DiagnosticsRecorder::new(
            DIAGNOSTICS_CAPACITY,
        ))))
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            handle_activation(app, ActivationIntent::OpenControlWindow)
        }))
        .setup(|app| {
            let core = open_core_store(app);
            if let Some(error) = core.startup_error.as_ref() {
                record_diagnostic(
                    app.handle(),
                    DiagnosticLevel::Error,
                    "core_persistence_recovery_entered",
                    Some(startup_error_code(error)),
                );
            }

            let recovery_required = core.runtime_state == CoreRuntimeStateDto::RecoveryRequired;
            app.manage(ApplicationState {
                app_version: app.package_info().version.to_string(),
                platform: std::env::consts::OS.to_owned(),
                runtime_state: core.runtime_state,
                schema_version: core.schema_version,
                startup_error: core.startup_error,
                registry: Mutex::new(FeatureRegistry::new()),
                _core_store: Mutex::new(core.core_store),
                exit_requested: AtomicBool::new(false),
            });

            let open_item =
                MenuItemBuilder::with_id(OPEN_MENU_ID, "Open Qbit Toolbox").build(app)?;
            let quit_item = MenuItemBuilder::with_id(QUIT_MENU_ID, "Quit").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&open_item, &quit_item])
                .build()?;

            let mut tray = TrayIconBuilder::with_id("qbit-toolbox")
                .menu(&menu)
                .tooltip("Qbit Toolbox")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    OPEN_MENU_ID => handle_activation(app, ActivationIntent::OpenControlWindow),
                    QUIT_MENU_ID => quit_application(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        handle_activation(tray.app_handle(), ActivationIntent::OpenControlWindow);
                    }
                });
            if let Some(icon) = app.default_window_icon().cloned() {
                tray = tray.icon(icon);
            }
            tray.build(app)?;

            if recovery_required {
                handle_activation(app.handle(), ActivationIntent::OpenControlWindow);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_core_status])
        .build(tauri::generate_context!())?;

    application.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            let exit_requested = app
                .try_state::<ApplicationState>()
                .is_some_and(|state| state.exit_requested.load(Ordering::Relaxed));
            if !exit_requested {
                api.prevent_exit();
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn future_schema_maps_to_a_safe_migration_error() {
        let error = persistence_command_error(&PersistenceError::FutureSchema {
            current_version: 2,
            latest_supported_version: 1,
        });

        assert_eq!(error.category, CommandErrorCategoryDto::Migration);
        assert_eq!(error.code, "core_schema_unsupported");
        assert_eq!(error.message_key, "errors.core.schema_unsupported");
        assert!(error.context.is_empty());
    }

    #[test]
    fn invalid_migration_plan_maps_to_a_safe_migration_error() {
        let error = persistence_command_error(&PersistenceError::InvalidMigrationPlan {
            source: persistence::MigrationPlanError::EmptyName { version: 42 },
        });

        assert_eq!(error.category, CommandErrorCategoryDto::Migration);
        assert_eq!(error.code, "core_migration_plan_invalid");
        assert_eq!(startup_error_code(&error), "CORE_MIGRATION_PLAN_INVALID");
        assert_eq!(error.message_key, "errors.core.migration_failed");
        assert!(error.context.is_empty());
    }

    #[test]
    fn invalid_feature_id_maps_to_a_safe_persistence_error() {
        let error = persistence_command_error(&PersistenceError::InvalidFeatureId {
            source: feature_api::FeatureIdError::InvalidCharacter,
        });

        assert_eq!(error.category, CommandErrorCategoryDto::Persistence);
        assert_eq!(error.code, "core_persistence_feature_id_invalid");
        assert_eq!(
            startup_error_code(&error),
            "CORE_PERSISTENCE_FEATURE_ID_INVALID"
        );
        assert_eq!(error.message_key, "errors.core.persistence_unavailable");
        assert!(error.context.is_empty());
    }

    #[test]
    fn io_error_maps_to_a_safe_persistence_error() {
        let error = persistence_command_error(&PersistenceError::Io {
            source: std::io::Error::other("sensitive filesystem detail"),
        });

        assert_eq!(error.category, CommandErrorCategoryDto::Persistence);
        assert_eq!(error.code, "core_persistence_io_failed");
        assert_eq!(error.message_key, "errors.core.persistence_unavailable");
        assert!(error.context.is_empty());
        assert!(!error.code.contains("sensitive"));
        assert!(!error.message_key.contains("sensitive"));
    }
}
