use super::storage::{load_settings, save_settings};
use crate::{
    cli::Cancel,
    model::{Cli, Phase, Session, Settings},
    profiles::Files,
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};
use tokio::{runtime::Handle, sync::Mutex as AsyncMutex};

#[derive(Clone)]
pub(super) struct Job {
    pub(super) cancel: Cancel,
    pub(super) phase: Phase,
    pub(super) shared_sso: bool,
}

pub(super) struct Data {
    pub(super) settings: Settings,
    pub(super) cli: Option<Cli>,
    pub(super) scanning: bool,
    pub(super) wizard: bool,
    pub(super) closing: bool,
    pub(super) jobs: BTreeMap<String, Job>,
    pub(super) activity: BTreeMap<String, String>,
    pub(super) profiles_error: Option<String>,
    pub(super) settings_error: Option<String>,
}

pub struct AppState {
    pub files: Files,
    pub(super) home: PathBuf,
    settings_path: PathBuf,
    pub(super) runtime: Handle,
    data: Mutex<Data>,
    pub(super) discovery: AsyncMutex<()>,
    pub(super) aws_files: AsyncMutex<()>,
    pub installer: AsyncMutex<()>,
}

impl AppState {
    pub fn new(
        home: PathBuf,
        config: PathBuf,
        runtime: Handle,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let settings_path = config.join("settings.json");
        let legacy_settings_path = config
            .with_file_name("io.github.sejoung.cloud-session-tray")
            .join("settings.json");
        let (settings, settings_error) = match load_settings(&settings_path, &legacy_settings_path)
        {
            Ok(settings) => (settings, None),
            Err(error) => (Settings::default(), Some(error.into())),
        };
        Ok(Self {
            files: Files::new(&home)?,
            home,
            settings_path,
            runtime,
            data: Mutex::new(Data {
                settings,
                cli: None,
                scanning: true,
                wizard: false,
                closing: false,
                jobs: BTreeMap::new(),
                activity: BTreeMap::new(),
                profiles_error: None,
                settings_error,
            }),
            discovery: AsyncMutex::new(()),
            aws_files: AsyncMutex::new(()),
            installer: AsyncMutex::new(()),
        })
    }

    pub(super) fn data(&self) -> MutexGuard<'_, Data> {
        self.data.lock().expect("app state lock")
    }

    pub(super) fn update_settings(
        &self,
        data: &mut Data,
        update: impl FnOnce(&mut Settings),
    ) -> Result<(), String> {
        if let Some(error) = &data.settings_error {
            return Err(error.clone());
        }
        let mut settings = data.settings.clone();
        update(&mut settings);
        save_settings(&self.settings_path, &settings)
            .map_err(|_| "앱 설정을 저장할 수 없습니다. 로컬 설정 폴더 권한을 확인해 주세요.")?;
        data.settings = settings;
        Ok(())
    }

    pub fn sessions(&self) -> Vec<Session> {
        self.data().settings.sessions.clone()
    }

    pub(super) fn current_cli(&self) -> Result<Cli, String> {
        self.data()
            .cli
            .as_ref()
            .filter(|cli| cli.v2)
            .cloned()
            .ok_or_else(|| "AWS CLI v2를 설치하거나 실행 파일을 선택해 주세요.".into())
    }

    pub(super) fn phase(&self, id: &str, phase: Phase) {
        if let Some(job) = self.data().jobs.get_mut(id) {
            job.phase = phase;
        }
    }

    pub(super) fn activity(&self, id: &str, message: String) {
        self.data().activity.insert(id.into(), message);
    }
}
