//! Versioned logical gameplay snapshots and completed-tick storage requests.
mod value;
mod world;
use super::*;
use crate::storage::SharedStorage;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub use value::SaveValue;

#[derive(Clone)]
pub struct GameProgress {
    pub values: HashMap<String, Value>,
    pub spawn_counter: u64,
}

pub const GAME_SAVE_VERSION: u32 = 1;
pub const MAX_GAME_SAVE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptSnapshot {
    pub signature: String,
    pub locals: Vec<SaveValue>,
    pub values: BTreeMap<String, SaveValue>,
    pub properties: BTreeMap<String, f64>,
    pub frame: u64,
    pub elapsed: f64,
    pub time_scale: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachedSnapshot {
    pub path: String,
    pub started: bool,
    pub tick: u64,
    pub memory: ScriptSnapshot,
}

/// A complete active-scene save, independent of the sample score/coin API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSave {
    pub version: u32,
    pub spawn_counter: u64,
    pub pending_scene: Option<String>,
    pub signals: Vec<(String, SaveValue)>,
    pub project_id: String,
    pub scene: kiln_core::Scene,
    pub entities: BTreeMap<String, world::SavedEntity>,
    pub root: ScriptSnapshot,
    pub attached: BTreeMap<String, AttachedSnapshot>,
    pub game_data: BTreeMap<String, SaveValue>,
    pub ui: kiln_ui::authored::UiSnapshot,
    /// Retained for old scripts; logical world restoration does not use it.
    pub sample_compatibility: SliceState,
}

