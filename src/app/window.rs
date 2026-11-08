use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use adw::{prelude::*, Application};
use gtk::{gdk, gio, glib, CssProvider, IconTheme, STYLE_PROVIDER_PRIORITY_APPLICATION};

use crate::audio::engine::AudioEngine;
use crate::audio::sound_bank::SoundBank;
use crate::model::{default_states, AppSettings, BeatState, Language, SharedParams, LANGUAGES};

const TIME_SIGNATURES: [(&str, u32); 4] = [("2/4", 2), ("3/4", 3), ("4/4", 4), ("6/8", 6)];

/// Visible settings-modal copy. Keeping the native language in the selector
/// while translating the surrounding controls makes a language switch obvious
/// even when the user cannot read the current interface language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsText {
    Settings,
    Language,
    Sample,
    ClassicBundled,
    PlayMeasure,
    HighlightFirstBeat,
    AboutMetrognome,
}

/// One row per `Language`, one column per `SettingsText`.
const SETTINGS_TEXTS: [[&str; 7]; 25] = [
    [
        "Settings",
        "Language",
        "Sample",
        "Classic (bundled)",
        "Play measure",
        "Highlight first beat",
        "About Metrognome",
    ],
    [
        "设置",
        "语言",
        "样本",
        "经典（内置）",
        "播放一小节",
        "突出第一拍",
        "关于 Metrognome",
    ],
    [
        "सेटिंग्स",
        "भाषा",
        "सैंपल",
        "क्लासिक (बिल्ट-इन)",
        "एक माप बजाएँ",
        "पहला बीट हाइलाइट करें",
        "Metrognome के बारे में",
    ],
    [
        "Ajustes",
        "Idioma",
        "Muestra",
        "Clásica (incluida)",
        "Reproducir compás",
        "Resaltar primer tiempo",
        "Acerca de Metrognome",
    ],
    [
        "Paramètres",
        "Langue",
        "Échantillon",
        "Classique (intégré)",
        "Jouer une mesure",
        "Mettre en valeur le premier temps",
        "À propos de Metrognome",
    ],
    [
        "الإعدادات",
        "اللغة",
        "العينة",
        "كلاسيكي (مدمج)",
        "تشغيل ميزان",
        "تمييز النبضة الأولى",
        "حول Metrognome",
    ],
    [
        "সেটিংস",
        "ভাষা",
        "স্যাম্পল",
        "ক্লাসিক (অন্তর্ভুক্ত)",
        "মাপ বাজান",
        "প্রথম বিট হাইলাইট করুন",
        "Metrognome সম্পর্কে",
    ],
    [
        "Configurações",
        "Idioma",
        "Amostra",
        "Clássico (integrado)",
        "Tocar compasso",
        "Destacar primeiro tempo",
        "Sobre o Metrognome",
    ],
    [
        "Настройки",
        "Язык",
        "Семпл",
        "Classic (встроенный)",
        "Воспроизвести такт",
        "Выделять первый удар",
        "О программе",
    ],
    [
        "ترتیبات",
        "زبان",
        "سیمپل",
        "کلاسک (شامل)",
        "میزان چلائیں",
        "پہلی دھن نمایاں کریں",
        "Metrognome کے بارے میں",
    ],
    [
        "Einstellungen",
        "Sprache",
        "Sample",
        "Klassisch (integriert)",
        "Takt abspielen",
        "Ersten Schlag hervorheben",
        "Über Metrognome",
    ],
    [
        "設定",
        "言語",
        "サンプル",
        "クラシック（内蔵）",
        "1小節を再生",
        "最初のビートを強調",
        "Metrognome について",
    ],
    [
        "ਸੈਟਿੰਗਾਂ",
        "ਭਾਸ਼ਾ",
        "ਸੈਂਪਲ",
        "ਕਲਾਸਿਕ (ਸ਼ਾਮਲ)",
        "ਤਾਲ ਚਲਾਓ",
        "ਪਹਿਲਾ ਬੀਟ ਉਭਾਰੋ",
        "Metrognome ਬਾਰੇ",
    ],
    [
        "सेटिंग्ज",
        "भाषा",
        "नमुना",
        "क्लासिक (अंतर्भूत)",
        "ताल वाजवा",
        "पहिला ठोका ठळक करा",
        "Metrognome विषयी",
    ],
    [
        "సెట్టింగ్‌లు",
        "భాష",
        "సాంపిల్",
        "క్లాసిక్ (అంతర్భుతం)",
        "తాళం వాయించు",
        "మొదటి బీట్ హైలైట్ చేయండి",
        "Metrognome గురించి",
    ],
    [
        "Ayarlar",
        "Dil",
        "Örnek",
        "Klasik (yerleşik)",
        "Ölçüyü çal",
        "İlk vurguyu öne çıkar",
        "Metrognome Hakkında",
    ],
    [
        "설정",
        "언어",
        "샘플",
        "클래식(내장)",
        "마디 재생",
        "첫 박자 강조",
        "Metrognome 정보",
    ],
    [
        "Cài đặt",
        "Ngôn ngữ",
        "Mẫu âm thanh",
        "Cổ điển (nhúng)",
        "Phát một ô nhịp",
        "Làm nổi bật nhịp đầu",
        "Giới thiệu Metrognome",
    ],
    [
        "Impostazioni",
        "Lingua",
        "Campione",
        "Classico (integrato)",
        "Riproduci misura",
        "Evidenzia primo battito",
        "Informazioni su Metrognome",
    ],
    [
        "Pengaturan",
        "Bahasa",
        "Sampel",
        "Classic (bawaan)",
        "Putar birama",
        "Sorot ketukan pertama",
        "Tentang Metrognome",
    ],
    [
        "Ustawienia",
        "Język",
        "Próbka",
        "Classic (wbudowany)",
        "Zagraj takt",
        "Wyróżnij pierwsze uderzenie",
        "O Metrognome",
    ],
    [
        "Налаштування",
        "Мова",
        "Семпл",
        "Classic (вбудований)",
        "Відтворити такт",
        "Виділяти перший удар",
        "Про програму",
    ],
    [
        "Instellingen",
        "Taal",
        "Voorbeeld",
        "Classic (ingebed)",
        "Maat afspelen",
        "Eerste slag markeren",
        "Over Metrognome",
    ],
    [
        "การตั้งค่า",
        "ภาษา",
        "ตัวอย่างเสียง",
        "คลาสสิก (ในตัว)",
        "เล่นหนึ่งห้อง",
        "เน้นจังหวะแรก",
        "เกี่ยวกับ Metrognome",
    ],
    [
        "Mipangilio",
        "Lugha",
        "Sampuli",
        "Classic (ndani)",
        "Cheza kipimo",
        "Angazia mdundo wa kwanza",
        "Kuhusu Metrognome",
    ],
];

