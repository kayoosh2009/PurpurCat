use crate::state::{BehaviorMode, CatState, State, CAT_SIZE};
use crate::anim;
use std::time::{Duration, Instant};

pub fn pseudo_rand(seed: usize) -> f64 {
    let x = (seed as u64)
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((x >> 33) as f64) / (u32::MAX as f64)
}

pub fn rand_range(min: f64, max: f64, seed: usize) -> f64 {
    min + pseudo_rand(seed) * (max - min)
}

pub fn rand_duration_secs(seed: usize) -> Duration {
    let secs = 5 + (pseudo_rand(seed) * 11.0) as u64;
    Duration::from_secs(secs)
}

pub fn tick(
    s: &mut State,
    now: Instant,
    screen_width: f64,
    screen_height: f64,
    cursor_x: f64,
    cursor_y: f64,
) {
    // 1. Проверка зажатия на 3 сек
    if let Some(start_time) = s.press_start_time {
        if !s.dragging && now.duration_since(start_time) >= Duration::from_secs(3) {
            s.dragging = true;
            s.behavior = BehaviorMode::Dragging;
            s.random_run_until = None;
        }
    }

    // 2. Перетаскивание
    if s.dragging {
        s.x = (s.drag_win_x + cursor_x).clamp(0.0, screen_width - CAT_SIZE as f64);
        s.y = (s.drag_win_y + cursor_y).clamp(0.0, screen_height - CAT_SIZE as f64);
        s.cat_state = CatState::Idle;
        return;
    }

    // 3. Автоматическое переключение поведения
    if s.behavior == BehaviorMode::Roaming || s.behavior == BehaviorMode::Idle {
        let expired = s.behavior_until.map_or(true, |until| now >= until);
        if expired {
            s.rng_counter = s.rng_counter.wrapping_add(1);
            let roll = pseudo_rand(s.rng_counter);
            if roll < 0.4 {
                s.behavior = BehaviorMode::Roaming;
                s.rng_counter = s.rng_counter.wrapping_add(1);
                let tx = rand_range(50.0, screen_width - CAT_SIZE as f64 - 50.0, s.rng_counter);
                s.rng_counter = s.rng_counter.wrapping_add(1);
                let ty = rand_range(50.0, screen_height - CAT_SIZE as f64 - 50.0, s.rng_counter);
                s.random_target = Some((tx, ty));
                s.behavior_until = Some(now + Duration::from_secs(8));
            } else if roll < 0.7 {
                s.behavior = BehaviorMode::Idle;
                s.random_target = None;
                s.behavior_until = Some(now + Duration::from_secs(4));
            } else {
                s.behavior = BehaviorMode::Running;
                s.rng_counter = s.rng_counter.wrapping_add(1);
                let tx = rand_range(50.0, screen_width - CAT_SIZE as f64 - 50.0, s.rng_counter);
                s.rng_counter = s.rng_counter.wrapping_add(1);
                let ty = rand_range(50.0, screen_height - CAT_SIZE as f64 - 50.0, s.rng_counter);
                s.random_target = Some((tx, ty));
                s.behavior_until = Some(now + Duration::from_secs(5));
            }
        }
    }

    if s.behavior == BehaviorMode::Running {
        let expired = s.behavior_until.map_or(true, |until| now >= until);
        if expired {
            s.behavior = BehaviorMode::Roaming;
            s.behavior_until = None;
        }
    }

    // 4. Цель
    match s.behavior {
        BehaviorMode::Roaming | BehaviorMode::Running => {
            if let Some((tx, ty)) = s.random_target {
                s.target_x = tx;
                s.target_y = ty;
            }
        }
        BehaviorMode::Hunting => {
            s.target_x = s.x + cursor_x - (CAT_SIZE as f64 / 2.0);
            s.target_y = s.y + cursor_y - (CAT_SIZE as f64 / 2.0);
        }
        BehaviorMode::Idle | BehaviorMode::Dragging => {
            s.target_x = s.x;
            s.target_y = s.y;
        }
    }

    // 5. Физика
    let dx = s.target_x - s.x;
    let dy = s.target_y - s.y;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist > 3.0 {
        let max_speed = match s.behavior {
            BehaviorMode::Running => 18.0,
            BehaviorMode::Hunting => 12.0,
            _ => 5.0,
        };
        let target_vx = (dx / dist) * max_speed;
        let target_vy = (dy / dist) * max_speed;

        s.vx += (target_vx - s.vx) * 0.15;
        s.vy += (target_vy - s.vy) * 0.15;
        s.x += s.vx;
        s.y += s.vy;

        if s.vx.abs() > 0.5 {
            s.flip = s.vx < 0.0;
        }

        let speed = (s.vx * s.vx + s.vy * s.vy).sqrt();
        let desired = if speed > 8.0 {
            CatState::Running
        } else {
            CatState::Walking
        };

        if s.cat_state != desired {
            s.cat_state = desired;
            s.frame = 0;
            s.anim_tick = 0;
        } else {
            s.anim_tick += 1;
            if s.anim_tick % 3 == 0 {
                let max_frames = match s.cat_state {
                    CatState::Walking => anim::walk::SPRITES.len(),
                    CatState::Running => anim::run::SPRITES.len(),
                    CatState::Idle => anim::idle::SPRITES.len(),
                    CatState::Sleeping => anim::sleep::SPRITES_LOOP.len(),
                };
                s.frame = (s.frame + 1) % max_frames;
            }
        }
    } else {
        s.vx *= 0.7;
        s.vy *= 0.7;
        s.x += s.vx;
        s.y += s.vy;

        if s.vx.abs() < 0.2 && s.vy.abs() < 0.2 {
            if s.cat_state != CatState::Idle {
                s.cat_state = CatState::Idle;
                s.frame = 0;
                s.anim_tick = 0;
            } else {
                s.anim_tick += 1;
                if s.anim_tick % 4 == 0 {
                    s.frame = (s.frame + 1) % anim::idle::SPRITES.len();
                }
            }
        }
    }

    s.x = s.x.clamp(0.0, screen_width - CAT_SIZE as f64);
    s.y = s.y.clamp(0.0, screen_height - CAT_SIZE as f64);
}
