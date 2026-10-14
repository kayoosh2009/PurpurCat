use glib::timeout_add_local;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, DrawingArea};
use gtk4_layer_shell::{Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

const CAT_SIZE: i32 = 192;
const TICK_MS: u64 = 20; // Около 50 кадров в секунду для идеальной плавности

// Спрайты анимаций
const IDLE_SPRITES: &[&str] = &[
    "cat/move 4.png",
    "cat/move 5.png",
    "cat/move 6.png",
    "cat/move 7.png",
];
const WALK_SPRITES: &[&str] = &["cat/move 1.png", "cat/move 2.png"];
const RUN_SPRITES: &[&str] = &["cat/move 2.png", "cat/move 3.png"];

#[derive(PartialEq, Clone, Copy)]
enum CatState {
    Idle,
    Walking,
    Running,
}

struct State {
    x: f64,
    y: f64,
    frame: usize,
    flip: bool,
    cat_state: CatState,

    // Перетаскивание по зажатию (3 сек)
    dragging: bool,
    press_start_time: Option<Instant>,
    drag_win_x: f64,
    drag_win_y: f64,

    // Режимы поведения
    is_hunting: bool, // Охота за мышкой при клике

    // Скорость и физика
    vx: f64,
    vy: f64,
    target_x: f64,
    target_y: f64,

    random_run_until: Option<Instant>,
    random_target: Option<(f64, f64)>,
    last_target_change: Instant,
    rng_counter: usize,
}

impl State {
    fn new(start_x: f64, start_y: f64) -> Self {
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
            is_hunting: false,
            vx: 0.0,
            vy: 0.0,
            target_x: start_x,
            target_y: start_y,
            random_run_until: Some(Instant::now() + Duration::from_secs(15)),
            random_target: None,
            last_target_change: Instant::now(),
            rng_counter: 0,
        }
    }
}

fn pseudo_rand(seed: usize) -> f64 {
    let x = (seed as u64)
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((x >> 33) as f64) / (u32::MAX as f64)
}

fn rand_range(min: f64, max: f64, seed: usize) -> f64 {
    min + pseudo_rand(seed) * (max - min)
}

fn rand_duration_secs(seed: usize) -> Duration {
    let secs = 5 + (pseudo_rand(seed) * 11.0) as u64; // 5..15 секунд бега
    Duration::from_secs(secs)
}

fn load_pixbufs(paths: &[&str]) -> Vec<gdk_pixbuf::Pixbuf> {
    paths
        .iter()
        .map(|path| {
            gdk_pixbuf::Pixbuf::from_file_at_scale(path, CAT_SIZE, CAT_SIZE, true)
                .unwrap_or_else(|e| panic!("Не могу загрузить {path}: {e}"))
        })
        .collect()
}