fn settings_text(language: Language, text: SettingsText) -> &'static str {
    SETTINGS_TEXTS[language.index()][text as usize]
}

fn about_text(language: Language) -> &'static str {
    settings_text(language, SettingsText::AboutMetrognome)
}

fn show_about(parent: &adw::Window) {
    adw::AboutWindow::builder()
        .transient_for(parent)
        .application_name("Metrognome")
        .application_icon("metrognome")
        .version(env!("CARGO_PKG_VERSION"))
        .comments("A stable and customizable GNOME metronome built with Rust, GTK 4, libadwaita, and cpal.")
        .developer_name("ez404-36")
        .license_type(gtk::License::MitX11)
        .build()
        .present();
}

/// Errors are less prominent than the settings controls; Russian keeps its
/// existing copy and all other selections fall back to English. This avoids
/// pretending that every runtime failure already has a reviewed translation.
#[derive(Debug, Clone, Copy)]
enum ErrorText {
    SaveSettings,
    LoadWav,
    OpenFileChooser,
    AudioStream,
    PlayTapSound,
    StartAudio,
    LoadSample,
    PlaySample,
    RecoverAudio,
}

fn error_text(language: Language, text: ErrorText) -> &'static str {
    if language == Language::Russian {
        return match text {
            ErrorText::SaveSettings => "Не удалось сохранить настройки",
            ErrorText::LoadWav => "Не удалось загрузить WAV",
            ErrorText::OpenFileChooser => "Не удалось открыть выбор файла",
            ErrorText::AudioStream => "Ошибка аудиопотока",
            ErrorText::PlayTapSound => "Не удалось воспроизвести звук тапа",
            ErrorText::StartAudio => "Не удалось запустить аудио",
            ErrorText::LoadSample => "Не удалось загрузить семпл",
            ErrorText::PlaySample => "Не удалось воспроизвести семпл",
            ErrorText::RecoverAudio => "Аудиовыход восстановлен",
        };
    }

    match text {
        ErrorText::SaveSettings => "Could not save settings",
        ErrorText::LoadWav => "Could not load WAV",
        ErrorText::OpenFileChooser => "Could not open the file chooser",
        ErrorText::AudioStream => "Audio stream error",
        ErrorText::PlayTapSound => "Could not play the tap sound",
        ErrorText::StartAudio => "Could not start audio",
        ErrorText::LoadSample => "Could not load sample",
        ErrorText::PlaySample => "Could not play sample",
        ErrorText::RecoverAudio => "Audio output recovered",
    }
}

