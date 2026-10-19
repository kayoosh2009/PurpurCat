mod anim;
mod physics;
mod state;

use glib::timeout_add_local;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, DrawingArea};
use gtk4_layer_shell::{Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use state::{BehaviorMode, CatState, State, CAT_SIZE, TICK_MS};

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
        let display = gdk4::Display::default().unwrap();
        let monitor = display.monitors().item(0).expect("Нет мониторов");
        let monitor = monitor.downcast::<gdk4::Monitor>().expect("Не монитор");
        let geometry = monitor.geometry();
        let screen_width = geometry.width() as f64;
        let screen_height = geometry.height() as f64;

        let start_x = screen_width - CAT_SIZE as f64 - 50.0;
        let start_y = screen_height - CAT_SIZE as f64 - 50.0;

        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(CAT_SIZE)
            .default_height(CAT_SIZE)
            .decorated(false)
            .build();

        let css = gtk4::CssProvider::new();
        css.load_from_data("window { background: transparent; }");
        gtk4::style_context_add_provider_for_display(
            &gdk4::Display::default().unwrap(),
            &css,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(gtk4_layer_shell::Edge::Left, true);
        window.set_anchor(gtk4_layer_shell::Edge::Top, true);
        window.set_margin(gtk4_layer_shell::Edge::Left, start_x as i32);
        window.set_margin(gtk4_layer_shell::Edge::Top, start_y as i32);
        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);

        let idle_frames = Rc::new(load_pixbufs(anim::idle::SPRITES));
        let run_frames = Rc::new(load_pixbufs(anim::run::SPRITES));
        let sleep_frames = Rc::new(load_pixbufs(anim::sleep::SPRITES_LOOP));

        let state = Rc::new(RefCell::new(State::new(start_x, start_y)));

        let drawing = DrawingArea::new();
        drawing.set_size_request(CAT_SIZE, CAT_SIZE);

        let idle_draw = idle_frames.clone();
        let run_draw = run_frames.clone();
        let sleep_draw = sleep_frames.clone();
        let state_draw = state.clone();

        drawing.set_draw_func(move |_, cr, w, _h| {
            let s = state_draw.borrow();
            let frames = match s.cat_state {
                CatState::Idle => &idle_draw,
                CatState::Walking => &run_draw,
                CatState::Running => &run_draw,
                CatState::Sleeping => &sleep_draw,
            };
            let tex = &frames[s.frame % frames.len()];

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

        let cursor_x_ref = Rc::new(RefCell::new(0.0f64));
        let cursor_y_ref = Rc::new(RefCell::new(0.0f64));

        let motion = gtk4::EventControllerMotion::new();
        let cx_motion = cursor_x_ref.clone();
        let cy_motion = cursor_y_ref.clone();
        let state_motion = state.clone();
        motion.connect_motion(move |_, mx, my| {
            *cx_motion.borrow_mut() = mx;
            *cy_motion.borrow_mut() = my;

            let mut s = state_motion.borrow_mut();
            if s.behavior != BehaviorMode::Dragging && s.behavior != BehaviorMode::Petting {
                s.behavior = BehaviorMode::Petting;
                s.petting_tick = 0;
                s.petting_steps = 0;
                s.flip = !s.flip;
            }
        });
        drawing.add_controller(motion);

        let click = gtk4::GestureClick::new();
        click.set_button(gdk4::BUTTON_PRIMARY);

        let state_press = state.clone();
        click.connect_pressed(move |_, n_press, _, _| {
            let mut s = state_press.borrow_mut();
            if n_press == 1 {
                s.press_start_time = Some(Instant::now());
                s.drag_win_x = s.x;
                s.drag_win_y = s.y;
                s.click_count += 1;
                if s.click_count >= 5 {
                    s.click_count = 0;
                    s.behavior = BehaviorMode::Petting;
                    s.petting_tick = 0;
                    s.petting_steps = 0;
                }
            }
        });

        let state_released = state.clone();
        click.connect_released(move |_, n_press, _, _| {
            let mut s = state_released.borrow_mut();
            s.press_start_time = None;

            if s.dragging {
                s.dragging = false;
                s.behavior = BehaviorMode::Roaming;
            } else if n_press == 1 {
                s.behavior = if s.behavior == BehaviorMode::Hunting {
                    BehaviorMode::Roaming
                } else {
                    BehaviorMode::Hunting
                };
                s.random_run_until = None;
            }
        });

        drawing.add_controller(click);
        window.present();

        let state_tick = state.clone();
        let drawing_tick = drawing.clone();
        let window_tick = window.clone();
        let cursor_x_tick = cursor_x_ref.clone();
        let cursor_y_tick = cursor_y_ref.clone();

        timeout_add_local(Duration::from_millis(TICK_MS), move || {
            let mut s = state_tick.borrow_mut();
            let now = Instant::now();
            let cx = *cursor_x_tick.borrow();
            let cy = *cursor_y_tick.borrow();

            physics::tick(&mut s, now, screen_width, screen_height, cx, cy);

            window_tick.set_margin(gtk4_layer_shell::Edge::Left, s.x as i32);
            window_tick.set_margin(gtk4_layer_shell::Edge::Top, s.y as i32);
            drawing_tick.queue_draw();

            glib::ControlFlow::Continue
        });
    });

    app.run();
}
