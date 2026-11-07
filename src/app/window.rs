use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use adw::{prelude::*, Application};
use gtk::{glib, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION};

use crate::audio::engine::AudioEngine;
use crate::audio::sound_bank::SoundPreset;
use crate::model::{AppSettings, SharedParams};

const TIME_SIGNATURES: [(&str, u32); 4] = [("2/4", 2), ("3/4", 3), ("4/4", 4), ("6/8", 6)];

pub fn launch() -> anyhow::Result<()> {
    let app = Application::builder()
        .application_id("io.github.metrognome.Metrognome")
        .build();

    app.connect_activate(|app| {
        let window = build_window(app);
        window.present();
    });

    app.run();
    Ok(())
}

fn build_window(app: &Application) -> adw::ApplicationWindow {
    let engine_result = AudioEngine::new();
    let engine = Rc::new(RefCell::new(match engine_result {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("Audio engine unavailable: {error:#}");
            std::process::exit(1);
        }
    }));
    let params = engine.borrow().shared_params();
    let settings = AppSettings::load();
    settings.apply_to(&params);

    load_css();
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Metrognome")
        .default_width(430)
        .default_height(590)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 22);
    vbox.set_margin_top(24);
    vbox.set_margin_bottom(28);
    vbox.set_margin_start(24);
    vbox.set_margin_end(24);

    let header = build_header(&window, &engine);
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(430);
    clamp.set_tightening_threshold(460);
    clamp.set_child(Some(&vbox));

    toolbar.set_content(Some(&clamp));

    vbox.append(&build_pattern_row(&params));
    vbox.append(&build_tempo_card(&params));
    vbox.append(&build_beat_dots(&params));
    vbox.append(&build_controls(&engine, &params));

    let params_copy = params;
    window.connect_close_request(move |_| {
        let (beats, accents) = params_copy.pattern();
        let current = AppSettings {
            bpm: params_copy.bpm(),
            volume: params_copy.volume(),
            beats,
            accents,
        };
        if let Err(error) = current.save() {
            eprintln!("Could not save settings: {error}");
        }
        glib::Propagation::Proceed
    });

    window.set_content(Some(&toolbar));
    window
}

fn build_header(
    window: &adw::ApplicationWindow,
    engine: &Rc<RefCell<AudioEngine>>,
) -> adw::HeaderBar {
    let header = adw::HeaderBar::new();
    let title = gtk::Label::new(Some("Metrognome"));
    title.add_css_class("metrognome-title");
    header.set_title_widget(Some(&title));

    let sound_menu = gtk::MenuButton::new();
    sound_menu.set_icon_name("open-menu-symbolic");
    sound_menu.set_tooltip_text(Some("Sound settings"));

    let popover = gtk::Popover::new();
    let menu = gtk::Box::new(gtk::Orientation::Vertical, 8);
    menu.set_margin_top(12);
    menu.set_margin_bottom(12);
    menu.set_margin_start(12);
    menu.set_margin_end(12);

    for preset in SoundPreset::ALL {
        let button = gtk::Button::with_label(preset.title());
        let engine = engine.clone();
        let popover = popover.clone();
        button.connect_clicked(move |_| {
            let bank =
                crate::audio::sound_bank::SoundBank::preset(preset, engine.borrow().sample_rate);
            engine.borrow().set_sound_bank(bank);
            popover.popdown();
        });
        menu.append(&button);
    }

    menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let normal_button = gtk::Button::with_label("Set normal WAV…");
    let accent_button = gtk::Button::with_label("Set accent WAV…");
    let engine_for_normal = engine.clone();
    let window_for_normal = window.clone();
    let engine_for_accent = engine.clone();
    let window_for_accent = window.clone();
    normal_button.connect_clicked(move |_| {
        choose_wav(&window_for_normal, &engine_for_normal, false);
    });
    accent_button.connect_clicked(move |_| {
        choose_wav(&window_for_accent, &engine_for_accent, true);
    });
    menu.append(&normal_button);
    menu.append(&accent_button);

    popover.set_child(Some(&menu));
    sound_menu.set_popover(Some(&popover));
    header.pack_end(&sound_menu);
    header
}

fn choose_wav(window: &adw::ApplicationWindow, engine: &Rc<RefCell<AudioEngine>>, accent: bool) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title(if accent {
        "Choose accent WAV"
    } else {
        "Choose normal WAV"
    });
    let engine = engine.clone();

    dialog.open(
        Some(window),
        None::<&gtk::gio::Cancellable>,
        move |result| match result {
            Ok(file) => {
                let Some(path) = file.path() else {
                    return;
                };
                let result = if accent {
                    engine.borrow().set_accent_wav(&path)
                } else {
                    engine.borrow().set_normal_wav(&path)
                };
                if let Err(error) = result {
                    eprintln!("Could not load WAV: {error:#}");
                }
            }
            Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => {}
            Err(error) => eprintln!("Could not open file chooser: {error}"),
        },
    );
}