/// Bottom-right transient error popup. A single revealer is reused: every
/// show bumps a generation counter so a superseded hide timeout cannot hide
/// a newer message early.
struct ErrorToast {
    revealer: gtk::Revealer,
    label: gtk::Label,
    generation: Cell<u64>,
}

impl ErrorToast {
    fn new(overlay: &gtk::Overlay) -> Rc<Self> {
        let label = gtk::Label::new(None);
        label.set_wrap(true);
        label.set_xalign(0.0);
        label.set_max_width_chars(48);
        label.add_css_class("error-toast-label");

        let toast_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        toast_box.append(&gtk::Image::from_icon_name("dialog-error-symbolic"));
        toast_box.append(&label);
        toast_box.add_css_class("error-toast");

        let revealer = gtk::Revealer::builder()
            .child(&toast_box)
            .transition_type(gtk::RevealerTransitionType::SlideUp)
            .transition_duration(250)
            .reveal_child(false)
            .valign(gtk::Align::End)
            .halign(gtk::Align::End)
            .margin_bottom(24)
            .margin_end(24)
            .build();
        overlay.add_overlay(&revealer);

        Rc::new(Self {
            revealer,
            label,
            generation: Cell::new(0),
        })
    }

    fn show(self: &Rc<Self>, message: &str) {
        let next = self.generation.get().wrapping_add(1);
        self.generation.set(next);
        self.label.set_text(message);
        self.revealer.set_reveal_child(true);
        let toast = self.clone();
        glib::timeout_add_local_once(Duration::from_secs(5), move || {
            if toast.generation.get() == next {
                toast.revealer.set_reveal_child(false);
            }
        });
    }
}

pub fn launch() -> anyhow::Result<()> {
    register_icon_resources();
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

    // Restore the persisted sample choice before any widget is built so the
    // audio engine, the selector and the previous session agree immediately.
    // Startup failures have no toast yet, so they stay on stderr; later
    // user-triggered failures surface as popups.
    if let Err(error) = load_sample(&engine, params.sample()) {
        eprintln!("Saved sample failed to load: {error:#}");
        params.set_sample(0);
        if let Err(error) = load_sample(&engine, 0) {
            eprintln!("Bundled sample failed to load: {error:#}");
        }
    }

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

    let header = build_header();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(430);
    clamp.set_tightening_threshold(460);
    clamp.set_child(Some(&vbox));

    toolbar.set_content(Some(&clamp));

    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&toolbar));
    let toast = ErrorToast::new(&overlay);

    vbox.append(&build_pattern_row(&params));
    vbox.append(&build_tempo_card(&params));
    vbox.append(&build_beat_dots(&params));
    vbox.append(&build_controls(&window, &engine, &params, &toast));

    // The autosave loop persists every UI-relevant change (tempo, volume,
    // meter, beat toggles, highlight switch and sample choice) shortly after
    // it happens, so even a crash or forced kill cannot lose the last state.
    let last_saved = Rc::new(RefCell::new(Some(AppSettings::from_params(&params))));
    let params_for_save = params.clone();
    let toast_for_save = toast.clone();
    let last_saved_for_loop = last_saved.clone();
    glib::timeout_add_local(Duration::from_millis(250), move || {
        let current = AppSettings::from_params(&params_for_save);
        if last_saved_for_loop.borrow().as_ref() != Some(&current) {
            if let Err(error) = current.save() {
                eprintln!("Could not save settings: {error}");
                toast_for_save.show(&format!(
                    "{}: {error}",
                    error_text(params_for_save.language(), ErrorText::SaveSettings)
                ));
            }
            *last_saved_for_loop.borrow_mut() = Some(current);
        }
        glib::ControlFlow::Continue
    });

    // Rebuild the output stream after a device error (unplugged headphones,
    // PipeWire restart, ...). Repeated failures are reported once per
    // episode so the watchdog cannot spam popups every 500 ms.
    let engine_for_watch = engine.clone();
    let toast_for_watch = toast.clone();
    let params_for_watch = params.clone();
    let watch_failing = Rc::new(Cell::new(false));
    glib::timeout_add_local(Duration::from_millis(500), move || {
        let result = engine_for_watch.borrow_mut().recover_stream_if_needed();
        let was_failing = watch_failing.get();
        match result {
            Ok(false) => {}
            Ok(true) => {
                watch_failing.set(false);
                toast_for_watch.show(error_text(
                    params_for_watch.language(),
                    ErrorText::RecoverAudio,
                ));
            }
            Err(error) => {
                if !was_failing {
                    toast_for_watch.show(&format!(
                        "{}: {error:#}",
                        error_text(params_for_watch.language(), ErrorText::AudioStream)
                    ));
                }
                watch_failing.set(true);
            }
        }
        glib::ControlFlow::Continue
    });

    let params_copy = params;
    window.connect_close_request(move |_| {
        let current = AppSettings::from_params(&params_copy);
        if let Err(error) = current.save() {
            eprintln!("Could not save settings: {error}");
        }
        glib::Propagation::Proceed
    });

    window.set_content(Some(&overlay));
    apply_pointer_cursors(window.upcast_ref());
    window
}

