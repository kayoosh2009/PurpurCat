use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, DrawingArea};
use gdk4::Display;
use glib::timeout_add_local;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

const SCALE: i32 = 3;
const CAT_SIZE: i32 = 80 * SCALE;
const TICK_MS: u64 = 120; // скорость смены кадров

// Пути к спрайтам
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
    flip: bool, // смотрит влево/вправо

    // для перетаскивания
    dragging: bool,
    drag_offset_x: f64,
    drag_offset_y: f64,

    // для определения скорости курсора
    cursor_x: f64,
    cursor_y: f64,
    cursor_vx: f64,
    cursor_vy: f64,
    last_tick: Instant,
}

impl State {
    fn new() -> Self {
        State {
            x: 200.0,
            y: 200.0,
            frame: 0,
            flip: false,
            dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_vx: 0.0,
            cursor_vy: 0.0,
            last_tick: Instant::now(),
        }
    }

    fn cursor_speed(&self) -> f64 {
        (self.cursor_vx * self.cursor_vx + self.cursor_vy * self.cursor_vy).sqrt()
    }
}

fn load_textures() -> Vec<gtk4::gdk::Texture> {
    SPRITES.iter().map(|path| {
        gtk4::gdk::Texture::from_filename(path)
            .unwrap_or_else(|_| panic!("Не могу загрузить спрайт: {path}"))
    }).collect()
}

fn main() {
    let app = Application::builder()
        .application_id("com.example.miru")
        .build();

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Miru")
            .default_width(CAT_SIZE)
            .default_height(CAT_SIZE)
            .decorated(false)
            .resizable(false)
            .build();

        // Прозрачный фон
        window.set_opacity(1.0);
        gtk4::Widget::set_css_classes(&window, &["miru-window"]);

        let css = gtk4::CssProvider::new();
        css.load_from_data("window.miru-window { background: transparent; }");
        gtk4::style_context_add_provider_for_display(
            &Display::default().unwrap(),
            &css,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        window.set_app_paintable(true);

        let textures = Rc::new(load_textures());
        let state = Rc::new(RefCell::new(State::new()));

        // Drawing area
        let drawing = DrawingArea::new();
        drawing.set_size_request(CAT_SIZE, CAT_SIZE);

        let textures_draw = textures.clone();
        let state_draw = state.clone();
        drawing.set_draw_func(move |_, cr, _w, _h| {
            let s = state_draw.borrow();
            let tex = &textures_draw[s.frame % textures_draw.len()];

            cr.save().unwrap();
            if s.flip {
                cr.translate(CAT_SIZE as f64, 0.0);
                cr.scale(-1.0, 1.0);
            }
            cr.rectangle(0.0, 0.0, CAT_SIZE as f64, CAT_SIZE as f64);
            cr.set_operator(cairo::Operator::Source);
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
            cr.fill().unwrap();

            tex.draw_to_cairo(cr, 0.0, 0.0, CAT_SIZE as f64, CAT_SIZE as f64)
                .unwrap();
            cr.restore().unwrap();
        });

        window.set_child(Some(&drawing));

        // --- Мышь: перетаскивание ---
        let drag_ctrl = gtk4::GestureDrag::new();
        let state_drag_begin = state.clone();
        let window_begin = window.clone();
        drag_ctrl.connect_drag_begin(move |_, _x, _y| {
            let mut s = state_drag_begin.borrow_mut();
            let (wx, wy) = window_begin.position();
            // запомним смещение внутри окна
            s.dragging = true;
            s.drag_offset_x = wx as f64;
            s.drag_offset_y = wy as f64;
        });

        let state_drag_update = state.clone();
        let window_drag = window.clone();
        drag_ctrl.connect_drag_update(move |ctrl, _dx, _dy| {
            let s = state_drag_update.borrow();
            if s.dragging {
                if let Some((start_x, start_y)) = ctrl.start_point() {
                    // Абсолютная позиция = начало окна + начало клика + смещение
                    let (wx, _) = window_drag.position();
                    // Проще: просто двигаем окно
                    let _ = wx; // подавляем warning
                }
            }
        });

        let state_drag_end = state.clone();
        drag_ctrl.connect_drag_end(move |_, _, _| {
            state_drag_end.borrow_mut().dragging = false;
        });
        drawing.add_controller(drag_ctrl);

        // Трекинг курсора через motion controller на весь экран
        let motion = gtk4::EventControllerMotion::new();
        let state_motion = state.clone();
        let drawing_motion = drawing.clone();
        motion.connect_motion(move |_, mx, my| {
            // mx/my — координаты относительно виджета
            // переводим в экранные
            let mut s = state_motion.borrow_mut();
            let prev_x = s.cursor_x;
            let prev_y = s.cursor_y;
            s.cursor_x = mx;
            s.cursor_y = my;
            s.cursor_vx = mx - prev_x;
            s.cursor_vy = my - prev_y;
            drawing_motion.queue_draw();
        });
        drawing.add_controller(motion);

        // Позиционируем окно
        {
            let s = state.borrow();
            window.move_(s.x as i32, s.y as i32);
        }

        window.set_keep_above(true);
        window.present();

        // Игровой тик
        let state_tick = state.clone();
        let drawing_tick = drawing.clone();
        let window_tick = window.clone();
        timeout_add_local(Duration::from_millis(TICK_MS), move || {
            let mut s = state_tick.borrow_mut();

            s.frame = (s.frame + 1) % SPRITES.len();

            // Если курсор быстро движется — бежим к нему
            let speed = s.cursor_speed();
            if !s.dragging && speed > 5.0 {
                let dx = s.cursor_x - CAT_SIZE as f64 / 2.0;
                let dy = s.cursor_y - CAT_SIZE as f64 / 2.0;
                let cat_speed = if speed > 20.0 { 6.0 } else { 3.0 };
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                s.x += dx / dist * cat_speed;
                s.y += dy / dist * cat_speed;
                s.flip = dx < 0.0;
                window_tick.move_(s.x as i32, s.y as i32);
            }

            // Сбрасываем скорость курсора (затухание)
            s.cursor_vx *= 0.5;
            s.cursor_vy *= 0.5;

            drawing_tick.queue_draw();
            glib::ControlFlow::Continue
        });
    });

    app.run();
}