fn build_pattern_row(params: &Arc<SharedParams>) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.set_valign(gtk::Align::Center);
    row.set_halign(gtk::Align::Center);

    let mut first = None;
    for (label, beats) in TIME_SIGNATURES {
        let button = gtk::ToggleButton::with_label(label);
        button.add_css_class("pill-button");
        button.set_width_request(68);
        button.set_hexpand(false);

        if let Some(previous) = first.as_ref() {
            button.set_group(Some(previous));
        } else {
            first = Some(button.clone());
        }

        let params = params.clone();
        button.connect_toggled(move |toggle| {
            if toggle.is_active() {
                let default_accents = match beats {
                    6 => vec![true, false, false, true, false, false],
                    beats => {
                        let mut accents = vec![false; beats as usize];
                        accents[0] = true;
                        accents
                    }
                };
                params.set_pattern(beats, &default_accents);
            }
        });

        if beats == 4 {
            button.set_active(true);
        }

        row.append(&button);
    }

    row
}

fn build_tempo_card(params: &Arc<SharedParams>) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 14);
    card.add_css_class("metro-card");
    card.add_css_class("card");

    let top = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    top.set_halign(gtk::Align::Center);
    top.set_valign(gtk::Align::Center);

    let bpm = gtk::Label::new(Some(&params.bpm().to_string()));
    bpm.add_css_class("bpm-label");
    top.append(&bpm);

    let adjusters = gtk::Box::new(gtk::Orientation::Vertical, 8);
    adjusters.set_valign(gtk::Align::Center);
    let increase = gtk::Button::from_icon_name("list-add-symbolic");
    let decrease = gtk::Button::from_icon_name("list-remove-symbolic");
    increase.add_css_class("bpm-adjust");
    decrease.add_css_class("bpm-adjust");
    adjusters.append(&increase);
    adjusters.append(&decrease);
    top.append(&adjusters);

    let adjustment = gtk::Adjustment::new(f64::from(params.bpm()), 20.0, 301.0, 1.0, 5.0, 0.0);
    let slider = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&adjustment));
    slider.set_digits(0);
    slider.set_hexpand(true);
    slider.add_mark(20.0, gtk::PositionType::Bottom, Some("20"));
    slider.add_mark(80.0, gtk::PositionType::Bottom, Some("80"));
    slider.add_mark(140.0, gtk::PositionType::Bottom, Some("140"));
    slider.add_mark(200.0, gtk::PositionType::Bottom, Some("200"));
    slider.add_mark(260.0, gtk::PositionType::Bottom, Some("260"));

    let bpm_for_plus = bpm.clone();
    let adjustment_for_plus = adjustment.clone();
    let params_for_plus = params.clone();
    increase.connect_clicked(move |_| {
        let next = params_for_plus.bpm().saturating_add(1).min(300);
        params_for_plus.set_bpm(next);
        bpm_for_plus.set_text(&next.to_string());
        adjustment_for_plus.set_value(f64::from(next));
    });

    let bpm_for_minus = bpm.clone();
    let adjustment_for_minus = adjustment.clone();
    let params_for_minus = params.clone();
    decrease.connect_clicked(move |_| {
        let next = params_for_minus.bpm().saturating_sub(1).max(20);
        params_for_minus.set_bpm(next);
        bpm_for_minus.set_text(&next.to_string());
        adjustment_for_minus.set_value(f64::from(next));
    });

    let bpm_for_scale = bpm.clone();
    let params_for_scale = params.clone();
    slider.connect_value_changed(move |scale| {
        let value = scale.value().round() as u32;
        params_for_scale.set_bpm(value);
        bpm_for_scale.set_text(&value.to_string());
    });

    let bpm_for_poll = bpm.clone();
    let adjustment_for_poll = adjustment.clone();
    let params_for_poll = params.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        let shown = bpm_for_poll.text().parse::<u32>().unwrap_or_default();
        let actual = params_for_poll.bpm();

        if shown != actual {
            bpm_for_poll.set_text(&actual.to_string());
            adjustment_for_poll.set_value(f64::from(actual));
        }

        glib::ControlFlow::Continue
    });

    let volume_adjustment =
        gtk::Adjustment::new(f64::from(params.volume()), 0.0, 1.0, 0.01, 0.05, 0.0);
    let volume = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&volume_adjustment));
    volume.set_digits(2);
    volume.set_hexpand(true);
    let params_for_volume = params.clone();
    volume.connect_value_changed(move |scale| {
        params_for_volume.set_volume(scale.value() as f32);
    });

    let volume_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let low_volume = gtk::Image::from_icon_name("audio-volume-low-symbolic");
    volume_row.append(&low_volume);
    volume_row.append(&volume);

    card.append(&top);
    card.append(&slider);
    card.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    card.append(&volume_row);
    card
}