impl GameSave {
    pub fn from_json(text: &str) -> Result<Self, String> {
        if text.len() > MAX_GAME_SAVE_BYTES {
            return Err("Gameplay save exceeds 8 MiB".into());
        }
        let saved: Self = serde_json::from_str(text).map_err(|e| {
            format!("Invalid gameplay save (legacy sample saves cannot restore general games): {e}")
        })?;
        if saved.version != GAME_SAVE_VERSION {
            return Err(format!(
                "Unsupported gameplay save version {}",
                saved.version
            ));
        }
        Ok(saved)
    }
    pub fn to_json(&self) -> Result<String, String> {
        if self.version != GAME_SAVE_VERSION {
            return Err("Unsupported gameplay save version".into());
        }
        let json = serde_json::to_string(self).map_err(|e| e.to_string())?;
        if json.len() > MAX_GAME_SAVE_BYTES {
            return Err("Gameplay save exceeds 8 MiB".into());
        }
        Ok(json)
    }
    /// Construct a candidate world/session before any live state is replaced.
    pub fn restore(
        &self,
        project: &kiln_core::Project,
        read: impl Fn(&str) -> Result<String, String>,
        resolve: impl FnMut(&str) -> Option<(kiln_render::TextureId, [u32; 2])>,
    ) -> Result<(SliceSession, GameWorld), String> {
        if self.version != GAME_SAVE_VERSION || self.project_id != project.id {
            return Err("Save belongs to a different project or schema".into());
        }
        let original = project
            .scenes
            .get(&self.scene.id)
            .ok_or("Saved scene no longer exists")?;
        if original.script != self.scene.script {
            return Err("Saved scene script has changed".into());
        }
        world::validate(&self.scene, &self.entities, project)?;
        if !self.sample_compatibility.elapsed.is_finite()
            || !self.sample_compatibility.player_x.is_finite()
            || !self.sample_compatibility.player_y.is_finite()
        {
            return Err("Invalid compatibility state".into());
        }
        let mut candidate = project.clone();
        candidate
            .scenes
            .insert(self.scene.id.clone(), self.scene.clone());
        let mut session = SliceSession::with_scene_sources(&candidate, &self.scene.id, read)?;
        if self
            .pending_scene
            .as_ref()
            .is_some_and(|id| !project.scenes.contains_key(id))
            || self.signals.len() > 256
        {
            return Err("Invalid saved scene request or signal queue".into());
        }
        session.restore_memory(&self.root)?;
        session.vm.host_mut().spawn_counter = self.spawn_counter;
        session.vm.host_mut().scene_request = self.pending_scene.clone();
        session.signals = self
            .signals
            .iter()
            .map(|(name, value)| {
                if name.len() > 256 {
                    return Err("Invalid saved signal name".into());
                }
                Ok((name.clone(), value.value()?))
            })
            .collect::<Result<_, String>>()?;
        session.state = self.sample_compatibility.clone();
        session.vm.host_mut().state = session.state.clone();
        session.vm.host_mut().game_data = value::restore_map(&self.game_data)?;
        session.vm.host_mut().ui.restore_snapshot(&self.ui)?;
        if self.attached.len() != session.attached.len() {
            return Err("Saved script attachment set has changed".into());
        }
        for attached in &mut session.attached {
            let saved = self
                .attached
                .get(&attached.entity_id)
                .ok_or("Missing saved script instance")?;
            if saved.path != attached.path {
                return Err("Saved script path has changed".into());
            }
            attached.session.restore_memory(&saved.memory)?;
            attached.started = saved.started;
            attached.frame = saved.tick;
        }
        let world = world::restore(&self.scene, &self.entities, &candidate, resolve)?;
        Ok((session, world))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SaveRequest {
    Save(u32),
    Load(u32),
}

impl SliceHost {
    pub fn player_preferences(&self) -> &crate::preferences::PlayerPreferences {
        &self.preferences
    }
    pub fn set_platform_audio(&mut self, mut audio: crate::audio_backend::AudioBackend) {
        if let Some(settings) = &self.preferences.audio {
            audio.set_settings(settings.clone());
        }
        self.platform_audio = Some(audio);
    }
    pub(super) fn call_platform_audio(
        &mut self,
        id: u32,
        args: &[Value],
    ) -> Result<Value, VmError> {
        let audio = self
            .platform_audio
            .as_mut()
            .ok_or_else(|| VmError::HostCallError("Audio backend missing".into()))?;
        let old = audio.settings();
        let result = audio.call(id, args).map_err(VmError::HostCallError)?;
        let settings = audio.settings();
        if matches!(id, 16..=18 | 76) && settings != old && self.storage.is_some() {
            let mut candidate = self.preferences.clone();
            candidate.audio = Some(settings);
            if let Err(error) = self.write_preferences(candidate) {
                if let Some(audio) = &mut self.platform_audio {
                    audio.set_settings(old);
                }
                return Err(error);
            }
        }
        Ok(result)
    }

    /// Install a platform storage backend and read independent preferences.
    pub fn set_storage(&mut self, storage: SharedStorage) {
        let result = storage.read_preferences().and_then(|text| match text {
            None => Ok(crate::preferences::PlayerPreferences::default()),
            Some(text) => {
                let preferences: crate::preferences::PlayerPreferences =
                    serde_json::from_str(&text).map_err(|e| e.to_string())?;
                if preferences.version != crate::preferences::PREFERENCES_VERSION {
                    return Err("Unsupported player preferences version".into());
                }
                Ok(preferences)
            }
        });
        match result {
            Ok(preferences) => {
                self.preferences = preferences;
                self.preference_fault = None;
            }
            Err(e) => self.preference_fault = Some(e),
        }
        if let (Some(audio), Some(settings)) = (&mut self.platform_audio, &self.preferences.audio) {
            audio.set_settings(settings.clone());
        }
        self.storage = Some(storage);
        #[cfg(feature = "audio")]
        if let (Some(audio), Some(settings)) = (&mut self.audio, &self.preferences.audio) {
            audio.set_settings(settings.clone());
        }
    }
    pub fn save_status(&self) -> &str {
        if self.save_status.is_empty() {
            "idle"
        } else {
            &self.save_status
        }
    }
    pub fn save_error(&self) -> &str {
        &self.save_error
    }
    pub fn game_value(&self, key: &str) -> Option<&Value> {
        self.game_data.get(key)
    }
    pub fn set_game_value(&mut self, key: &str, value: Value) -> Result<(), String> {
        if key.is_empty()
            || key.len() > 256
            || (self.game_data.len() >= 4096 && !self.game_data.contains_key(key))
        {
            return Err("Game data key limit reached".into());
        }
        SaveValue::capture(&value)?;
        self.game_data.insert(key.into(), value);
        Ok(())
    }
    pub(super) fn swap_game_services(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.storage, &mut other.storage);
        std::mem::swap(&mut self.platform_audio, &mut other.platform_audio);
        std::mem::swap(&mut self.storage_events, &mut other.storage_events);
        std::mem::swap(&mut self.spawn_counter, &mut other.spawn_counter);
        std::mem::swap(&mut self.game_data, &mut other.game_data);
        std::mem::swap(&mut self.save_requests, &mut other.save_requests);
        std::mem::swap(&mut self.save_status, &mut other.save_status);
        std::mem::swap(&mut self.save_error, &mut other.save_error);
    }
    pub fn request_save(&mut self, slot: u32, load: bool) -> bool {
        if slot > 99 || self.storage.is_none() || self.save_requests.len() >= 16 {
            self.save_status = "error".into();
            self.save_error = "Save slot/storage/request limit is invalid".into();
            return false;
        }
        self.save_status = "queued".into();
        self.save_error.clear();
        self.save_requests.push(if load {
            SaveRequest::Load(slot)
        } else {
            SaveRequest::Save(slot)
        });
        true
    }
    pub(super) fn call_save(&mut self, id: u32, args: &[Value]) -> Result<Value, VmError> {
        match id {
            53 | 54 => {
                let slot = args
                    .first()
                    .and_then(Value::as_i64)
                    .filter(|v| *v >= 0 && *v <= 99);
                Ok(Value::Bool(
                    slot.is_some_and(|v| self.request_save(v as u32, id == 54)),
                ))
            }
            89 => Ok(self
                .game_data
                .get(args[0].as_str().unwrap_or_default())
                .cloned()
                .unwrap_or_else(|| args[1].clone())),
            90 => {
                let key = args[0].as_str().ok_or_else(|| {
                    VmError::HostCallError("game_set expects a string key".into())
                })?;
                self.set_game_value(key, args[1].clone())
                    .map_err(VmError::HostCallError)?;
                Ok(args[1].clone())
            }
            91 => Ok(Value::Str(self.save_status().into())),
            92 => Ok(Value::Str(self.save_error.clone())),
            93 => {
                let slot = args[0].as_i64().filter(|v| *v >= 0 && *v <= 99);
                let exists = slot
                    .zip(self.storage.as_ref())
                    .and_then(|(slot, storage)| storage.read_slot(slot as u32).ok().flatten())
                    .is_some_and(|text| GameSave::from_json(&text).is_ok());
                Ok(Value::Bool(exists))
            }
            _ => Err(VmError::HostCallError("Unknown save builtin".into())),
        }
    }
}

impl SliceSession {
    fn signature(&self) -> String {
        let source = self
            .source
            .as_ref()
            .map(|s| s.source.clone())
            .unwrap_or_else(|| format!("{:?}", self.program.instructions()));
        let mut hash = 0xcbf29ce484222325u64;
        for byte in source.as_bytes() {
            hash = (hash ^ *byte as u64).wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}:{}", self.program.local_slots())
    }
    fn memory(&self) -> Result<ScriptSnapshot, String> {
        let host = self.vm.host();
        if !self.state.elapsed.is_finite() || !host.time_scale.is_finite() {
            return Err("Non-finite saved script time".into());
        }
        Ok(ScriptSnapshot {
            signature: self.signature(),
            locals: self
                .vm
                .locals()
                .iter()
                .map(SaveValue::capture)
                .collect::<Result<_, _>>()?,
            values: value::capture_map(&host.script_state)?,
            properties: host
                .properties
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
            frame: host.frame_count,
            elapsed: self.state.elapsed,
            time_scale: host.time_scale,
        })
    }
    fn restore_memory(&mut self, saved: &ScriptSnapshot) -> Result<(), String> {
        if saved.signature != self.signature()
            || saved.locals.len() > 65536
            || saved.properties.len() > 4096
            || saved.properties.values().any(|v| !v.is_finite())
            || !saved.elapsed.is_finite()
            || saved.elapsed < 0.0
            || !saved.time_scale.is_finite()
            || saved.time_scale < 0.0
        {
            return Err("Incompatible saved script memory or time".into());
        }
        self.vm
            .restore_locals(
                &self.program,
                saved
                    .locals
                    .iter()
                    .map(SaveValue::value)
                    .collect::<Result<_, _>>()?,
            )
            .map_err(|e| e.to_string())?;
        self.vm.host_mut().script_state = value::restore_map(&saved.values)?;
        self.vm.host_mut().properties = saved
            .properties
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        self.vm.host_mut().frame_count = saved.frame;
        self.vm.host_mut().time_scale = saved.time_scale;
        self.state.elapsed = saved.elapsed;
        Ok(())
    }
    pub fn scene_id(&self) -> Option<&str> {
        self.save_identity.as_ref().map(|(_, id, _)| id.as_str())
    }
    pub fn snapshot(&self, world: &GameWorld) -> Result<GameSave, String> {
        let (project_id, scene_id, script) = self
            .save_identity
            .as_ref()
            .ok_or("Session was not created from a project scene")?;
        let (scene, entities) = world::capture(world, scene_id, script.clone())?;
        let mut attached = BTreeMap::new();
        for script in &self.attached {
            if entities.contains_key(&script.entity_id) {
                attached.insert(
                    script.entity_id.clone(),
                    AttachedSnapshot {
                        path: script.path.clone(),
                        started: script.started,
                        tick: script.frame,
                        memory: script.session.memory()?,
                    },
                );
            }
        }
        let saved = GameSave {
            version: GAME_SAVE_VERSION,
            spawn_counter: self.vm.host().spawn_counter,
            pending_scene: self.vm.host().scene_request.clone().or_else(|| {
                self.attached
                    .iter()
                    .find_map(|a| a.session.vm.host().scene_request.clone())
            }),
            signals: self
                .signals
                .iter()
                .chain(&self.vm.host().emitted_signals)
                .map(|(name, v)| Ok((name.clone(), SaveValue::capture(v)?)))
                .collect::<Result<_, String>>()?,
            project_id: project_id.clone(),
            scene,
            entities,
            root: self.memory()?,
            attached,
            game_data: value::capture_map(&self.vm.host().game_data)?,
            ui: self.vm.host().ui.snapshot(),
            sample_compatibility: self.state.clone(),
        };
        world::validate(
            &saved.scene,
            &saved.entities,
            self.save_project
                .as_ref()
                .ok_or("Missing save project context")?,
        )?;
        if saved.spawn_counter == u64::MAX
            || saved.signals.len() > 256
            || saved.signals.iter().any(|(name, _)| name.len() > 256)
            || saved.pending_scene.as_ref().is_some_and(|id| {
                !self
                    .save_project
                    .as_ref()
                    .is_some_and(|p| p.scenes.contains_key(id))
            })
        {
            return Err("Invalid saved runtime counter or event queue".into());
        }
        saved.to_json()?;
        Ok(saved)
    }
    pub fn save_game(&mut self, world: &GameWorld, slot: u32) -> Result<(), String> {
        let result = (|| {
            if slot > 99 {
                return Err("Save slot must be between 0 and 99".into());
            }
            let storage = self
                .vm
                .host()
                .storage
                .as_ref()
                .ok_or("Player storage is not configured")?;
            if let Some(previous) = storage.read_slot(slot)? {
                let saved = GameSave::from_json(&previous)?;
                if self
                    .save_identity
                    .as_ref()
                    .is_none_or(|id| id.0 != saved.project_id)
                {
                    return Err("Existing slot belongs to another project".into());
                }
            }
            let saved = self.snapshot(world)?;
            storage.write_slot(slot, &saved.to_json()?)
        })();
        self.finish_storage(&result, "saved");
        result
    }
    pub fn load_game(
        &mut self,
        world: &mut GameWorld,
        slot: u32,
        project: &kiln_core::Project,
        read: impl Fn(&str) -> Result<String, String>,
        resolve: impl FnMut(&str) -> Option<(kiln_render::TextureId, [u32; 2])>,
    ) -> Result<(), String> {
        let result = (|| {
            if slot > 99 {
                return Err("Save slot must be between 0 and 99".into());
            }
            let storage = self
                .vm
                .host()
                .storage
                .clone()
                .ok_or("Player storage is not configured")?;
            let text = storage.read_slot(slot)?.ok_or("Save slot does not exist")?;
            let saved = GameSave::from_json(&text)?;
            let (mut candidate, candidate_world) = saved.restore(project, read, resolve)?;
            candidate.vm.host_mut().storage = Some(storage);
            candidate.vm.host_mut().preferences = self.vm.host().preferences.clone();
            candidate.vm.host_mut().preference_fault = self.vm.host().preference_fault.clone();
            #[cfg(feature = "audio")]
            std::mem::swap(
                &mut self.vm.host_mut().audio,
                &mut candidate.vm.host_mut().audio,
            );
            std::mem::swap(
                &mut self.vm.host_mut().platform_audio,
                &mut candidate.vm.host_mut().platform_audio,
            );
            std::mem::swap(
                &mut self.vm.host_mut().audio_player,
                &mut candidate.vm.host_mut().audio_player,
            );
            *world = candidate_world;
            *self = candidate;
            Ok(())
        })();
        self.finish_storage(&result, "loaded");
        result
    }
    fn finish_storage(&mut self, result: &Result<(), String>, success: &str) {
        let host = self.vm.host_mut();
        if host.storage_events.len() < 16 {
            host.storage_events.push(if result.is_ok() {
                if success == "loaded" {
                    "load_complete".into()
                } else {
                    "save_complete".into()
                }
            } else {
                if success == "loaded" {
                    "load_failed".into()
                } else {
                    "save_failed".into()
                }
            });
        }
        match result {
            Ok(()) => {
                host.save_status = success.into();
                host.save_error.clear();
            }
            Err(error) => {
                host.save_status = "error".into();
                host.save_error = error.clone();
                log::warn!("Gameplay storage: {error}");
            }
        }
    }
    /// Service requests after a complete tick. A successful load discards old-timeline requests.
    pub fn service_saves(
        &mut self,
        world: &mut GameWorld,
        project: &kiln_core::Project,
        read: impl Fn(&str) -> Result<String, String>,
        mut resolve: impl FnMut(&str) -> Option<(kiln_render::TextureId, [u32; 2])>,
    ) -> Result<bool, String> {
        let requests = std::mem::take(&mut self.vm.host_mut().save_requests);
        for request in requests {
            match request {
                SaveRequest::Save(slot) => self.save_game(world, slot)?,
                SaveRequest::Load(slot) => {
                    self.load_game(world, slot, project, &read, &mut resolve)?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    pub fn game_progress(&self) -> GameProgress {
        GameProgress {
            values: self.vm.host().game_data.clone(),
            spawn_counter: self.vm.host().spawn_counter,
        }
    }
    pub fn set_game_progress(&mut self, progress: GameProgress) -> Result<(), String> {
        value::capture_map(&progress.values)?;
        self.vm.host_mut().game_data = progress.values;
        self.vm.host_mut().spawn_counter = progress.spawn_counter;
        Ok(())
    }
    /// Carry game-wide progress across a normal scene transition; preferences/storage stay independent.
    pub fn transfer_game_data_to(&self, next: &mut SliceSession) {
        next.vm.host_mut().game_data = self.vm.host().game_data.clone();
        next.vm.host_mut().spawn_counter = self.vm.host().spawn_counter;
        next.vm.host_mut().storage = self.vm.host().storage.clone();
        next.vm.host_mut().preferences = self.vm.host().preferences.clone();
        next.vm.host_mut().preference_fault = self.vm.host().preference_fault.clone();
    }
}
