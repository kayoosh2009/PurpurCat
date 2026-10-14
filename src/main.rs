use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, DrawingArea};
use gtk4_layer_shell::{Layer, LayerShell};
use glib::timeout_add_local;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

const CAT_SIZE: i32 = 128;
const TICK_MS: u64 = 120;

const SPRITES: &[&str] = &[
    "cat/main.png",
    "cat/move 1.png",
    "cat/move 2.png",
    "cat/move 3.png",
];

struct State {
    x: f64,
    y: f64,
    frame: usize,
    flip: bool,
    dragging: bool,
    drag_start_x: f64,
    drag_start_y: f64,
    drag_win_x: f64,
    drag_win_y: f64,
    cursor_vx: f64,
    cursor_vy: f64,
    prev_cursor_x: f64,
    prev_cursor_y: f64,
}

impl State {
    fn new() -> Self {
        State {
            x: 200.0,
            y: 400.0,
            frame: 0,
            flip: false,
            dragging: false,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            drag_win_x: 0.0,
            drag_win_y: 0.0,
            cursor_vx: 0.0,
            cursor_vy: 0.0,
            prev_cursor_x: 0.0,
            prev_cursor_y: 0.0,
        }
    }
}

fn load_textures() -> Vec<gdk_pixbuf::Pixbuf> {
    SPRITES.iter().map(|path| {
        gdk_pixbuf::Pixbuf::from_file_at_scale(path, CAT_SIZE, CAT_SIZE, true)
            .unwrap_or_else(|e| panic!("Не могу загрузить {path}: {e}"))
    }).collect()
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

        // Layer shell настройка
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(gtk4_layer_shell::Edge::Left, true);
        window.set_anchor(gtk4_layer_shell::Edge::Top, true);
        window.set_margin(gtk4_layer_shell::Edge::Left, 200);
        window.set_margin(gtk4_layer_shell::Edge::Top, 400);
        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);

        let textures = Rc::new(load_textures());
        let state = Rc::new(RefCell::new(State::new()));

        let drawing = DrawingArea::new();
        drawing.set_size_request(CAT_SIZE, CAT_SIZE);

        // Рисуем кота
        let textures_draw = textures.clone();
        let state_draw = state.clone();
        drawing.set_draw_func(move |_, cr, w, h| {
            let s = state_draw.borrow();
            let tex = &textures_draw[s.frame % textures_draw.len()];

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
        drag.connect_drag_begin(move |_, x, y| {
            let mut s = state_begin.borrow_mut();
            s.dragging = true;
            s.drag_start_x = x;
            s.drag_start_y = y;
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
        timeout_add_local(Duration::from_millis(TICK_MS), move || {
            let mut s = state_tick.borrow_mut();

            s.frame = (s.frame + 1) % SPRITES.len();

            // Погоня за курсором
            let speed = (s.cursor_vx * s.cursor_vx + s.cursor_vy * s.cursor_vy).sqrt();
            if !s.dragging && speed > 8.0 {
                let step = if speed > 20.0 { 8.0 } else { 4.0 };
                let dx = s.cursor_vx;
                let dy = s.cursor_vy;
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                s.x += dx / dist * step;
                s.y += dy / dist * step;
                s.flip = dx < 0.0;
                window_tick.set_margin(gtk4_layer_shell::Edge::Left, s.x as i32);
                window_tick.set_margin(gtk4_layer_shell::Edge::Top, s.y as i32);
            }

            s.cursor_vx *= 0.3;
            s.cursor_vy *= 0.3;

            drawing_tick.queue_draw();
            glib::ControlFlow::Continue
        });
    });

    app.run();
}