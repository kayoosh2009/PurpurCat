use crate::anim;
use crate::state::{BehaviorMode, CatState, State, CAT_SIZE};
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

pub fn tick(
    s: &mut State,
    now: Instant,
    screen_width: f64,
    screen_height: f64,
    cursor_x: f64,
    cursor_y: f64,
) {
    // 1. Перетаскивание
    if s.dragging {
        s.cat_state = CatState::Idle;
        s.vx = 0.0;
        s.vy = 0.0;
        return;
    }

    // 2.5. Поглаживание и сон
    if s.behavior == BehaviorMode::Petting {
        s.petting_tick += 1;

        if s.petting_steps < 3 {
            // Делаем несколько шагов перед тем, как лечь
            s.x += if s.flip { -1.5 } else { 1.5 };
            s.cat_state = CatState::Walking;
            if s.petting_tick % 6 == 0 {
                s.frame = (s.frame + 1) % anim::run::SPRITES.len();
                s.petting_steps += 1;
            }
        } else {
            if s.cat_state != CatState::SleepIntro && s.cat_state != CatState::Sleeping {
                s.cat_state = CatState::SleepIntro;
                s.frame = 0;
                s.anim_tick = 0;
            }

            if s.cat_state == CatState::SleepIntro {
                s.anim_tick += 1;
                // 1 секунда между кадрами (33 тика по 30мс)
                if s.anim_tick >= 33 {
                    s.anim_tick = 0;
                    s.frame += 1;
                    if s.frame >= anim::sleep::SPRITES_INTRO.len() {
                        s.cat_state = CatState::Sleeping;
                        s.frame = 0;
                    }
                }
            } else if s.cat_state == CatState::Sleeping {
                s.anim_tick += 1;
                // Медленная смена кадров сна 5 <-> 6 (каждые ~0.75 сек)
                if s.anim_tick >= 25 {
                    s.anim_tick = 0;
                    s.frame = (s.frame + 1) % anim::sleep::SPRITES_LOOP.len();
                }

                // Кот спит около 15 секунд, затем просыпается
                if s.petting_tick >= 600 {
                    s.behavior = BehaviorMode::Roaming;
                    s.cat_state = CatState::Idle;
                    s.petting_tick = 0;
                    s.petting_steps = 0;
                    s.frame = 0;
                    s.behavior_until = Some(now + Duration::from_secs(5));
                }
            }
        }
        return;
    }

    // 3. Автоматическое переключение поведения
    if s.behavior == BehaviorMode::Roaming || s.behavior == BehaviorMode::Idle {
        let expired = s.behavior_until.map_or(true, |until| now >= until);
        if expired {
            s.rng_counter = s.rng_counter.wrapping_add(1);
            let roll = pseudo_rand(s.rng_counter);
            if roll < 0.6 {
                s.behavior = BehaviorMode::Roaming;
                s.rng_counter = s.rng_counter.wrapping_add(1);
                let tx = rand_range(50.0, screen_width - CAT_SIZE as f64 - 50.0, s.rng_counter);
                s.rng_counter = s.rng_counter.wrapping_add(1);
                let ty = rand_range(50.0, screen_height - CAT_SIZE as f64 - 50.0, s.rng_counter);
                s.random_target = Some((tx, ty));
                s.behavior_until = Some(now + Duration::from_secs(8));
            } else {
                s.behavior = BehaviorMode::Idle;
                s.random_target = None;
                s.behavior_until = Some(now + Duration::from_secs(4));
            }
        }
    }

    // 4. Цель
    match s.behavior {
        BehaviorMode::Roaming => {
            if let Some((tx, ty)) = s.random_target {
                s.target_x = tx;
                s.target_y = ty;
            }
        }
        BehaviorMode::Hunting => {
            s.target_x = s.x + cursor_x - (CAT_SIZE as f64 / 2.0);
            s.target_y = s.y + cursor_y - (CAT_SIZE as f64 / 2.0);
        }
        BehaviorMode::Idle | BehaviorMode::Dragging | BehaviorMode::Petting => {
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
            BehaviorMode::Hunting => 7.0,
            _ => 3.0,
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
            if s.anim_tick % 6 == 0 {
                let max_frames = match s.cat_state {
                    CatState::Walking => anim::run::SPRITES.len(),
                    CatState::Idle => anim::idle::SPRITES.len(),
                    CatState::SleepIntro => anim::sleep::SPRITES_INTRO.len(),
                    CatState::Sleeping => anim::sleep::SPRITES_LOOP.len(),
                    CatState::Running => anim::run::SPRITES.len(),
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
