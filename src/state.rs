use std::time::{Duration, Instant};

pub const CAT_SIZE: i32 = 192;
pub const TICK_MS: u64 = 30;

#[derive(PartialEq, Clone, Copy)]
pub enum CatState {
    Idle,
    Walking,
    Running,
    #[allow(dead_code)]
    Sleeping,
}

#[derive(PartialEq, Clone, Copy)]
pub enum BehaviorMode {
    Roaming,
    Idle,
    Hunting,
    Dragging,
    Petting,
}

pub struct State {
    pub x: f64,
    pub y: f64,
    pub frame: usize,
    pub flip: bool,
    pub cat_state: CatState,

    pub dragging: bool,
    pub press_start_time: Option<Instant>,
    pub drag_win_x: f64,
    pub drag_win_y: f64,

    pub behavior: BehaviorMode,
    pub behavior_until: Option<Instant>,

    pub vx: f64,
    pub vy: f64,
    pub target_x: f64,
    pub target_y: f64,

    pub rng_counter: usize,
    pub anim_tick: usize,
    pub random_run_until: Option<Instant>,
    pub random_target: Option<(f64, f64)>,
    #[allow(dead_code)]
    pub last_target_change: Instant,
    pub petting_tick: usize,
    pub petting_steps: usize,
    pub click_count: usize,
}

impl State {
    pub fn new(start_x: f64, start_y: f64) -> Self {
        State {
            x: start_x,
            y: start_y,
            frame: 0,
            flip: true,
            cat_state: CatState::Running,
            dragging: false,
            press_start_time: None,
            drag_win_x: 0.0,
            drag_win_y: 0.0,
            behavior: BehaviorMode::Roaming,
            behavior_until: Some(Instant::now() + Duration::from_secs(5)),
            vx: 0.0,
            vy: 0.0,
            target_x: start_x,
            target_y: start_y,
            rng_counter: 0,
            anim_tick: 0,
            random_run_until: Some(Instant::now() + Duration::from_secs(15)),
            random_target: None,
            last_target_change: Instant::now(),
            petting_tick: 0,
            petting_steps: 0,
            click_count: 0,
        }
    }
}