fn build_header() -> adw::HeaderBar {
    let header = adw::HeaderBar::new();
    let title = gtk::Box::new(gtk::Orientation::Horizontal, 7);

    let logo = gtk::Image::builder()
        .icon_name("metrognome-header")
        .pixel_size(18)
        .build();
    logo.add_css_class("app-logo");
    title.append(&logo);

    let label = gtk::Label::new(Some("Metrognome"));
    label.add_css_class("metrognome-title");
    title.append(&label);
    title.set_halign(gtk::Align::Center);

    header.set_title_widget(Some(&title));
    header
}

fn choose_wav(
    window: &adw::ApplicationWindow,
    engine: &Rc<RefCell<AudioEngine>>,
    accent: bool,
    params: &Arc<SharedParams>,
    toast: &Rc<ErrorToast>,
) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title(if accent {
        "Choose accent WAV"
    } else {
        "Choose normal WAV"
    });
    let engine = engine.clone();
    let toast = toast.clone();
    let params = params.clone();

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
                    toast.show(&format!(
                        "{}: {error:#}",
                        error_text(params.language(), ErrorText::LoadWav)
                    ));
                }
            }
            Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => {}
            Err(error) => {
                eprintln!("Could not open file chooser: {error}");
                toast.show(&format!(
                    "{}: {error}",
                    error_text(params.language(), ErrorText::OpenFileChooser)
                ));
            }
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

        // Restore the active pill to the persisted meter instead of always
        // defaulting to 4/4 after restart. Do this before installing the
        // handler so restoring state cannot replace the saved beat pattern.
        if beats == params.beat_count() {
            button.set_active(true);
        }

        let params = params.clone();
        button.connect_toggled(move |toggle| {
            if toggle.is_active() {
                params.set_states(beats, &default_states(beats));
            }
        });

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

    let adjustment = gtk::Adjustment::new(f64::from(params.bpm()), 20.0, 300.0, 1.0, 5.0, 0.0);
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
        params_for_scale.set_bpm(scale.value().round() as u32);
        // Show the clamped value so dragging to the slider's edge cannot
        // briefly display a BPM the scheduler will never use.
        bpm_for_scale.set_text(&params_for_scale.bpm().to_string());
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
    let buttons = Rc::new(RefCell::new(Vec::<gtk::Button>::new()));
    let signature = Rc::new(Cell::new(0_u32));

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
    let signature_for_redraw = signature.clone();
    let dots_for_redraw = dots.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        let desired = params_for_redraw.beat_count();
        if buttons_for_redraw.borrow().len() != desired as usize
            || signature_for_redraw.get() != desired
        {
            replace_beat_buttons(&dots_for_redraw, &buttons_for_redraw, &params_for_redraw);
            signature_for_redraw.set(desired);
        }
        glib::ControlFlow::Continue
    });

    replace_beat_buttons(&dots, &buttons, params);
    signature.set(params.beat_count());

    dots
}

