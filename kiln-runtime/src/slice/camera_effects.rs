//! Bounded callback camera commands shared by scene and attachment services.
use super::*;
use crate::camera::{CameraController, CameraEffect, CameraShake, MAX_CAMERA_EFFECTS};
impl SliceHost {
    pub(super) fn call_camera_effect(&mut self, id: u32, args: &[Value]) -> Result<Value, VmError> {
        let effect = if id == 125 {
            CameraEffect::StopShake
        } else {
            let number = |index: usize| {
                args[index].to_f64().map(|v| v as f32).ok_or_else(|| {
                    VmError::HostCallError(
                        "Camera shake expects numeric amplitude/duration/frequency".into(),
                    )
                })
            };
            let seed = args[4]
                .as_i64()
                .and_then(|v| u32::try_from(v).ok())
                .ok_or_else(|| {
                    VmError::HostCallError(
                        "Camera shake seed must be an integer in 0..4294967295".into(),
                    )
                })?;
            let request = CameraShake {
                amplitude: [number(0)?, number(1)?],
                duration: number(2)?,
                frequency: number(3)?,
                seed,
            };
            request.validate().map_err(VmError::HostCallError)?;
            CameraEffect::Shake(request)
        };
        if self.camera_effects.len() + self.pending_camera_effects >= MAX_CAMERA_EFFECTS {
            return Ok(Value::Bool(false));
        }
        self.pending_camera_effects += 1;
        self.commands.push(SliceCommand::CameraEffect(effect));
        Ok(Value::Bool(true))
    }
}
impl SliceSession {
    /// Drain committed transient effects only between completed ticks.
    pub fn take_camera_effects(&mut self) -> Vec<CameraEffect> {
        if self.pending_tick.is_some() || self.program_pending {
            return Vec::new();
        }
        std::mem::take(&mut self.vm.host_mut().camera_effects)
    }
    /// Shared host adapter: consume committed effects and hold their time during UI pause.
    pub fn update_camera(
        &mut self,
        controller: &mut CameraController,
        settings: &kiln_core::camera::CameraSettings,
        world: &GameWorld,
        size: [f32; 2],
        dt: f32,
    ) -> Result<kiln_render::camera::Camera, String> {
        if self.pending_tick.is_some() || self.program_pending {
            return Err("Resume the suspended tick before updating camera effects".into());
        }
        for effect in self.take_camera_effects() {
            controller.apply_effect(effect)?;
        }
        let dt = if self.host_ui().pauses_game() {
            0.0
        } else {
            dt
        };
        controller.update(settings, world, size, dt)
    }
}
