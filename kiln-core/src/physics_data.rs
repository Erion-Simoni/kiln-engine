//! Serializable 2D body, collider and joint authoring data.

/// 2D rigid body physics component.
///
/// When the `physics` feature is enabled, this component is synced with a
/// rapier2d rigid body each frame. When disabled, it is a plain data holder.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RigidBody2D {
    /// Body type: static, dynamic, or kinematic.
    pub body_type: RigidBodyType,
    /// Linear velocity (world units per second).
    pub linear_velocity: [f32; 2],
    /// Angular velocity (radians per second).
    pub angular_velocity: f32,
    /// Gravity scale multiplier (1.0 = normal gravity).
    pub gravity_scale: f32,
    /// Whether the body can sleep.
    pub can_sleep: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RigidBodyType {
    Static,
    Dynamic,
    Kinematic,
}

impl Default for RigidBody2D {
    fn default() -> Self {
        Self {
            body_type: RigidBodyType::Dynamic,
            linear_velocity: [0.0, 0.0],
            angular_velocity: 0.0,
            gravity_scale: 1.0,
            can_sleep: true,
        }
    }
}

/// 2D collider component.
///
/// Defines the collision shape attached to a rigid body.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Collider2D {
    /// Collider shape.
    pub shape: ColliderShape,
    /// Whether the collider is a sensor (triggers events but no physical response).
    pub is_sensor: bool,
    /// Friction coefficient (0.0 = frictionless, 1.0 = high friction).
    pub friction: f32,
    /// Restitution / bounciness (0.0 = no bounce, 1.0 = super bouncy).
    pub restitution: f32,
    /// Density (used for mass calculation of dynamic bodies).
    pub density: f32,
    /// Collision membership bitset. Both colliders' membership/mask tests must
    /// pass for the pair to interact.
    pub layer: u16,
    /// Collision mask - which layers this collider will collide with.
    /// Bit i is set if this collider collides with layer i.
    pub mask: u16,
}

/// Collider shape variants. Uses `PartialEq` only (no `Eq`) because radii
/// are `f32`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ColliderShape {
    /// A circle with the given radius.
    Circle { radius: f32 },
    /// A rectangle with the given half-extents.
    Rectangle { half_width: f32, half_height: f32 },
}

impl Default for Collider2D {
    fn default() -> Self {
        Self {
            shape: ColliderShape::Rectangle {
                half_width: 16.0,
                half_height: 16.0,
            },
            is_sensor: false,
            friction: 0.5,
            restitution: 0.0,
            density: 1.0,
            layer: 0xFFFF,
            mask: 0xFFFF,
        }
    }
}

/// Joint type for 2D physics joints.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JointType {
    /// Keeps two bodies at a fixed distance from each other.
    Distance,
    /// Revolute joint (hinge) - allows rotation around a point.
    Revolute,
    /// Prismatic joint (slider) - allows linear motion along an axis.
    Prismatic,
}

/// A 2D joint component connecting two entities via their physics bodies.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Joint2D {
    /// First entity in the joint.
    pub entity_a: String,
    /// Second entity in the joint.
    pub entity_b: String,
    /// Type of joint.
    pub joint_type: JointType,
    /// Anchor point on entity A (local coordinates).
    pub anchor_a: [f32; 2],
    /// Anchor point on entity B (local coordinates).
    pub anchor_b: [f32; 2],
    /// Reference angle (radians) - relative angle between bodies.
    pub reference_angle: f32,
    /// For distance joints: target distance.
    pub length: f32,
    /// For prismatic joints: translation limits.
    pub lower_translation: f32,
    pub upper_translation: f32,
    /// Enable translation limits.
    pub enable_limit: bool,
    /// Motor speed (for revolute/prismatic joints).
    pub motor_speed: f32,
    /// Maximum motor force/torque.
    pub max_motor_torque: f32,
    /// Enable motor.
    pub enable_motor: bool,
}

/// Validate finite, non-degenerate body/collider data before authoring or spawn.
pub fn validate_body(body: &RigidBody2D, collider: &Collider2D) -> Result<(), String> {
    if !body.linear_velocity.iter().all(|value| value.is_finite())
        || !body.angular_velocity.is_finite()
        || !body.gravity_scale.is_finite()
    {
        return Err("Body values must be finite".into());
    }
    if ![collider.friction, collider.restitution, collider.density]
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
        || collider.restitution > 1.0
    {
        return Err("Collider material values must be valid".into());
    }
    let valid = match collider.shape {
        ColliderShape::Circle { radius } => radius.is_finite() && radius > 0.0,
        ColliderShape::Rectangle {
            half_width,
            half_height,
        } => {
            half_width.is_finite()
                && half_height.is_finite()
                && half_width > 0.0
                && half_height > 0.0
        }
    };
    if !valid {
        return Err("Collider dimensions must be finite and positive".into());
    }
    Ok(())
}

/// Validate joint endpoints and constraint parameters.
pub fn validate_joint(joint: &Joint2D) -> Result<(), String> {
    if joint.entity_a.is_empty() || joint.entity_b.is_empty() || joint.entity_a == joint.entity_b {
        return Err("Joint endpoints must be different entities".into());
    }
    if !joint
        .anchor_a
        .iter()
        .chain(joint.anchor_b.iter())
        .chain([
            &joint.reference_angle,
            &joint.length,
            &joint.lower_translation,
            &joint.upper_translation,
            &joint.motor_speed,
            &joint.max_motor_torque,
        ])
        .all(|value| value.is_finite())
        || joint.length <= 0.0
        || joint.max_motor_torque < 0.0
        || (joint.enable_limit && joint.lower_translation > joint.upper_translation)
    {
        return Err("Joint parameters must be finite and limits ordered".into());
    }
    Ok(())
}