fn replace_beat_buttons(
    row: &gtk::Box,
    buttons: &Rc<RefCell<Vec<gtk::Button>>>,
    params: &Arc<SharedParams>,
) {
    // GTK's child list is live: removing a widget while iterating it shifts
    // the remaining indices and can leave stale beat buttons in the row.
    // Snapshot the children first so every old button is removed reliably.
    let old_children: Vec<gtk::glib::Object> = row
        .observe_children()
        .into_iter()
        .filter_map(Result::ok)
        .collect();
    for child in old_children {
        if let Some(button) = child.downcast_ref::<gtk::Button>() {
            row.remove(button);
        }
    }
    buttons.borrow_mut().clear();

    let states = params.states();
    let mut created = Vec::with_capacity(states.len());

    for (index, state) in states.iter().copied().enumerate() {
        let button = gtk::Button::with_label("\u{25CF}");
        button.add_css_class("beat-toggle");
        button.set_valign(gtk::Align::Center);
        button.set_cursor_from_name(Some("pointer"));
        if index == 0 {
            // The downbeat remains slightly larger even when the special
            // musical highlight is disabled.
            button.add_css_class("first-beat");
        }
        apply_state_class(&button, state);

        let buttons = buttons.clone();
        let params = params.clone();
        let index_for_click = index;
        button.connect_clicked(move |_| {
            let mut states = params.states();
            let current = states
                .get(index_for_click)
                .copied()
                .unwrap_or(BeatState::Enabled);
            // A click toggles a dot between its two exposed states: enabled
            // and disabled.
            let next = match current {
                BeatState::Enabled | BeatState::Normal => BeatState::Muted,
                BeatState::Muted => BeatState::Enabled,
            };
            if let Some(slot) = states.get_mut(index_for_click) {
                *slot = next;
            }
            params.set_states(params.beat_count(), &states);

            let states = params.states();
            for (position, button) in buttons.borrow().iter().enumerate() {
                let state = states.get(position).copied().unwrap_or(BeatState::Enabled);
                apply_state_class(button, state);
            }
        });

        row.append(&button);
        created.push(button);
    }

    *buttons.borrow_mut() = created;
}

fn apply_state_class(button: &gtk::Button, state: BeatState) {
    for class in ["state-accent", "state-muted"] {
        button.remove_css_class(class);
    }

    button.add_css_class(match state {
        BeatState::Enabled => "state-accent",
        BeatState::Normal => "state-accent",
        BeatState::Muted => "state-muted",
    });
}