fn build_beat_dots(params: &Arc<SharedParams>) -> gtk::Box {
    let dots = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    dots.set_halign(gtk::Align::Center);
    let buttons = Rc::new(RefCell::new(Vec::<gtk::ToggleButton>::new()));
    let rebuilding = Rc::new(Cell::new(false));

    let buttons_for_poll = buttons.clone();
    let params_for_poll = params.clone();
    glib::timeout_add_local(Duration::from_millis(16), move || {
        let current = params_for_poll.current_beat() as usize;
        let running = params_for_poll
            .running
            .load(std::sync::atomic::Ordering::Relaxed);
        for (index, button) in buttons_for_poll.borrow().iter().enumerate() {
            if running && index == current {
                button.add_css_class("beat-active");
            } else {
                button.remove_css_class("beat-active");
            }
        }
        glib::ControlFlow::Continue
    });

    let params_for_redraw = params.clone();
    let buttons_for_redraw = buttons.clone();
    let rebuilding_for_redraw = rebuilding.clone();
    let dots_for_redraw = dots.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        let desired = params_for_redraw.beat_count() as usize;
        if buttons_for_redraw.borrow().len() != desired {
            replace_beat_buttons(
                &dots_for_redraw,
                &buttons_for_redraw,
                &rebuilding_for_redraw,
                &params_for_redraw,
            );
        }
        glib::ControlFlow::Continue
    });

    replace_beat_buttons(&dots, &buttons, &rebuilding, params);

    dots
}

fn replace_beat_buttons(
    row: &gtk::Box,
    buttons: &Rc<RefCell<Vec<gtk::ToggleButton>>>,
    rebuilding: &Rc<Cell<bool>>,
    params: &Arc<SharedParams>,
) {
    for child in row.observe_children().into_iter().flatten() {
        if let Some(button) = child.downcast_ref::<gtk::ToggleButton>() {
            row.remove(button);
        }
    }
    buttons.borrow_mut().clear();

    let (_, accents) = params.pattern();
    rebuilding.set(true);
    let mut created = Vec::with_capacity(accents.len());

    for accented in accents {
        let button = gtk::ToggleButton::with_label("●");
        button.add_css_class("beat-toggle");
        button.set_active(accented);

        let buttons = buttons.clone();
        let rebuilding = rebuilding.clone();
        let params = params.clone();
        button.connect_toggled(move |_| {
            if rebuilding.get() {
                return;
            }

            let accents: Vec<bool> = buttons
                .borrow()
                .iter()
                .map(|button| button.is_active())
                .collect();
            params.set_pattern(params.beat_count(), &accents);
        });

        row.append(&button);
        created.push(button);
    }

    *buttons.borrow_mut() = created;
    rebuilding.set(false);
}

fn build_controls(engine: &Rc<RefCell<AudioEngine>>, params: &Arc<SharedParams>) -> gtk::CenterBox {
    let row = gtk::CenterBox::new();
    row.set_hexpand(true);
    row.set_valign(gtk::Align::Center);

    let tap = gtk::Button::with_label("Tap");
    tap.add_css_class("tap-button");
    let play = gtk::Button::from_icon_name("media-playback-start");
    play.add_css_class("play-button");
    play.add_css_class("suggested-action");

    let tap_wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    tap_wrapper.set_halign(gtk::Align::End);
    tap_wrapper.set_margin_end(64);
    tap_wrapper.append(&tap);

    let tap_times = Rc::new(RefCell::new(Vec::<std::time::Instant>::new()));
    let params_for_tap = params.clone();
    tap.connect_clicked(move |_| {
        let now = std::time::Instant::now();
        let mut times = tap_times.borrow_mut();
        times.retain(|time| now.duration_since(*time).as_secs_f64() < 3.0);
        times.push(now);

        if times.len() >= 2 {
            let deltas: Vec<f64> = times
                .windows(2)
                .map(|pair| (pair[1] - pair[0]).as_secs_f64())
                .collect();
            let average = deltas.iter().sum::<f64>() / deltas.len() as f64;
            if average > 0.0 {
                let bpm = (60.0 / average).round() as u32;
                params_for_tap.set_bpm(bpm.clamp(20, 300));
            }
        }
    });

    let engine_for_play = engine.clone();
    let params_for_play = params.clone();
    play.connect_clicked(move |button| {
        let mut engine = engine_for_play.borrow_mut();
        if engine.is_running() {
            engine.stop();
            button.set_icon_name("media-playback-start");
        } else if let Err(error) = engine.play() {
            eprintln!("Could not start audio: {error:#}");
        } else {
            button.set_icon_name("media-playback-stop");
        }
        params_for_play
            .current_beat
            .store(0, std::sync::atomic::Ordering::Relaxed);
    });

    row.set_start_widget(Some(&tap_wrapper));
    row.set_center_widget(Some(&play));
    row
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string(crate::app::style::CSS);

    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
