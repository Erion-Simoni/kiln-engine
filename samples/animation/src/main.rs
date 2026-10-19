//! Sample demonstrating the 2D animation system.

use kiln_anim::{Clip, Ease, Frame, FrameEvent, Tween};

fn main() {
    println!("Kiln 2D Animation System Sample");
    println!("================================\n");

    // Demonstrate spritesheet clips
    println!("1. Spritesheet Clip Demo");
    let mut walk_clip = Clip::new("walk".to_string(), "walk.png".to_string());
    walk_clip.add_frame(Frame {
        rect: [0, 0, 32, 32],
        pivot: [0.5, 0.5],
        duration: 0.1,
        events: vec![],
    });
    walk_clip.add_frame(Frame {
        rect: [32, 0, 32, 32],
        pivot: [0.5, 0.5],
        duration: 0.1,
        events: vec![],
    });
    walk_clip.add_frame(Frame {
        rect: [64, 0, 32, 32],
        pivot: [0.5, 0.5],
        duration: 0.1,
        events: vec![],
    });

    println!("   Clip: {}", walk_clip.name);
    println!("   Texture: {}", walk_clip.texture);
    println!("   Frames: {}", walk_clip.frames.len());
    println!("   Total duration: {:.2}s", walk_clip.total_duration());
    println!("   Loops: {}", walk_clip.loops);

    // Get frame at different times
    for t in [0.0, 0.05, 0.15, 0.25, 0.35] {
        let (frame_idx, local_t) = walk_clip.frame_at_time(t).unwrap();
        println!(
            "   Time {:.2}s: frame {}, local time {:.2}s",
            t, frame_idx, local_t
        );
    }

    // Demonstrate frame events
    println!("\n2. Frame Events Demo");
    let mut attack_clip = Clip::new("attack".to_string(), "attack.png".to_string());
    attack_clip.add_frame(Frame {
        rect: [0, 0, 32, 32],
        pivot: [0.5, 0.5],
        duration: 0.1,
        events: vec![],
    });
    attack_clip.add_frame(Frame {
        rect: [32, 0, 32, 32],
        pivot: [0.5, 0.5],
        duration: 0.1,
        events: vec![FrameEvent {
            name: "hit".to_string(),
            payload: Some("damage".to_string()),
        }],
    });

    let events = attack_clip.events_at_time(0.15);
    println!("   Events at 0.15s: {} events", events.len());
    for event in &events {
        println!("     - {}: {:?}", event.name, event.payload);
    }

    // Demonstrate tween library
    println!("\n3. Tween Library Demo");
    let tween = Tween::new(0.0, 100.0, 1.0, Ease::QuadIn);
    println!(
        "   Tween: {} -> {} over {:.1}s with QuadIn",
        tween.from, tween.to, tween.duration
    );

    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let value = tween.value_at(t).unwrap();
        println!("   Time {:.2}s: {:.2}", t, value);
    }

    // Demonstrate different easing functions
    println!("\n4. Easing Functions Comparison");
    let ease_functions = [
        Ease::Linear,
        Ease::QuadIn,
        Ease::QuadOut,
        Ease::CubicInOut,
        Ease::ElasticOut,
        Ease::BounceOut,
    ];

    for ease in &ease_functions {
        let t = 0.5;
        let eased = ease.apply(t);
        println!("   {:?} at 0.5: {:.3}", ease, eased);
    }

    // Demonstrate looping tween
    println!("\n5. Looping Tween Demo");
    let loop_tween = Tween::new(0.0, 100.0, 1.0, Ease::Linear).with_loops(true);
    for t in [0.0, 0.5, 1.0, 1.5, 2.0] {
        let value = loop_tween.value_at(t).unwrap();
        println!("   Time {:.1}s: {:.2}", t, value);
    }

    println!("\nSample complete!");
}