fn build_controls(
    window: &adw::ApplicationWindow,
    engine: &Rc<RefCell<AudioEngine>>,
    params: &Arc<SharedParams>,
    toast: &Rc<ErrorToast>,
) -> gtk::CenterBox {
    let row = gtk::CenterBox::new();
    row.set_hexpand(true);
    row.set_valign(gtk::Align::Center);

    let tap = gtk::Button::with_label("Tap");
    tap.add_css_class("tap-button");
    tap.add_css_class("circular");
    tap.set_width_request(52);
    tap.set_height_request(52);
    tap.set_valign(gtk::Align::Center);
    // Mirror the tap button and the burger around the centered play button:
    // identical sizes and equal margins on both sides keep the side buttons
    // at exactly the same distance from it.
    tap.set_margin_start(24);
    tap.set_margin_end(24);
    let play = gtk::Button::from_icon_name("media-playback-start");
    play.add_css_class("play-button");
    play.add_css_class("suggested-action");

    let settings_button = gtk::Button::from_icon_name("open-menu-symbolic");
    settings_button.add_css_class("flat");
    settings_button.add_css_class("controls-menu-button");
    settings_button.set_width_request(52);
    settings_button.set_height_request(52);
    settings_button.set_tooltip_text(Some(settings_text(
        params.language(),
        SettingsText::Settings,
    )));
    settings_button.set_valign(gtk::Align::Center);
    settings_button.set_margin_start(24);
    settings_button.set_margin_end(24);
    let params_for_open = params.clone();
    let (settings_dialog, settings_sample_select, settings_language_select) =
        build_settings_modal(window, engine, params);
    let settings_button_for_language = settings_button.clone();
    let params_for_language_button = params.clone();
    settings_language_select.connect_selected_notify(move |select| {
        let language = Language::from_index(select.selected() as usize);
        params_for_language_button.set_language(language);
        settings_button_for_language
            .set_tooltip_text(Some(settings_text(language, SettingsText::Settings)));
    });
    settings_button.connect_clicked(move |_| {
        // The bank for the persisted sample is already loaded; only resync
        // the dropdown so a failed selection cannot leave the visual choice
        // pointing at a sample that is not actually loaded.
        settings_sample_select.set_selected(params_for_open.sample() as u32);
        settings_dialog.present();
    });

    let tap_times = Rc::new(RefCell::new(Vec::<std::time::Instant>::new()));
    let params_for_tap = params.clone();
    let engine_for_tap = engine.clone();
    let toast_for_tap = toast.clone();
    tap.connect_clicked(move |_| {
        // Give the user audible feedback from the sample they currently have
        // selected, while the tap itself only recalculates the tempo.
        if let Err(error) = engine_for_tap.borrow_mut().play_normal_one_shot() {
            eprintln!("Could not play tap sound: {error:#}");
            toast_for_tap.show(&format!(
                "{}: {error:#}",
                error_text(params_for_tap.language(), ErrorText::PlayTapSound)
            ));
        }

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
    let toast_for_play = toast.clone();
    play.connect_clicked(move |button| {
        let mut engine = engine_for_play.borrow_mut();
        if engine.is_running() {
            engine.stop();
            button.set_icon_name("media-playback-start");
        } else if let Err(error) = engine.play() {
            eprintln!("Could not start audio: {error:#}");
            toast_for_play.show(&format!(
                "{}: {error:#}",
                error_text(params_for_play.language(), ErrorText::StartAudio)
            ));
        } else {
            button.set_icon_name("media-playback-stop");
        }
        params_for_play
            .current_beat
            .store(0, std::sync::atomic::Ordering::Relaxed);
    });

    row.set_start_widget(Some(&tap));
    row.set_center_widget(Some(&play));
    row.set_end_widget(Some(&settings_button));
    row
}

fn build_settings_modal(
    parent: &adw::ApplicationWindow,
    engine: &Rc<RefCell<AudioEngine>>,
    params: &Arc<SharedParams>,
) -> (adw::Window, gtk::DropDown, gtk::DropDown) {
    let dialog = adw::Window::new();
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.set_destroy_with_parent(true);
    dialog.set_title(Some(settings_text(
        params.language(),
        SettingsText::Settings,
    )));
    dialog.set_default_width(380);

    // GTK's default close handling destroys the window. Since the settings
    // dialog is reused by the burger button, hide it instead and recreate the
    // native surface the next time it is presented.
    let dialog_for_close = dialog.clone();
    dialog.connect_close_request(move |_| {
        dialog_for_close.set_visible(false);
        glib::Propagation::Stop
    });

    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let header_title = gtk::Label::new(Some(settings_text(
        params.language(),
        SettingsText::Settings,
    )));
    header.set_title_widget(Some(&header_title));
    toolbar.add_top_bar(&header);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    content.set_margin_top(18);
    content.set_margin_bottom(24);
    content.set_margin_start(18);
    content.set_margin_end(18);

    let language_title = gtk::Label::new(Some(settings_text(
        params.language(),
        SettingsText::Language,
    )));
    language_title.set_halign(gtk::Align::Start);
    language_title.add_css_class("settings-section-title");
    content.append(&language_title);

    let language_labels: Vec<String> = LANGUAGES
        .iter()
        .map(|(language, _, _)| language.selector_label())
        .collect();
    let language_select = gtk::DropDown::from_strings(
        language_labels
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice(),
    );
    language_select.set_hexpand(true);
    language_select.set_valign(gtk::Align::Center);
    // Restore before connecting the handler, just like every other selector.
    language_select.set_selected(params.language().index() as u32);

    let language_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    language_row.set_hexpand(true);
    language_row.append(&language_select);
    content.append(&language_row);

    let sample_title =
        gtk::Label::new(Some(settings_text(params.language(), SettingsText::Sample)));
    sample_title.set_halign(gtk::Align::Start);
    sample_title.add_css_class("settings-section-title");
    content.append(&sample_title);

    let mut sample_labels = vec!["Classic (bundled)"];
    sample_labels.extend(SoundBank::candidate_labels());
    let preview_busy = Rc::new(Cell::new(false));
    let sample_select = gtk::DropDown::from_strings(sample_labels.as_slice());
    sample_select.set_tooltip_text(Some(settings_text(params.language(), SettingsText::Sample)));
    sample_select.set_hexpand(true);
    sample_select.set_valign(gtk::Align::Center);
    // Select the restored sample before installing the change handler so the
    // initial selection cannot trigger a redundant bank reload.
    sample_select.set_selected(params.sample() as u32);

    let play_button = gtk::Button::from_icon_name("media-playback-start");
    play_button.add_css_class("flat");
    play_button.add_css_class("circular");
    play_button.add_css_class("sample-play-button");
    play_button.set_valign(gtk::Align::Center);
    play_button.set_tooltip_text(Some(settings_text(
        params.language(),
        SettingsText::PlayMeasure,
    )));

    let sample_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    sample_row.set_hexpand(true);
    sample_row.append(&sample_select);
    sample_row.append(&play_button);
    content.append(&sample_row);

    let normal_wav_button = gtk::Button::with_label("Обычный WAV…");
    let accent_wav_button = gtk::Button::with_label("Акцентный WAV…");

    let wav_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    wav_row.set_hexpand(true);
    wav_row.append(&normal_wav_button);
    wav_row.append(&accent_wav_button);
    // Custom WAV import stays available in code but is temporarily hidden
    // from the settings UI. Restore it by appending `wav_row` here again.
    // content.append(&wav_row);

    let first_beat = adw::SwitchRow::builder()
        .title(settings_text(
            params.language(),
            SettingsText::HighlightFirstBeat,
        ))
        .build();
    first_beat.set_active(params.highlight_first());
    let params_for_toggle = params.clone();
    first_beat.connect_active_notify(move |toggle| {
        params_for_toggle.set_highlight_first(toggle.is_active());
    });

    let settings_group = adw::PreferencesGroup::new();
    settings_group.add(&first_beat);
    content.append(&settings_group);

    let about_button = gtk::Button::builder()
        .label(about_text(params.language()))
        .css_classes(["flat", "caption", "about-button"])
        .halign(gtk::Align::Center)
        .margin_top(12)
        .build();
    let parent_for_about = dialog.clone();
    about_button.connect_clicked(move |_| show_about(&parent_for_about));
    content.append(&about_button);

    toolbar.set_content(Some(&content));
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&toolbar));
    let toast = ErrorToast::new(&overlay);

    // Update the modal in place instead of rebuilding it, so the switch state
    // and any in-progress preview stay intact while the language changes.
    let dialog_for_language = dialog.clone();
    let header_title_for_language = header_title.clone();
    let language_title_for_language = language_title.clone();
    let sample_title_for_language = sample_title.clone();
    let sample_select_for_language = sample_select.clone();
    let play_button_for_language = play_button.clone();
    let first_beat_for_language = first_beat.clone();
    let about_button_for_language = about_button.clone();
    let params_for_language = params.clone();
    language_select.connect_selected_notify(move |select| {
        let language = Language::from_index(select.selected() as usize);
        params_for_language.set_language(language);

        let settings = settings_text(language, SettingsText::Settings);
        dialog_for_language.set_title(Some(settings));
        header_title_for_language.set_text(settings);
        language_title_for_language.set_text(settings_text(language, SettingsText::Language));
        sample_title_for_language.set_text(settings_text(language, SettingsText::Sample));
        play_button_for_language
            .set_tooltip_text(Some(settings_text(language, SettingsText::PlayMeasure)));
        first_beat_for_language
            .set_title(settings_text(language, SettingsText::HighlightFirstBeat));
        about_button_for_language.set_label(about_text(language));

        // `Classic (bundled)` is generated by the UI; candidate labels come
        // from user filenames and must not be translated.
        if let Some(sample_model) = sample_select_for_language
            .model()
            .and_then(|model| model.downcast::<gtk::StringList>().ok())
        {
            sample_model.splice(
                0,
                1,
                &[settings_text(language, SettingsText::ClassicBundled)],
            );
        }
    });

    let engine_for_list = engine.clone();
    let params_for_list = params.clone();
    let toast_for_list = toast.clone();
    sample_select.connect_selected_notify(move |select| {
        let position = select.selected() as usize;
        if position == params_for_list.sample() {
            return;
        }

        match load_sample(&engine_for_list, position) {
            Ok(()) => params_for_list.set_sample(position),
            Err(error) => {
                eprintln!("Could not load sample: {error:#}");
                toast_for_list.show(&format!(
                    "{}: {error:#}",
                    error_text(params_for_list.language(), ErrorText::LoadSample)
                ));
                // Restore the dropdown to the sample that is actually loaded
                // so the visible selection never disagrees with the bank.
                select.set_selected(params_for_list.sample() as u32);
            }
        }
    });

    let engine_for_preview = engine.clone();
    let dropdown_for_preview = sample_select.clone();
    let button_for_preview = play_button.clone();
    let params_for_preview = params.clone();
    let busy_for_preview = preview_busy.clone();
    let toast_for_preview = toast.clone();
    play_button.connect_clicked(move |_| {
        if busy_for_preview.get() {
            return;
        }

        let position = dropdown_for_preview.selected() as usize;
        if let Err(error) = load_sample(&engine_for_preview, position) {
            eprintln!("Could not load sample for preview: {error:#}");
            toast_for_preview.show(&format!(
                "{}: {error:#}",
                error_text(params_for_preview.language(), ErrorText::LoadSample)
            ));
            return;
        }

        params_for_preview.set_sample(position);
        busy_for_preview.set(true);
        let preview_result = {
            let mut engine = engine_for_preview.borrow_mut();
            engine.preview_measure()
        };

        match preview_result {
            Ok(duration) => {
                button_for_preview.set_icon_name("media-playback-stop");
                button_for_preview.set_sensitive(false);
                let button = button_for_preview.clone();
                let busy = busy_for_preview.clone();
                glib::timeout_add_local_once(duration + Duration::from_millis(120), move || {
                    button.set_icon_name("media-playback-start");
                    button.set_sensitive(true);
                    busy.set(false);
                });
            }
            Err(error) => {
                eprintln!("Could not play sample: {error:#}");
                toast_for_preview.show(&format!(
                    "{}: {error:#}",
                    error_text(params_for_preview.language(), ErrorText::PlaySample)
                ));
                button_for_preview.set_sensitive(true);
                busy_for_preview.set(false);
            }
        }
    });

    let parent_for_normal = parent.clone();
    let engine_for_normal = engine.clone();
    let toast_for_normal = toast.clone();
    let params_for_normal = params.clone();
    normal_wav_button.connect_clicked(move |_| {
        choose_wav(
            &parent_for_normal,
            &engine_for_normal,
            false,
            &params_for_normal,
            &toast_for_normal,
        );
    });
    let parent_for_accent = parent.clone();
    let engine_for_accent = engine.clone();
    let toast_for_accent = toast.clone();
    let params_for_accent = params.clone();
    accent_wav_button.connect_clicked(move |_| {
        choose_wav(
            &parent_for_accent,
            &engine_for_accent,
            true,
            &params_for_accent,
            &toast_for_accent,
        );
    });

    dialog.set_content(Some(&overlay));
    apply_pointer_cursors(dialog.upcast_ref());
    (dialog, sample_select, language_select)
}