fn main() {
    let app = Application::builder()
        .application_id("com.example.miru")
        .build();

    app.connect_activate(|app| {
        // Получаем размеры экрана заранее для корректного стартового положения
        let display = gdk4::Display::default().unwrap();
        let monitor = display.monitors().item(0).expect("Нет мониторов");
        let monitor = monitor.downcast::<gdk4::Monitor>().expect("Не монитор");
        let geometry = monitor.geometry();
        let screen_width = geometry.width() as f64;
        let screen_height = geometry.height() as f64;

        // Стартовая позиция: правый нижний угол
        let start_x = screen_width - CAT_SIZE as f64 - 50.0;
        let start_y = screen_height - CAT_SIZE as f64 - 50.0;

        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(CAT_SIZE)
            .default_height(CAT_SIZE)
            .decorated(false)
            .build();

        // Прозрачность
        let css = gtk4::CssProvider::new();
        css.load_from_data("window { background: transparent; }");
        gtk4::style_context_add_provider_for_display(
            &gdk4::Display::default().unwrap(),
            &css,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // Layer shell
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(gtk4_layer_shell::Edge::Left, true);
        window.set_anchor(gtk4_layer_shell::Edge::Top, true);
        window.set_margin(gtk4_layer_shell::Edge::Left, start_x as i32);
        window.set_margin(gtk4_layer_shell::Edge::Top, start_y as i32);
        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);

        let idle_frames = Rc::new(load_pixbufs(IDLE_SPRITES));
        let walk_frames = Rc::new(load_pixbufs(WALK_SPRITES));
        let run_frames = Rc::new(load_pixbufs(RUN_SPRITES));

        let state = Rc::new(RefCell::new(State::new(start_x, start_y)));

        let drawing = DrawingArea::new();
        drawing.set_size_request(CAT_SIZE, CAT_SIZE);

        let idle_draw = idle_frames.clone();
        let walk_draw = walk_frames.clone();
        let run_draw = run_frames.clone();
        let state_draw = state.clone();

        drawing.set_draw_func(move |_, cr, w, _h| {
            let s = state_draw.borrow();

            let frames = match s.cat_state {
                CatState::Idle => &idle_draw,
                CatState::Walking => &walk_draw,
                CatState::Running => &run_draw,
            };
            let tex = &frames[s.frame % frames.len()];

            // Прозрачный фон
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
            cr.set_operator(cairo::Operator::Source);
            cr.paint().unwrap();

            cr.save().unwrap();
            if s.flip {
                cr.translate(w as f64, 0.0);
                cr.scale(-1.0, 1.0);
            }
            cr.set_source_pixbuf(tex, 0.0, 0.0);
            cr.paint().unwrap();
            cr.restore().unwrap();
        });

        window.set_child(Some(&drawing));

        // Глобальные координаты курсора
        let cursor_x_ref = Rc::new(RefCell::new(0.0));
        let cursor_y_ref = Rc::new(RefCell::new(0.0));

        // Трекинг позиции курсора
        let motion = gtk4::EventControllerMotion::new();
        let cx_motion = cursor_x_ref.clone();
        let cy_motion = cursor_y_ref.clone();
        motion.connect_motion(move |_, mx, my| {
            *cx_motion.borrow_mut() = mx;
            *cy_motion.borrow_mut() = my;
        });
        drawing.add_controller(motion);

        // Обработчик кликов и зажатий
        let click = gtk4::GestureClick::new();
        click.set_button(gdk4::BUTTON_PRIMARY);

        let state_press = state.clone();
        click.connect_pressed(move |_, n_press, _, _| {
            let mut s = state_press.borrow_mut();
            if n_press == 1 {
                s.press_start_time = Some(Instant::now());
                s.drag_win_x = s.x;
                s.drag_win_y = s.y;
            } else if n_press == 2 {
                // Двойной клик — рандомный бег
                s.is_hunting = false;
                s.rng_counter = s.rng_counter.wrapping_add(1);
                s.random_run_until = Some(Instant::now() + rand_duration_secs(s.rng_counter));
                s.random_target = None;
            }
        });

        let state_released = state.clone();
        click.connect_released(move |_, n_press, _, _| {
            let mut s = state_released.borrow_mut();
            s.press_start_time = None;

            if s.dragging {
                s.dragging = false;
            } else if n_press == 1 {
                // Одиночный клик — переключение режима охоты за мышкой
                s.is_hunting = !s.is_hunting;
                s.random_run_until = None;
            }
        });

        drawing.add_controller(click);
        window.present();

        // Главный игровой тик
        let state_tick = state.clone();
        let drawing_tick = drawing.clone();
        let window_tick = window.clone();
        let cursor_x_tick = cursor_x_ref.clone();
        let cursor_y_tick = cursor_y_ref.clone();

        timeout_add_local(Duration::from_millis(TICK_MS), move || {
            let mut s = state_tick.borrow_mut();
            let now = Instant::now();

            // 1. Проверка зажатия мыши на 3 секунды для начала перетаскивания
            if let Some(start_time) = s.press_start_time {
                if !s.dragging && now.duration_since(start_time) >= Duration::from_secs(3) {
                    s.dragging = true;
                    s.is_hunting = false;
                    s.random_run_until = None;
                }
            }

            // Если кота перетаскивают мышкой:
            if s.dragging {
                let mx = *cursor_x_tick.borrow();
                let my = *cursor_y_tick.borrow();
                s.x = s.drag_win_x + mx;
                s.y = s.drag_win_y + my;

                s.x = s.x.clamp(0.0, screen_width - CAT_SIZE as f64);
                s.y = s.y.clamp(0.0, screen_height - CAT_SIZE as f64);

                window_tick.set_margin(gtk4_layer_shell::Edge::Left, s.x as i32);
                window_tick.set_margin(gtk4_layer_shell::Edge::Top, s.y as i32);

                s.cat_state = CatState::Idle;
                drawing_tick.queue_draw();
                return glib::ControlFlow::Continue;
            }

            // 2. Обработка режимов движения
            let in_random_run = s.random_run_until.map_or(false, |until| now < until);

            if in_random_run {
                let need_new_target = s.random_target.is_none()
                    || now.duration_since(s.last_target_change).as_millis() > 800;
                if need_new_target {
                    s.rng_counter = s.rng_counter.wrapping_add(1);
                    let tx = rand_range(50.0, screen_width - CAT_SIZE as f64 - 50.0, s.rng_counter);
                    s.rng_counter = s.rng_counter.wrapping_add(1);
                    let ty =
                        rand_range(50.0, screen_height - CAT_SIZE as f64 - 50.0, s.rng_counter);
                    s.random_target = Some((tx, ty));
                    s.last_target_change = now;
                }
                if let Some((tx, ty)) = s.random_target {
                    s.target_x = tx;
                    s.target_y = ty;
                }
            } else if s.is_hunting {
                let mx = *cursor_x_tick.borrow();
                let my = *cursor_y_tick.borrow();
                s.target_x = s.x + mx - (CAT_SIZE as f64 / 2.0);
                s.target_y = s.y + my - (CAT_SIZE as f64 / 2.0);
            } else {
                if s.random_run_until.is_some() {
                    s.random_run_until = None;
                    s.random_target = None;
                }
                s.target_x = s.x;
                s.target_y = s.y;
            }

            // 3. Плавная физика движения (Lerp)
            let dx = s.target_x - s.x;
            let dy = s.target_y - s.y;
            let dist = (dx * dx + dy * dy).sqrt();
            let speed_threshold = 3.0;
            if dist > speed_threshold {
                // Увеличили скорости, чтобы котик шустро бегал и ходил
                let max_speed = if in_random_run { 35.0 } else { 14.0 };
                let target_vx = (dx / dist) * max_speed;
                let target_vy = (dy / dist) * max_speed;

                // Плавное ускорение под 20мс тики
                s.vx += (target_vx - s.vx) * 0.15;
                s.vy += (target_vy - s.vy) * 0.15;

                s.x += s.vx;
                s.y += s.vy;

                if s.vx.abs() > 0.5 {
                    s.flip = s.vx < 0.0;
                }

                let current_speed = (s.vx * s.vx + s.vy * s.vy).sqrt();
                let desired_state = if current_speed > 8.0 {
                    CatState::Running
                } else {
                    CatState::Walking
                };

                if s.cat_state != desired_state {
                    s.cat_state = desired_state;
                    s.frame = 0;
                } else {
                    let max_frames = match s.cat_state {
                        CatState::Walking => WALK_SPRITES.len(),
                        CatState::Running => RUN_SPRITES.len(),
                        CatState::Idle => IDLE_SPRITES.len(),
                    };
                    s.frame = (s.frame + 1) % max_frames;
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
                    } else {
                        s.frame = (s.frame + 1) % IDLE_SPRITES.len();
                    }
                }
            }

            s.x = s.x.clamp(0.0, screen_width - CAT_SIZE as f64);
            s.y = s.y.clamp(0.0, screen_height - CAT_SIZE as f64);

            window_tick.set_margin(gtk4_layer_shell::Edge::Left, s.x as i32);
            window_tick.set_margin(gtk4_layer_shell::Edge::Top, s.y as i32);

            drawing_tick.queue_draw();
            glib::ControlFlow::Continue
        });
    });

    app.run();
}