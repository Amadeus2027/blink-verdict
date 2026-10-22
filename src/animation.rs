use crate::config::Animation;

pub struct Motion {
    pub opacity: f32,
    pub scale: f32,
    pub offset_y: f32,
}

pub fn sample(effect: Animation, elapsed: f32, remaining: Option<f32>) -> Motion {
    let progress = (elapsed / 0.22).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress).powi(3);
    let outro = remaining.map(|s| (s / 0.3).clamp(0.0, 1.0)).unwrap_or(1.0);
    let mut value = Motion {
        opacity: outro,
        scale: 1.0,
        offset_y: 0.0,
    };
    match effect {
        Animation::None => value.opacity = 1.0,
        Animation::Fade => value.opacity *= progress,
        // A single short emphasis pulse, not repeated blinking.
        Animation::Flash => {
            value.opacity *= if elapsed < 0.08 {
                1.0
            } else if elapsed < 0.16 {
                0.65
            } else {
                1.0
            }
        }
        Animation::Pop => {
            value.scale = 0.78 + 0.22 * eased;
            value.opacity *= progress;
        }
        Animation::Slide => {
            value.offset_y = 28.0 * (1.0 - eased);
            value.opacity *= progress;
        }
    }
    value
}

/// No idle animation polling: sleep until the exit phase, or stop when staying visible.
pub fn next_tick(effect: Animation, elapsed: f32, remaining: Option<f32>) -> Option<u32> {
    if effect != Animation::None && (elapsed < 0.25 || remaining.is_some_and(|s| s <= 0.32)) {
        Some(16)
    } else {
        remaining.map(|s| {
            ((s - if effect == Animation::None { 0.0 } else { 0.3 }).max(0.001) * 1000.0) as u32
        })
    }
}
