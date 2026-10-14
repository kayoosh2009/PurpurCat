use glib::timeout_add_local;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, DrawingArea};
use gtk4_layer_shell::{Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

const CAT_SIZE: i32 = 192;
const TICK_MS: u64 = 150;
const IDLE_SPRITES: &[&str] = &["cat/main.png"];
const MOVE_SPRITES: &[&str] = &["cat/main.png"];

#[derive(PartialEq)]
enum CatState {
    Idle,
    Moving,
}

struct State {
    x: f64,
    y: f64,
    frame: usize,
    flip: bool,
    cat_state: CatState,
    dragging: bool,
    drag_win_x: f64,
    drag_win_y: f64,
    cursor_vx: f64,
    cursor_vy: f64,
    prev_cursor_x: f64,
    prev_cursor_y: f64,
    random_run_until: Option<Instant>,
    random_target: Option<(f64, f64)>,
    last_target_change: Instant,
    rng_counter: usize,
}

impl State {
    fn new() -> Self {
        State {
            x: 200.0,
            y: 400.0,
            frame: 0,
            flip: false,
            cat_state: CatState::Idle,
            dragging: false,
            drag_win_x: 0.0,
            drag_win_y: 0.0,
            cursor_vx: 0.0,
            cursor_vy: 0.0,
            prev_cursor_x: 0.0,
            prev_cursor_y: 0.0,
            random_run_until: None,
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
    let secs = 5 + (pseudo_rand(seed) * 26.0) as u64; // 5..30 секунд
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

        // Layer shell — только один раз
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(gtk4_layer_shell::Edge::Left, true);
        window.set_anchor(gtk4_layer_shell::Edge::Top, true);
        window.set_margin(gtk4_layer_shell::Edge::Left, 200);
        window.set_margin(gtk4_layer_shell::Edge::Top, 400);
        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);

        let idle_frames = Rc::new(load_pixbufs(IDLE_SPRITES));
        let move_frames = Rc::new(load_pixbufs(MOVE_SPRITES));
        let state = Rc::new(RefCell::new(State::new()));

        let drawing = DrawingArea::new();
        drawing.set_size_request(CAT_SIZE, CAT_SIZE);

        let idle_draw = idle_frames.clone();
        let move_draw = move_frames.clone();
        let state_draw = state.clone();
        drawing.set_draw_func(move |_, cr, w, _h| {
            let s = state_draw.borrow();

            let frames = if s.cat_state == CatState::Idle {
                &idle_draw
            } else {
                &move_draw
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

        // Перетаскивание
        let drag = gtk4::GestureDrag::new();

        let state_begin = state.clone();
        drag.connect_drag_begin(move |_, _, _| {
            let mut s = state_begin.borrow_mut();
            s.dragging = true;
            s.drag_win_x = s.x;
            s.drag_win_y = s.y;
        });

        let state_update = state.clone();
        let window_update = window.clone();
        drag.connect_drag_update(move |_, dx, dy| {
            let mut s = state_update.borrow_mut();
            if s.dragging {
                s.x = s.drag_win_x + dx;
                s.y = s.drag_win_y + dy;
                window_update.set_margin(gtk4_layer_shell::Edge::Left, s.x as i32);
                window_update.set_margin(gtk4_layer_shell::Edge::Top, s.y as i32);
            }
        });

        let state_end = state.clone();
        drag.connect_drag_end(move |_, _, _| {
            state_end.borrow_mut().dragging = false;
        });

        drawing.add_controller(drag);

        // Трекинг курсора
        let motion = gtk4::EventControllerMotion::new();
        let state_motion = state.clone();
        motion.connect_motion(move |_, mx, my| {
            let mut s = state_motion.borrow_mut();
            s.cursor_vx = mx - s.prev_cursor_x;
            s.cursor_vy = my - s.prev_cursor_y;
            s.prev_cursor_x = mx;
            s.prev_cursor_y = my;
        });
        drawing.add_controller(motion);

        window.present();

        // Тик
        let state_tick = state.clone();
        let drawing_tick = drawing.clone();
        let window_tick = window.clone();
        // Получаем размер экрана для ограничения границ
        let display = gdk4::Display::default().unwrap();
        let monitor = display.monitors().item(0).expect("Нет мониторов");
        let monitor = monitor.downcast::<gdk4::Monitor>().expect("Не монитор");
        let geometry = monitor.geometry();
        let screen_width = geometry.width() as f64;
        let screen_height = geometry.height() as f64;

        timeout_add_local(Duration::from_millis(TICK_MS), move || {
            let mut s = state_tick.borrow_mut();
            let speed = (s.cursor_vx * s.cursor_vx + s.cursor_vy * s.cursor_vy).sqrt();

            if !s.dragging && speed > 1.5 {
                // Режим движения
                if s.cat_state != CatState::Moving {
                    s.cat_state = CatState::Moving;
                    s.frame = 0;
                } else {
                    s.frame = (s.frame + 1) % MOVE_SPRITES.len();
                }

                // Медленная плавная скорость (15 пикселей за тик)
                let target_speed = 15.0;
                let dist = speed.max(1.0);
                s.x += (s.cursor_vx / dist) * target_speed;
                s.y += (s.cursor_vy / dist) * target_speed;

                // Переворачиваем спрайт только при уверенном движении по X
                if s.cursor_vx.abs() > 2.0 {
                    s.flip = s.cursor_vx < 0.0;
                }

                // Ограничиваем границы экрана
                s.x = s.x.clamp(0.0, screen_width - CAT_SIZE as f64);
                s.y = s.y.clamp(0.0, screen_height - CAT_SIZE as f64);

                window_tick.set_margin(gtk4_layer_shell::Edge::Left, s.x as i32);
                window_tick.set_margin(gtk4_layer_shell::Edge::Top, s.y as i32);

                // Очень плавное торможение (инерция)
                s.cursor_vx *= 0.92;
                s.cursor_vy *= 0.92;
            } else {
                // Idle
                if s.cat_state != CatState::Idle {
                    s.cat_state = CatState::Idle;
                    s.frame = 0;
                } else {
                    s.frame = (s.frame + 1) % IDLE_SPRITES.len();
                }
                // Плавное затухание в покое
                s.cursor_vx *= 0.85;
                s.cursor_vy *= 0.85;
            }

            drawing_tick.queue_draw();
            glib::ControlFlow::Continue
        });
    });

    app.run();
}