/// GTK 4 does not support the `cursor` CSS property, so interactive widgets
/// must get their pointer cursor through `set_cursor_from_name`. Walk the
/// widget tree once and mark every clickable or draggable widget.
fn apply_pointer_cursors(widget: &gtk::Widget) {
    let is_interactive = widget.downcast_ref::<gtk::Button>().is_some()
        || widget.downcast_ref::<gtk::CheckButton>().is_some()
        || widget.downcast_ref::<gtk::DropDown>().is_some()
        || widget.downcast_ref::<gtk::Scale>().is_some()
        || widget.downcast_ref::<gtk::SpinButton>().is_some()
        || widget.downcast_ref::<gtk::Switch>().is_some();

    let mut needs_pointer = is_interactive;
    if !needs_pointer {
        if let Some(row) = widget.downcast_ref::<gtk::ListBoxRow>() {
            needs_pointer = row.is_activatable() || row.is_selectable();
        }
    }

    if needs_pointer {
        widget.set_cursor_from_name(Some("pointer"));
    }

    let mut child = widget.first_child();
    while let Some(current) = child {
        apply_pointer_cursors(&current);
        child = current.next_sibling();
    }
}

fn load_sample(engine: &Rc<RefCell<AudioEngine>>, position: usize) -> anyhow::Result<()> {
    let sample_rate = engine.borrow().sample_rate;
    let result = if position == 0 {
        SoundBank::embedded(sample_rate)
    } else {
        SoundBank::candidate(position - 1, sample_rate)
    };

    engine.borrow().set_sound_bank(result?);
    Ok(())
}

fn register_icon_resources() {
    let bytes = glib::Bytes::from_static(super::style::ICONS);
    let resource =
        gio::Resource::from_data(&bytes).expect("compiled icon resource should be valid");
    gio::resources_register(&resource);
}

fn add_icon_theme_resource_path() {
    if let Some(display) = gdk::Display::default() {
        IconTheme::for_display(&display).add_resource_path(super::style::ICON_RESOURCE_PATH);
    }
}

fn load_css() {
    add_icon_theme_resource_path();

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

#[cfg(all(test, feature = "ui"))]
mod icon_resource_tests {
    use super::*;

    #[test]
    fn compiled_icons_resolve_from_the_application_resource_path() {
        if !gtk::is_initialized() {
            gtk::init().expect("GTK should initialize in the test harness");
        }

        register_icon_resources();

        let theme = gtk::IconTheme::new();
        theme.add_resource_path(crate::app::style::ICON_RESOURCE_PATH);

        for name in ["metrognome", "metrognome-symbolic", "metrognome-header"] {
            theme.lookup_icon(
                name,
                &[],
                32,
                1,
                gtk::TextDirection::Ltr,
                gtk::IconLookupFlags::empty(),
            );
            assert!(theme.has_icon(name), "{name} was not registered");
        }
    }
}
