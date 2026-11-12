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
use crate::model::{
    default_states, meter_label, tempo_marking, AppSettings, BeatState, Language, MeterChange,
    MeterUpdate, SharedParams, LANGUAGES, MAX_BEATS, MAX_CUSTOM_METERS, MAX_VOLUME,
    METER_DENOMINATORS, PRESET_METERS, TEMPO_MARKINGS,
};
use crate::update::{self, AssetKind, InstallContext, Release};

/// Visible settings-modal copy. Keeping the native language in the selector
/// while translating the surrounding controls makes a language switch obvious
/// even when the user cannot read the current interface language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsText {
    Settings,
    Language,
    Sample,
    PlayMeasure,
    HighlightFirstBeat,
    AboutTakto,
    CheckForUpdates,
    EditSizes,
    ShownInTheBar,
    HiddenSizes,
    UpToFourChips,
    CurrentMeterVisible,
    AddCustomSize,
    EditCustomSize,
    AddButton,
    SaveButton,
    CancelButton,
    RemoveButton,
    EditButton,
    BeatsLabel,
    UnitLabel,
    SizeExists,
}

/// One row per `Language`, one column per `SettingsText`. New columns keep
/// their English text until they are translated; the Russian row is filled
/// in as the reference for the remaining localizations.
const SETTINGS_TEXTS: [[&str; 22]; 25] = [
    [
        "Settings",
        "Language",
        "Sample",
        "Play measure",
        "Highlight first beat",
        "About Takto",
        "Check for updates",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "设置",
        "语言",
        "样本",
        "播放一小节",
        "突出第一拍",
        "关于 Takto",
        "检查更新",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "सेटिंग्स",
        "भाषा",
        "सैंपल",
        "एक माप बजाएँ",
        "पहला बीट हाइलाइट करें",
        "Takto के बारे में",
        "अपडेट के लिए जाँच करें",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "Ajustes",
        "Idioma",
        "Muestra",
        "Reproducir compás",
        "Resaltar primer tiempo",
        "Acerca de Takto",
        "Buscar actualizaciones",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "Paramètres",
        "Langue",
        "Échantillon",
        "Jouer une mesure",
        "Mettre en valeur le premier temps",
        "À propos de Takto",
        "Rechercher des mises à jour",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "الإعدادات",
        "اللغة",
        "العينة",
        "تشغيل ميزان",
        "تمييز النبضة الأولى",
        "حول Takto",
        "التحقق من التحديثات",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "সেটিংস",
        "ভাষা",
        "স্যাম্পল",
        "মাপ বাজান",
        "প্রথম বিট হাইলাইট করুন",
        "Takto সম্পর্কে",
        "আপডেট চেক করুন",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "Configurações",
        "Idioma",
        "Amostra",
        "Tocar compasso",
        "Destacar primeiro tempo",
        "Sobre o Takto",
        "Verificar atualizações",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "Настройки",
        "Язык",
        "Семпл",
        "Воспроизвести такт",
        "Выделять первый удар",
        "О программе",
        "Проверить обновления",
        "Изменить размеры",
        "В ряду",
        "Скрытые",
        "Максимум 4 чипа",
        "Текущий размер всегда в ряду",
        "Свой размер",
        "Правка размера",
        "Добавить",
        "Сохранить",
        "Отмена",
        "Удалить",
        "Править",
        "Доли",
        "Единица",
        "Такой размер уже есть",
    ],
    [
        "ترتیبات",
        "زبان",
        "سیمپل",
        "میزان چلائیں",
        "پہلی دھن نمایاں کریں",
        "Takto کے بارے میں",
        "تازہ کاریں چیک کریں",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "Einstellungen",
        "Sprache",
        "Sample",
        "Takt abspielen",
        "Ersten Schlag hervorheben",
        "Über Takto",
        "Nach Updates suchen",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "設定",
        "言語",
        "サンプル",
        "1小節を再生",
        "最初のビートを強調",
        "Takto について",
        "アップデートを確認",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "ਸੈਟਿੰਗਾਂ",
        "ਭਾਸ਼ਾ",
        "ਸੈਂਪਲ",
        "ਤਾਲ ਚਲਾਓ",
        "ਪਹਿਲਾ ਬੀਟ ਉਭਾਰੋ",
        "Takto ਬਾਰੇ",
        "ਅੱਪਡੇਟ ਚੈੱਕ ਕਰੋ",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "सेटिंग्ज",
        "भाषा",
        "नमुना",
        "ताल वाजवा",
        "पहिला ठोका ठळक करा",
        "Takto विषयी",
        "अपडेट तपासा",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "సెట్టింగ్‌లు",
        "భాష",
        "సాంపిల్",
        "తాళం వాయించు",
        "మొదటి బీట్ హైలైట్ చేయండి",
        "Takto గురించి",
        "అప్‌డేట్‌లను తనిఖీ చేయండి",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "Ayarlar",
        "Dil",
        "Örnek",
        "Ölçüyü çal",
        "İlk vurguyu öne çıkar",
        "Takto Hakkında",
        "Güncellemeleri denetle",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "설정",
        "언어",
        "샘플",
        "마디 재생",
        "첫 박자 강조",
        "Takto 정보",
        "업데이트 확인",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "Cài đặt",
        "Ngôn ngữ",
        "Mẫu âm thanh",
        "Phát một ô nhịp",
        "Làm nổi bật nhịp đầu",
        "Giới thiệu Takto",
        "Kiểm tra cập nhật",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "Impostazioni",
        "Lingua",
        "Campione",
        "Riproduci misura",
        "Evidenzia primo battito",
        "Informazioni su Takto",
        "Verifica aggiornamenti",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "Pengaturan",
        "Bahasa",
        "Sampel",
        "Putar birama",
        "Sorot ketukan pertama",
        "Tentang Takto",
        "Periksa pembaruan",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "Ustawienia",
        "Język",
        "Próbka",
        "Zagraj takt",
        "Wyróżnij pierwsze uderzenie",
        "O Takto",
        "Sprawdź aktualizacje",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "Налаштування",
        "Мова",
        "Семпл",
        "Відтворити такт",
        "Виділяти перший удар",
        "Про програму",
        "Перевірити оновлення",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "Instellingen",
        "Taal",
        "Voorbeeld",
        "Maat afspelen",
        "Eerste slag markeren",
        "Over Takto",
        "Controleren op updates",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "การตั้งค่า",
        "ภาษา",
        "ตัวอย่างเสียง",
        "เล่นหนึ่งห้อง",
        "เน้นจังหวะแรก",
        "เกี่ยวกับ Takto",
        "ตรวจสอบการอัปเดต",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
    [
        "Mipangilio",
        "Lugha",
        "Sampuli",
        "Cheza kipimo",
        "Angazia mdundo wa kwanza",
        "Kuhusu Takto",
        "Angalia masasisho",
        "Edit sizes",
        "Shown in the bar",
        "Hidden",
        "Up to 4 chips",
        "Current meter stays visible",
        "Add custom size",
        "Edit custom size",
        "Add",
        "Save",
        "Cancel",
        "Remove",
        "Edit",
        "Beats",
        "Unit",
        "This size already exists",
    ],
];

fn settings_text(language: Language, text: SettingsText) -> &'static str {
    SETTINGS_TEXTS[language.index()][text as usize]
}

fn about_text(language: Language) -> &'static str {
    settings_text(language, SettingsText::AboutTakto)
}

fn show_about(parent: &adw::Window) {
    adw::AboutWindow::builder()
        .transient_for(parent)
        .application_name("Takto")
        .application_icon("takto")
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
    CheckUpdates,
    InstallUpdate,
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
            ErrorText::CheckUpdates => "Не удалось проверить обновления",
            ErrorText::InstallUpdate => "Не удалось установить обновление",
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
        ErrorText::CheckUpdates => "Could not check for updates",
        ErrorText::InstallUpdate => "Could not install the update",
    }
}

/// Copy for the update banner and the manual check feedback. Like
/// `ErrorText`, Russian keeps a reviewed translation and every other
/// language falls back to English.
#[derive(Debug, Clone, Copy)]
enum UpdateText {
    /// Banner title prefix; the version is appended at the call site.
    Available,
    Update,
    RestartToUpdate,
    Restart,
    UpToDate,
    Downloading,
    Installing,
}

fn update_text(language: Language, text: UpdateText) -> &'static str {
    if language == Language::Russian {
        return match text {
            UpdateText::Available => "Доступна новая версия",
            UpdateText::Update => "Обновить",
            UpdateText::RestartToUpdate => "Перезапустите, чтобы применить обновление",
            UpdateText::Restart => "Перезапустить",
            UpdateText::UpToDate => "У вас последняя версия",
            UpdateText::Downloading => "Загрузка обновления…",
            UpdateText::Installing => "Установка обновления…",
        };
    }

    match text {
        UpdateText::Available => "New version available",
        UpdateText::Update => "Update",
        UpdateText::RestartToUpdate => "Restart to apply the update",
        UpdateText::Restart => "Restart",
        UpdateText::UpToDate => "You are on the latest version",
        UpdateText::Downloading => "Downloading update…",
        UpdateText::Installing => "Installing update…",
    }
}

/// What the background update threads report back to the GTK main loop.
enum UpdateEvent {
    /// Release lookup finished; `Ok(None)` means the app is up to date.
    Checked(Result<Option<Release>, String>),
    /// Download progress as a percentage; `None` means indeterminate.
    Progress(Option<u8>),
    /// The downloaded artifact was installed (or failed to install).
    InstallFinished(Result<(), String>),
}

/// Which terminal failure to synthesize when a task dies without reporting.
#[derive(Debug, Clone, Copy)]
enum UpdateTaskKind {
    Check,
    Install,
}

const UPDATE_TASK_DIED: &str = "update task ended unexpectedly";

/// Banner-driven update coordination for the main window. One instance per
/// window; the banner, the settings-modal check button and every background
/// task report through [`UpdateUi::handle_event`].
struct UpdateUi {
    window: adw::ApplicationWindow,
    app: Application,
    banner: adw::Banner,
    toast: Rc<ErrorToast>,
    params: Arc<SharedParams>,
    release: RefCell<Option<Release>>,
    busy: Cell<bool>,
    installed: Cell<bool>,
}

impl UpdateUi {
    fn new(
        window: &adw::ApplicationWindow,
        app: &Application,
        toast: Rc<ErrorToast>,
        params: &Arc<SharedParams>,
    ) -> Rc<Self> {
        let banner = adw::Banner::new("");
        banner.set_revealed(false);
        let updates = Rc::new(Self {
            window: window.clone(),
            app: app.clone(),
            banner,
            toast,
            params: params.clone(),
            release: RefCell::new(None),
            busy: Cell::new(false),
            installed: Cell::new(false),
        });
        let updates_for_button = updates.clone();
        updates
            .banner
            .connect_button_clicked(move |_| updates_for_button.on_banner_button());
        updates
    }

    fn language(&self) -> Language {
        self.params.language()
    }

    fn on_banner_button(self: &Rc<Self>) {
        if self.installed.get() {
            self.restart_application();
            return;
        }
        if self.busy.get() {
            return;
        }
        let Some(release) = self.release.borrow().clone() else {
            return;
        };

        match update::detect_install_context() {
            InstallContext::AppImage | InstallContext::Debian => self.start_install(release),
            // Sandboxed and development builds cannot self-install; point at
            // the release page instead of downloading anything.
            _ => self.open_release_page(&release),
        }
    }

    fn start_install(self: &Rc<Self>, release: Release) {
        let kind = match update::detect_install_context() {
            InstallContext::AppImage => AssetKind::AppImage,
            InstallContext::Debian => AssetKind::Deb,
            _ => return self.open_release_page(&release),
        };
        let Some(url) = release.asset_url(kind).map(str::to_string) else {
            // A release without the matching artifact is rare, but a manual
            // download beats a silent failure.
            return self.open_release_page(&release);
        };

        self.busy.set(true);
        self.banner.set_button_label(None::<&str>);
        self.banner
            .set_title(update_text(self.language(), UpdateText::Downloading));

        let (sender, receiver) = std::sync::mpsc::channel::<UpdateEvent>();
        std::thread::spawn(move || {
            let result = run_install_task(kind, &url, &release.version, &sender);
            let _ = sender.send(UpdateEvent::InstallFinished(
                result.map_err(|error| format!("{error:#}")),
            ));
        });
        attach_update_events(receiver, self, None, UpdateTaskKind::Install);
    }

    fn handle_event(&self, event: UpdateEvent, toast: Option<&Rc<ErrorToast>>) {
        match event {
            UpdateEvent::Checked(result) => self.handle_check_result(result, toast),
            UpdateEvent::Progress(percent) => self.show_progress(percent),
            UpdateEvent::InstallFinished(result) => self.handle_install_result(result),
        }
    }

    fn handle_check_result(
        &self,
        result: Result<Option<Release>, String>,
        toast: Option<&Rc<ErrorToast>>,
    ) {
        match result {
            Ok(Some(release)) => {
                self.release.replace(Some(release));
                self.busy.set(false);
                self.installed.set(false);
                self.show_available_state();
            }
            Ok(None) => {
                if let Some(toast) = toast {
                    toast.show(update_text(self.language(), UpdateText::UpToDate));
                }
            }
            Err(message) => {
                // Startup checks stay silent apart from stderr; manual checks
                // get an explicit toast.
                eprintln!("Update check failed: {message}");
                if let Some(toast) = toast {
                    toast.show(&format!(
                        "{}: {message}",
                        error_text(self.language(), ErrorText::CheckUpdates)
                    ));
                }
            }
        }
    }

    fn show_progress(&self, percent: Option<u8>) {
        if !self.busy.get() {
            return;
        }
        let language = self.language();
        let title = match percent {
            Some(percent) => {
                format!(
                    "{} {percent}%",
                    update_text(language, UpdateText::Downloading)
                )
            }
            None => update_text(language, UpdateText::Installing).to_string(),
        };
        self.banner.set_title(&title);
    }

    fn handle_install_result(&self, result: Result<(), String>) {
        self.busy.set(false);
        let language = self.language();
        match result {
            Ok(()) => {
                self.installed.set(true);
                self.banner
                    .set_title(update_text(language, UpdateText::RestartToUpdate));
                self.banner
                    .set_button_label(Some(update_text(language, UpdateText::Restart)));
                self.banner.set_revealed(true);
            }
            Err(message) => {
                self.installed.set(false);
                eprintln!("Update install failed: {message}");
                self.toast.show(&format!(
                    "{}: {message}",
                    error_text(language, ErrorText::InstallUpdate)
                ));
                self.show_available_state();
            }
        }
    }

    fn show_available_state(&self) {
        let language = self.language();
        let release = self.release.borrow();
        let Some(release) = release.as_ref() else {
            self.banner.set_revealed(false);
            return;
        };
        self.banner.set_title(&format!(
            "{}: Takto {}",
            update_text(language, UpdateText::Available),
            release.version
        ));
        self.banner
            .set_button_label(Some(update_text(language, UpdateText::Update)));
        self.banner.set_revealed(true);
    }

    /// Re-translates the banner after a language switch. In-progress titles
    /// reflect task state instead and are left untouched.
    fn sync_language(&self) {
        if !self.banner.is_revealed() || self.busy.get() {
            return;
        }
        if self.installed.get() {
            let language = self.language();
            self.banner
                .set_title(update_text(language, UpdateText::RestartToUpdate));
            self.banner
                .set_button_label(Some(update_text(language, UpdateText::Restart)));
        } else {
            self.show_available_state();
        }
    }

    fn open_release_page(&self, release: &Release) {
        gtk::UriLauncher::new(&release.page_url).launch(
            Some(&self.window),
            None::<&gio::Cancellable>,
            move |result| {
                if let Err(error) = result {
                    eprintln!("Could not open the release page: {error:#}");
                }
            },
        );
    }

    fn restart_application(&self) {
        match std::env::current_exe() {
            Ok(executable) => {
                if let Err(error) = std::process::Command::new(executable).spawn() {
                    eprintln!("Could not restart the application: {error:#}");
                }
            }
            Err(error) => eprintln!("Could not locate the application binary: {error:#}"),
        }
        self.app.quit();
    }
}

/// Runs a release check on a worker thread and reports through `sender`.
fn spawn_update_check(sender: std::sync::mpsc::Sender<UpdateEvent>) {
    std::thread::spawn(move || {
        let result = update::fetch_latest_release()
            .map(|release| {
                if update::is_newer(&release.version, env!("CARGO_PKG_VERSION")) {
                    Some(release)
                } else {
                    None
                }
            })
            .map_err(|error| format!("{error:#}"));
        let _ = sender.send(UpdateEvent::Checked(result));
    });
}

/// One manual check from the settings modal: spawns the lookup task and a
/// result drain, and re-enables the button when either finishes.
fn run_manual_update_check(updates: &Rc<UpdateUi>, toast: &Rc<ErrorToast>, button: &gtk::Button) {
    button.set_sensitive(false);
    let (sender, receiver) = std::sync::mpsc::channel::<UpdateEvent>();
    spawn_update_check(sender);

    let updates = updates.clone();
    let toast = toast.clone();
    let button = button.clone();
    glib::timeout_add_local(Duration::from_millis(150), move || {
        match receiver.try_recv() {
            Ok(event) => {
                button.set_sensitive(true);
                updates.handle_event(event, Some(&toast));
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                button.set_sensitive(true);
                updates.handle_event(
                    UpdateEvent::Checked(Err(UPDATE_TASK_DIED.to_string())),
                    Some(&toast),
                );
                glib::ControlFlow::Break
            }
        }
    });
}

/// Downloads the release artifact and installs it for the detected context.
/// Progress percentage events flow through `sender` while it works.
fn run_install_task(
    kind: AssetKind,
    url: &str,
    version: &str,
    sender: &std::sync::mpsc::Sender<UpdateEvent>,
) -> anyhow::Result<()> {
    let dest = update::download_dest(kind, version);
    let mut report = |total: Option<u64>, downloaded: u64| {
        let percent = total
            .filter(|total| *total > 0)
            .map(|total| ((downloaded.min(total) as f64 / total as f64) * 100.0).round() as u8);
        let _ = sender.send(UpdateEvent::Progress(percent));
    };
    update::download_to(url, &dest, &mut report)?;

    match update::detect_install_context() {
        InstallContext::AppImage => {
            let appimage = update::appimage_path()?;
            update::install_appimage(&appimage, &dest)?;
        }
        InstallContext::Debian => {
            // No meaningful progress signal while polkit + apt-get run.
            let _ = sender.send(UpdateEvent::Progress(None));
            update::install_debian(&dest)?;
        }
        // Unreachable through `start_install`, but harmless to ignore.
        _ => {}
    }

    // The staged copy is no longer needed: AppImage installs were copied
    // into place and .deb installs went through apt-get.
    let _ = std::fs::remove_file(&dest);
    Ok(())
}

/// Drains the task's events from the GTK main loop until the terminal event
/// arrives. `toast` decides whether failures surface as popups (manual
/// checks) or stay on stderr (startup checks).
fn attach_update_events(
    receiver: std::sync::mpsc::Receiver<UpdateEvent>,
    updates: &Rc<UpdateUi>,
    toast: Option<Rc<ErrorToast>>,
    kind: UpdateTaskKind,
) {
    let updates = updates.clone();
    glib::timeout_add_local(Duration::from_millis(150), move || {
        let outcome = match receiver.try_recv() {
            Ok(event) => Ok(event),
            Err(std::sync::mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // The thread died without reporting (a panic, for example);
                // synthesize a terminal error so the banner and the settings
                // button reset instead of hanging forever.
                Err(UPDATE_TASK_DIED.to_string())
            }
        };

        let event = match (kind, outcome) {
            (UpdateTaskKind::Check | UpdateTaskKind::Install, Ok(event)) => event,
            (UpdateTaskKind::Check, Err(message)) => UpdateEvent::Checked(Err(message)),
            (UpdateTaskKind::Install, Err(message)) => UpdateEvent::InstallFinished(Err(message)),
        };
        // Download progress keeps the loop alive; only Checked and
        // InstallFinished end the task.
        let terminal = !matches!(event, UpdateEvent::Progress(_));
        match toast.as_ref() {
            Some(toast) => updates.handle_event(event, Some(toast)),
            None => updates.handle_event(event, None),
        }
        if terminal {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
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
        .application_id("io.github.ez404_36.Takto")
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
            eprintln!("First candidate sample failed to load: {error:#}");
        }
    }

    load_css();
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Takto")
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

    // Release-based updates: a banner surfaces new versions and its button
    // downloads and installs the matching artifact (see the `update` module).
    let updates = UpdateUi::new(&window, app, toast.clone(), &params);
    vbox.prepend(&updates.banner);

    // Check quietly once shortly after launch. Errors stay on stderr so a
    // network hiccup never opens the app with an alarming popup.
    let (update_sender, update_receiver) = std::sync::mpsc::channel::<UpdateEvent>();
    let updates_for_startup = updates.clone();
    glib::timeout_add_local_once(Duration::from_secs(2), move || {
        spawn_update_check(update_sender);
        attach_update_events(
            update_receiver,
            &updates_for_startup,
            None,
            UpdateTaskKind::Check,
        );
    });

    vbox.append(&build_pattern_row(&window, &params));
    vbox.append(&build_tempo_card(&window, &overlay, &params));
    vbox.append(&build_beat_dots(&params));
    vbox.append(&build_controls(&window, &engine, &params, &toast, &updates));

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
        .icon_name("takto-header")
        .pixel_size(18)
        .build();
    logo.add_css_class("app-logo");
    title.append(&logo);

    let label = gtk::Label::new(Some("Takto"));
    label.add_css_class("takto-title");
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

fn build_pattern_row(window: &adw::ApplicationWindow, params: &Arc<SharedParams>) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.set_valign(gtk::Align::Center);
    row.set_halign(gtk::Align::Center);

    let chips = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.append(&chips);

    let pencil = gtk::Button::from_icon_name("document-edit-symbolic");
    pencil.add_css_class("flat");
    pencil.set_valign(gtk::Align::Center);
    pencil.set_cursor_from_name(Some("pointer"));
    pencil.set_tooltip_text(Some(settings_text(
        params.language(),
        SettingsText::EditSizes,
    )));
    let window_for_dialog = window.clone();
    let params_for_dialog = params.clone();
    pencil.connect_clicked(move |_| {
        let dialog = build_sizes_dialog(&window_for_dialog, &params_for_dialog);
        dialog.present();
    });
    row.append(&pencil);

    rebuild_pattern_chips(&chips, params);

    // The sizes dialog edits the visible meter set from outside this row, so
    // a light poll keeps the chips in sync the same way the beat dots track
    // the audio thread. Rebuilding is cheap: at most four buttons.
    let chips_for_poll = chips;
    let params_for_poll = params.clone();
    let signature = Rc::new(RefCell::new(pattern_row_signature(params)));
    let signature_for_poll = signature.clone();
    glib::timeout_add_local(Duration::from_millis(150), move || {
        let desired = pattern_row_signature(&params_for_poll);
        if signature_for_poll.borrow().as_str() != desired.as_str() {
            rebuild_pattern_chips(&chips_for_poll, &params_for_poll);
            *signature_for_poll.borrow_mut() = desired;
        }
        glib::ControlFlow::Continue
    });

    row
}

/// Fingerprint of everything the chip row renders: the visible meter set in
/// display order plus the active meter. When it changes, the chips rebuild.
fn pattern_row_signature(params: &Arc<SharedParams>) -> String {
    let mut signature = params
        .visible_meters()
        .iter()
        .map(|&(beats, denominator)| format!("{beats}/{denominator}"))
        .collect::<Vec<_>>()
        .join(",");
    signature.push_str(&format!("|{:?}", params.meter()));
    signature
}

fn rebuild_pattern_chips(row: &gtk::Box, params: &Arc<SharedParams>) {
    // GTK's child list is live: removing a widget while iterating it shifts
    // the remaining indices and can leave stale buttons in the row. Snapshot
    // the children first so every old chip is removed reliably.
    let old_children: Vec<gtk::glib::Object> = row
        .observe_children()
        .into_iter()
        .filter_map(Result::ok)
        .collect();
    for child in old_children {
        if let Some(button) = child.downcast_ref::<gtk::ToggleButton>() {
            row.remove(button);
        }
    }

    let active_meter = params.meter();
    let mut first = None;
    for &(beats, denominator) in &params.visible_meters() {
        let button = gtk::ToggleButton::with_label(&meter_label(beats, denominator));
        button.add_css_class("pill-button");
        button.set_width_request(68);
        button.set_hexpand(false);

        if let Some(previous) = first.as_ref() {
            button.set_group(Some(previous));
        } else {
            first = Some(button.clone());
        }

        // Restore the active pill to the persisted meter before installing
        // the handler so restoring cannot replace the saved beat pattern.
        if (beats, denominator) == active_meter {
            button.set_active(true);
        }

        let params = params.clone();
        button.connect_toggled(move |toggle| {
            if toggle.is_active() {
                params.set_meter_and_states(beats, denominator, &default_states(beats));
            }
        });

        row.append(&button);
    }
}

/// Which custom entry the editor section is rewriting. `None` means it is in
/// "add" mode.
struct SizesDialog {
    params: Arc<SharedParams>,
    shown_list: gtk::ListBox,
    hidden_list: gtk::ListBox,
    editor_title: gtk::Label,
    beats_spin: gtk::SpinButton,
    unit_buttons: Vec<(u32, gtk::ToggleButton)>,
    preview_label: gtk::Label,
    exists_hint: gtk::Label,
    add_button: gtk::Button,
    save_button: gtk::Button,
    cancel_button: gtk::Button,
    editing: Cell<Option<usize>>,
}

impl SizesDialog {
    fn editor_value(&self) -> (u32, u32) {
        let beats = self.beats_spin.value() as u32;
        let denominator = self
            .unit_buttons
            .iter()
            .find(|(_, button)| button.is_active())
            .map(|(unit, _)| *unit)
            .unwrap_or(4);
        (beats, denominator)
    }

    fn set_editor_value(&self, beats: u32, denominator: u32) {
        self.beats_spin
            .set_value(f64::from(beats.clamp(1, MAX_BEATS)));
        for (unit, button) in &self.unit_buttons {
            button.set_active(*unit == denominator);
        }
    }

    fn sync_editor(self: &Rc<Self>) {
        let (beats, denominator) = self.editor_value();
        self.preview_label
            .set_text(&meter_label(beats, denominator));

        match self.editing.get() {
            None => {
                self.editor_title.set_text(settings_text(
                    self.params.language(),
                    SettingsText::AddCustomSize,
                ));
                self.add_button.set_visible(true);
                self.save_button.set_visible(false);
                self.cancel_button.set_visible(false);
                self.exists_hint.set_visible(false);
                // The model keeps the cap; a full list disables the button so
                // the click can never be a silent no-op.
                self.add_button
                    .set_sensitive(self.params.custom_meters().len() < MAX_CUSTOM_METERS);
            }
            Some(index) => {
                self.editor_title.set_text(settings_text(
                    self.params.language(),
                    SettingsText::EditCustomSize,
                ));
                self.add_button.set_visible(false);
                self.save_button.set_visible(true);
                self.cancel_button.set_visible(true);
                let duplicate = self.params.meter_conflicts(Some(index), beats, denominator);
                self.save_button.set_sensitive(!duplicate);
                self.exists_hint.set_visible(duplicate);
            }
        }
    }

    fn rebuild_lists(self: &Rc<Self>) {
        clear_list(&self.shown_list);
        clear_list(&self.hidden_list);

        let active = self.params.meter();
        let visible = self.params.visible_meters();
        let customs = self.params.custom_meters();

        for (position, &(beats, denominator)) in visible.iter().enumerate() {
            let custom_index = customs
                .iter()
                .position(|&meter| meter == (beats, denominator));
            self.append_meter_row(
                &self.shown_list,
                beats,
                denominator,
                true,
                Some(position),
                custom_index,
                active,
            );
        }

        // Hidden rows keep the canonical order: presets first, then custom
        // meters in the order they were added.
        for &(beats, denominator) in &PRESET_METERS {
            if !visible.contains(&(beats, denominator)) {
                self.append_meter_row(
                    &self.hidden_list,
                    beats,
                    denominator,
                    false,
                    None,
                    None,
                    active,
                );
            }
        }
        for (custom_index, &(beats, denominator)) in customs.iter().enumerate() {
            if !visible.contains(&(beats, denominator)) {
                self.append_meter_row(
                    &self.hidden_list,
                    beats,
                    denominator,
                    false,
                    None,
                    Some(custom_index),
                    active,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn append_meter_row(
        self: &Rc<Self>,
        list: &gtk::ListBox,
        beats: u32,
        denominator: u32,
        visible_row: bool,
        drag_index: Option<usize>,
        custom_index: Option<usize>,
        active: (u32, u32),
    ) {
        let state = Rc::clone(self);
        let row = adw::ActionRow::builder()
            .title(meter_label(beats, denominator))
            .build();

        // The visibility switch moves a meter between the two groups. The
        // active meter cannot leave the bar: hiding it would leave playback
        // running with no chip pressed.
        let switch = gtk::Switch::new();
        switch.set_valign(gtk::Align::Center);
        switch.set_active(visible_row);
        if active == (beats, denominator) {
            switch.set_sensitive(false);
            switch.set_tooltip_text(Some(settings_text(
                self.params.language(),
                SettingsText::CurrentMeterVisible,
            )));
        }
        {
            let state = state.clone();
            let switch_for_toggle = switch.clone();
            switch.connect_active_notify(move |_| {
                let enabled = switch_for_toggle.is_active();
                // The model refuses to enable a fifth chip; the rebuild then
                // resets the switch to the real state.
                state.params.set_meter_visible(beats, denominator, enabled);
                state.rebuild_lists();
            });
        }
        row.add_suffix(&switch);

        if let Some(custom_index) = custom_index {
            let edit = gtk::Button::from_icon_name("document-edit-symbolic");
            edit.add_css_class("flat");
            edit.set_valign(gtk::Align::Center);
            edit.set_cursor_from_name(Some("pointer"));
            edit.set_tooltip_text(Some(settings_text(
                self.params.language(),
                SettingsText::EditButton,
            )));
            {
                let state = state.clone();
                edit.connect_clicked(move |_| {
                    state.editing.set(Some(custom_index));
                    state.set_editor_value(beats, denominator);
                    state.sync_editor();
                });
            }
            row.add_suffix(&edit);

            let remove = gtk::Button::from_icon_name("user-trash-symbolic");
            remove.add_css_class("flat");
            remove.set_valign(gtk::Align::Center);
            remove.set_cursor_from_name(Some("pointer"));
            remove.set_tooltip_text(Some(settings_text(
                self.params.language(),
                SettingsText::RemoveButton,
            )));
            {
                let state = state.clone();
                remove.connect_clicked(move |_| {
                    let removed = state.params.remove_custom_meter(custom_index);
                    if let Some(removed) = removed {
                        if state.params.meter() == removed {
                            pick_fallback_meter(&state.params);
                        }
                    }
                    // A removal shifts later custom indices, so an editor
                    // session pointing past it must be repaired or dropped.
                    if let Some(editing) = state.editing.get() {
                        if editing == custom_index {
                            state.editing.set(None);
                        } else if editing > custom_index {
                            state.editing.set(Some(editing - 1));
                        }
                    }
                    state.sync_editor();
                    state.rebuild_lists();
                });
            }
            row.add_suffix(&remove);
        }

        if let Some(drag_index) = drag_index {
            attach_drag_reorder(&state, &row, drag_index);
        }

        list.append(&row);
    }
}

fn build_sizes_dialog(parent: &adw::ApplicationWindow, params: &Arc<SharedParams>) -> adw::Window {
    let language = params.language();
    let dialog = adw::Window::new();
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.set_destroy_with_parent(true);
    dialog.set_title(Some(settings_text(language, SettingsText::EditSizes)));
    dialog.set_default_width(380);

    // Same close handling as the settings window: hide instead of destroy so
    // a future presentation rebuilds a native surface cleanly.
    let dialog_for_close = dialog.clone();
    dialog.connect_close_request(move |_| {
        dialog_for_close.set_visible(false);
        glib::Propagation::Stop
    });

    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let header_title = gtk::Label::new(Some(settings_text(language, SettingsText::EditSizes)));
    header.set_title_widget(Some(&header_title));
    toolbar.add_top_bar(&header);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.set_margin_top(18);
    content.set_margin_bottom(24);
    content.set_margin_start(18);
    content.set_margin_end(18);

    let shown_title = gtk::Label::new(Some(settings_text(language, SettingsText::ShownInTheBar)));
    shown_title.set_halign(gtk::Align::Start);
    shown_title.add_css_class("settings-section-title");
    content.append(&shown_title);

    let shown_list = gtk::ListBox::new();
    shown_list.set_selection_mode(gtk::SelectionMode::None);
    shown_list.add_css_class("boxed-list");
    content.append(&shown_list);

    let shown_hint = gtk::Label::new(Some(settings_text(language, SettingsText::UpToFourChips)));
    shown_hint.set_halign(gtk::Align::Start);
    shown_hint.add_css_class("caption");
    content.append(&shown_hint);

    let hidden_title = gtk::Label::new(Some(settings_text(language, SettingsText::HiddenSizes)));
    hidden_title.set_halign(gtk::Align::Start);
    hidden_title.add_css_class("settings-section-title");
    content.append(&hidden_title);

    let hidden_list = gtk::ListBox::new();
    hidden_list.set_selection_mode(gtk::SelectionMode::None);
    hidden_list.add_css_class("boxed-list");
    content.append(&hidden_list);

    let editor_title = gtk::Label::new(Some(settings_text(language, SettingsText::AddCustomSize)));
    editor_title.set_halign(gtk::Align::Start);
    editor_title.add_css_class("settings-section-title");
    content.append(&editor_title);

    let beats_spin = gtk::SpinButton::with_range(1.0, f64::from(MAX_BEATS), 1.0);
    beats_spin.set_value(4.0);
    beats_spin.set_width_request(72);

    let mut unit_buttons: Vec<(u32, gtk::ToggleButton)> = Vec::new();
    let mut first_unit = None;
    for &unit in &METER_DENOMINATORS {
        let button = gtk::ToggleButton::with_label(&unit.to_string());
        button.set_width_request(40);
        if let Some(previous) = first_unit.as_ref() {
            button.set_group(Some(previous));
        } else {
            first_unit = Some(button.clone());
        }
        if unit == 4 {
            button.set_active(true);
        }
        unit_buttons.push((unit, button));
    }

    let beats_label = gtk::Label::new(Some(settings_text(language, SettingsText::BeatsLabel)));
    beats_label.add_css_class("caption");
    beats_label.set_valign(gtk::Align::Center);
    let unit_label = gtk::Label::new(Some(settings_text(language, SettingsText::UnitLabel)));
    unit_label.add_css_class("caption");
    unit_label.set_valign(gtk::Align::Center);

    let editor_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    editor_row.set_halign(gtk::Align::Center);
    editor_row.append(&beats_label);
    editor_row.append(&beats_spin);
    editor_row.append(&unit_label);
    for (_, button) in &unit_buttons {
        editor_row.append(button);
    }
    content.append(&editor_row);

    let preview_label = gtk::Label::new(Some("4/4"));
    preview_label.add_css_class("title-2");
    preview_label.set_halign(gtk::Align::Center);
    content.append(&preview_label);

    let exists_hint = gtk::Label::new(Some(settings_text(language, SettingsText::SizeExists)));
    exists_hint.set_halign(gtk::Align::Center);
    exists_hint.add_css_class("caption");
    exists_hint.add_css_class("warning");
    exists_hint.set_visible(false);
    content.append(&exists_hint);

    let cancel_button =
        gtk::Button::with_label(settings_text(language, SettingsText::CancelButton));
    cancel_button.add_css_class("flat");
    cancel_button.set_visible(false);
    let add_button = gtk::Button::with_label(settings_text(language, SettingsText::AddButton));
    add_button.add_css_class("suggested-action");
    let save_button = gtk::Button::with_label(settings_text(language, SettingsText::SaveButton));
    save_button.add_css_class("suggested-action");
    save_button.set_visible(false);

    let buttons_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons_row.set_halign(gtk::Align::End);
    buttons_row.append(&cancel_button);
    buttons_row.append(&add_button);
    buttons_row.append(&save_button);
    content.append(&buttons_row);

    toolbar.set_content(Some(&content));
    dialog.set_content(Some(&toolbar));

    let state = Rc::new(SizesDialog {
        params: params.clone(),
        shown_list,
        hidden_list,
        editor_title,
        beats_spin,
        unit_buttons,
        preview_label,
        exists_hint,
        add_button,
        save_button,
        cancel_button,
        editing: Cell::new(None),
    });

    {
        let state = state.clone();
        let spin = state.beats_spin.clone();
        spin.connect_value_changed(move |_| {
            state.sync_editor();
        });
    }
    {
        let state = state.clone();
        let buttons = state.unit_buttons.clone();
        for (_, button) in buttons {
            let state = state.clone();
            let button_for_toggle = button.clone();
            button.connect_toggled(move |_| {
                // Only react to the button switching on: the one switching
                // off fires first and would read a valueless editor for a
                // moment.
                if button_for_toggle.is_active() {
                    state.sync_editor();
                }
            });
        }
    }
    {
        let state = state.clone();
        let add = state.add_button.clone();
        add.connect_clicked(move |_| {
            let (beats, denominator) = state.editor_value();
            let change = state.params.add_custom_meter(beats, denominator);
            if change != MeterChange::Full {
                // Presets and duplicates surface by enabling their visibility;
                // new meters appear as soon as the row has a free slot.
                state.params.set_meter_visible(beats, denominator, true);
            }
            state.rebuild_lists();
        });
    }
    {
        let state = state.clone();
        let save = state.save_button.clone();
        save.connect_clicked(move |_| {
            if let Some(index) = state.editing.get() {
                let (beats, denominator) = state.editor_value();
                if state.params.update_custom_meter(index, beats, denominator)
                    == MeterUpdate::Updated
                {
                    state.editing.set(None);
                    state.sync_editor();
                    state.rebuild_lists();
                }
            }
        });
    }
    {
        let state = state.clone();
        let cancel = state.cancel_button.clone();
        cancel.connect_clicked(move |_| {
            state.editing.set(None);
            state.sync_editor();
        });
    }

    state.rebuild_lists();
    state.sync_editor();

    dialog
}

fn clear_list(list: &gtk::ListBox) {
    let old_children: Vec<gtk::glib::Object> = list
        .observe_children()
        .into_iter()
        .filter_map(Result::ok)
        .collect();
    for child in old_children {
        if let Some(row) = child.downcast_ref::<gtk::ListBoxRow>() {
            list.remove(row);
        }
    }
}

fn attach_drag_reorder(state: &Rc<SizesDialog>, row: &adw::ActionRow, index: usize) {
    let drag = gtk::DragSource::new();
    drag.set_actions(gdk::DragAction::MOVE);
    let index_for_drag = index as u32;
    drag.connect_prepare(move |_, _x, _y| {
        let value = index_for_drag.to_value();
        Some(gdk::ContentProvider::for_value(&value))
    });
    row.add_controller(drag);

    let target = gtk::DropTarget::new(glib::Type::U32, gdk::DragAction::MOVE);
    let state_for_drop = state.clone();
    let index_for_drop = index;
    target.connect_drop(move |target, value, _x, y| {
        let Ok(from) = value.get::<u32>() else {
            return false;
        };
        let from = from as usize;
        let height = target.widget().map(|widget| widget.height()).unwrap_or(0);
        // Dropping onto the lower half of a row inserts after it.
        let mut to = if y > f64::from(height) / 2.0 {
            index_for_drop + 1
        } else {
            index_for_drop
        };
        if from < to {
            to -= 1;
        }
        state_for_drop.params.move_visible_meter(from, to);
        state_for_drop.rebuild_lists();
        true
    });
    row.add_controller(target);
}

/// After removing the active custom meter, fall back to 4/4 when it is on
/// the bar, else to the first visible chip. With nothing visible, restore
/// the default preset so playback never runs without a chip.
fn pick_fallback_meter(params: &SharedParams) {
    let visible = params.visible_meters();
    let target = if visible.contains(&(4, 4)) {
        Some((4, 4))
    } else {
        visible.first().copied()
    };
    match target {
        Some((beats, denominator)) => {
            params.set_meter_and_states(beats, denominator, &default_states(beats));
        }
        None => {
            params.set_meter_visible(4, 4, true);
            params.set_meter_and_states(4, 4, &default_states(4));
        }
    }
}

/// Builds the inline tempo selector: an expandable list of every marking
/// with its inclusive BPM range, a check on the active one, and
/// click-to-select that jumps to the range midpoint. The list lives in a
/// box shown and hidden inside the card itself instead of a popover, so it
/// works the same on every compositor and never fights window grabs.
/// Selecting a row drives the same UI updates as the adjusters so every
/// view stays in sync immediately.
/// Builds the compact tempo selector popover: one row per marking with its
/// inclusive BPM range, a check on the active one, and click-to-select that
/// jumps to the range midpoint. Selecting a row drives the same UI updates
/// as the adjusters so every view stays in sync immediately.
///
/// The popover is driven manually instead of through a `MenuButton`: it is
/// presented with `autohide` off (no seat grab) and closed by the button,
/// by a row selection or by a click anywhere else in the window. This keeps
/// it open reliably on Wayland, where a popup grab can be rejected and
/// instantly collapse an auto-hidden popover.
/// Builds the compact tempo selector: a floating list that appears inside
/// the card right under the marking (drawn by a `gtk::Overlay`, not a
/// native popover surface, so no compositor grab can dismiss it). One row
/// per marking with its inclusive BPM range, a check on the active one,
/// and click-to-select that jumps to the range midpoint. Selecting a row
/// drives the same UI updates as the adjusters so every view stays in sync
/// immediately.
fn build_tempo_selector(
    overlay: &gtk::Overlay,
    catcher: &gtk::Button,
    button: &gtk::Button,
    params: &Arc<SharedParams>,
    bpm_label: &gtk::Label,
    adjustment: &gtk::Adjustment,
    tempo_label: &gtk::Label,
) {
    let selector = gtk::Box::new(gtk::Orientation::Vertical, 0);
    selector.add_css_class("tempo-list-container");

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("tempo-list");

    let mut checks = Vec::new();
    for marking in TEMPO_MARKINGS {
        // A plain compact row instead of an ActionRow: no extra row
        // padding, so all ten markings fit the panel without scrolling.
        let row = gtk::ListBoxRow::builder()
            .activatable(true)
            .selectable(false)
            .build();
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let title = gtk::Label::new(Some(&format!(
            "{} ({}–{})",
            marking.name, marking.min_bpm, marking.max_bpm
        )));
        title.set_xalign(0.0);
        title.set_hexpand(true);
        row_box.append(&title);
        let check = gtk::Image::from_icon_name("object-select-symbolic");
        row_box.append(&check);
        row.set_child(Some(&row_box));
        list.append(&row);

        checks.push(check);
    }

    // A single activation handler for all compact rows: the row index maps
    // straight into the tempo table.
    let selector_for_row = selector.clone();
    let catcher_for_row = catcher.clone();
    let button_for_row = button.clone();
    let params_for_row = params.clone();
    let bpm_label_for_row = bpm_label.clone();
    let adjustment_for_row = adjustment.clone();
    let tempo_label_for_row = tempo_label.clone();
    list.connect_row_activated(move |_, row| {
        let marking = TEMPO_MARKINGS[row.index() as usize];
        // Clicking the marking that is already active must not move the
        // tempo: 120 BPM Allegro stays 120. Just close the panel.
        if tempo_marking(params_for_row.bpm()) != marking.name {
            let default_bpm = marking.default_bpm();
            params_for_row.set_bpm(default_bpm);
            bpm_label_for_row.set_text(&default_bpm.to_string());
            adjustment_for_row.set_value(f64::from(default_bpm));
            tempo_label_for_row.set_text(tempo_marking(default_bpm));
        }
        // Return focus to the marking button first: hiding widgets that
        // hold focus triggers GTK's "broken active state" warnings.
        button_for_row.grab_focus();
        selector_for_row.set_visible(false);
        catcher_for_row.set_visible(false);
    });

    // The floating panel: shown and hidden inside the overlay, anchored
    // under the marking button (margins are computed when it opens). The
    // list scrolls past a compact cap so the panel can never overflow the
    // window bottom.
    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .max_content_height(280)
        .propagate_natural_height(true)
        .build();
    scrolled.set_child(Some(&list));
    selector.append(&scrolled);
    selector.set_halign(gtk::Align::Start);
    selector.set_valign(gtk::Align::Start);
    selector.set_visible(false);
    overlay.add_overlay(&selector);

    // The catcher is an invisible full-window layer shown while the panel
    // is open: any press on it (i.e., anywhere outside the panel) closes
    // the panel — modal-popover behavior without a Wayland popup grab.
    let catcher_for_click = catcher.clone();
    let selector_for_click = selector.clone();
    let catcher_click = gtk::GestureClick::new();
    catcher_click.set_button(0);
    catcher_click.connect_pressed(move |_, _, _, _| {
        selector_for_click.set_visible(false);
        catcher_for_click.set_visible(false);
    });
    catcher.add_controller(catcher_click);

    // Refresh the checkmark on every open so it always reflects the tempo
    // as of the moment the user looks at the list.
    let selector_for_toggle = selector.clone();
    let catcher_for_toggle = catcher.clone();
    let overlay_for_toggle = overlay.clone();
    let params_for_toggle = params.clone();
    let button_for_toggle = button.clone();
    let checks_for_toggle = checks;
    button.connect_clicked(move |_| {
        let open = !selector_for_toggle.is_visible();
        selector_for_toggle.set_visible(open);
        catcher_for_toggle.set_visible(open);

        if open {
            // Anchor the panel's top-left corner right below the button
            // (margins are in overlay coordinates), but never let it spill
            // past the overlay's bottom edge.
            if let Some(origin) = button_for_toggle
                .compute_point(&overlay_for_toggle, &gtk::graphene::Point::new(0.0, 0.0))
            {
                let below_button = origin.y().round() as i32 + button_for_toggle.height() + 4;
                let max_top = overlay_for_toggle.height() - 280 - 16;
                selector_for_toggle.set_margin_top(below_button.min(max_top.max(0)));
                selector_for_toggle.set_margin_start(origin.x().round() as i32);
            }

            let current = tempo_marking(params_for_toggle.bpm());
            for (marking, check) in TEMPO_MARKINGS.iter().zip(&checks_for_toggle) {
                check.set_visible(marking.name == current);
            }
        }
    });

    // A click on the catcher (anywhere outside the panel) closes it.
    let catcher_for_click = catcher.clone();
    let selector_for_click = selector.clone();
    let button_for_click = button.clone();
    catcher.connect_clicked(move |_| {
        button_for_click.grab_focus();
        catcher_for_click.set_visible(false);
        selector_for_click.set_visible(false);
    });
}

#[cfg(all(test, feature = "ui"))]
fn find_tempo_button(widget: &gtk::Widget) -> Option<gtk::Button> {
    if let Some(button) = widget.downcast_ref::<gtk::Button>() {
        if button
            .css_classes()
            .contains(&"tempo-selector-button".into())
        {
            return Some(button.clone());
        }
    }
    let mut child = widget.first_child();
    while let Some(current) = child {
        if let Some(found) = find_tempo_button(&current) {
            return Some(found);
        }
        child = current.next_sibling();
    }
    None
}

#[cfg(all(test, feature = "ui"))]
fn find_tempo_selector(widget: &gtk::Widget) -> Option<gtk::Box> {
    if let Some(box_widget) = widget.downcast_ref::<gtk::Box>() {
        if box_widget
            .css_classes()
            .contains(&"tempo-list-container".into())
        {
            return Some(box_widget.clone());
        }
    }
    let mut child = widget.first_child();
    while let Some(current) = child {
        if let Some(found) = find_tempo_selector(&current) {
            return Some(found);
        }
        child = current.next_sibling();
    }
    None
}

fn build_tempo_card(
    _window: &adw::ApplicationWindow,
    overlay: &gtk::Overlay,
    params: &Arc<SharedParams>,
) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 14);
    card.add_css_class("metro-card");
    card.add_css_class("card");

    let top = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    top.set_halign(gtk::Align::Center);
    top.set_valign(gtk::Align::Center);

    let bpm = gtk::Label::new(Some(&params.bpm().to_string()));
    bpm.add_css_class("bpm-label");
    // Reserve room for three digits and keep them anchored to the
    // adjusters. Otherwise moving between 99 and 100 changes the label's
    // request and visibly shifts the adjacent buttons.
    bpm.set_width_chars(3);
    bpm.set_xalign(1.0);
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

    // The traditional tempo marking for the current BPM, shown to the left
    // above the slider. Clicking it opens the tempo selector; the adjusters
    // and the slider update it instantly, while the poll below is the
    // catch-all for every other source (tap tempo, settings restores) so
    // the marking never falls behind.
    let tempo_label = gtk::Label::new(Some(tempo_marking(params.bpm())));
    tempo_label.add_css_class("tempo-marking-label");

    // A flat button so the marking reads as a plain label at rest but
    // still gets a hover highlight, the pointer cursor and keyboard
    // activation like any other GNOME control.
    let tempo_button = gtk::Button::new();
    tempo_button.set_child(Some(&tempo_label));
    tempo_button.add_css_class("flat");
    tempo_button.add_css_class("tempo-selector-button");
    tempo_button.set_halign(gtk::Align::Start);
    tempo_button.set_valign(gtk::Align::Start);
    let adjustment = gtk::Adjustment::new(f64::from(params.bpm()), 20.0, 300.0, 1.0, 5.0, 0.0);

    // The marking belongs visually to the slider, so they share one tight
    // group instead of the card's wide 14px rhythm.
    let tempo_area = gtk::Box::new(gtk::Orientation::Vertical, 4);
    tempo_area.append(&tempo_button);
    let slider = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&adjustment));
    slider.set_digits(0);
    slider.set_hexpand(true);
    slider.add_mark(20.0, gtk::PositionType::Bottom, Some("20"));
    slider.add_mark(80.0, gtk::PositionType::Bottom, Some("80"));
    slider.add_mark(140.0, gtk::PositionType::Bottom, Some("140"));
    slider.add_mark(200.0, gtk::PositionType::Bottom, Some("200"));
    slider.add_mark(260.0, gtk::PositionType::Bottom, Some("260"));
    tempo_area.append(&slider);

    // Invisible full-window catcher shown while the panel is open: any
    // click outside the panel lands here and simply closes the panel
    // (modal-popover behavior without a Wayland popup grab).
    let tempo_catcher = gtk::Button::new();
    tempo_catcher.add_css_class("tempo-catcher");
    // The catcher must never take keyboard focus: hiding a focused widget
    // breaks GTK's active-state accounting (Gtk-WARNING noise).
    tempo_catcher.set_can_focus(false);
    tempo_catcher.set_focusable(false);
    tempo_catcher.set_visible(false);
    overlay.add_overlay(&tempo_catcher);
    build_tempo_selector(
        overlay,
        &tempo_catcher,
        &tempo_button,
        params,
        &bpm,
        &adjustment,
        &tempo_label,
    );

    let bpm_for_plus = bpm.clone();
    let tempo_for_plus = tempo_label.clone();
    let adjustment_for_plus = adjustment.clone();
    let params_for_plus = params.clone();
    increase.connect_clicked(move |_| {
        let next = params_for_plus.bpm().saturating_add(1).min(300);
        params_for_plus.set_bpm(next);
        bpm_for_plus.set_text(&next.to_string());
        tempo_for_plus.set_text(tempo_marking(next));
        adjustment_for_plus.set_value(f64::from(next));
    });

    let bpm_for_minus = bpm.clone();
    let tempo_for_minus = tempo_label.clone();
    let adjustment_for_minus = adjustment.clone();
    let params_for_minus = params.clone();
    decrease.connect_clicked(move |_| {
        let next = params_for_minus.bpm().saturating_sub(1).max(20);
        params_for_minus.set_bpm(next);
        bpm_for_minus.set_text(&next.to_string());
        tempo_for_minus.set_text(tempo_marking(next));
        adjustment_for_minus.set_value(f64::from(next));
    });

    let bpm_for_scale = bpm.clone();
    let tempo_for_scale = tempo_label.clone();
    let params_for_scale = params.clone();
    slider.connect_value_changed(move |scale| {
        params_for_scale.set_bpm(scale.value().round() as u32);
        // Show the clamped value so dragging to the slider's edge cannot
        // briefly display a BPM the scheduler will never use.
        bpm_for_scale.set_text(&params_for_scale.bpm().to_string());
        tempo_for_scale.set_text(tempo_marking(params_for_scale.bpm()));
    });

    let bpm_for_poll = bpm.clone();
    let adjustment_for_poll = adjustment.clone();
    let tempo_for_poll = tempo_label.clone();
    let params_for_poll = params.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        let shown = bpm_for_poll.text().parse::<u32>().unwrap_or_default();
        let actual = params_for_poll.bpm();

        if shown != actual {
            bpm_for_poll.set_text(&actual.to_string());
            adjustment_for_poll.set_value(f64::from(actual));
            tempo_for_poll.set_text(tempo_marking(actual));
        }

        glib::ControlFlow::Continue
    });

    let volume_adjustment = gtk::Adjustment::new(
        f64::from(params.volume()),
        0.0,
        f64::from(MAX_VOLUME),
        0.01,
        0.05,
        0.0,
    );
    let volume = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&volume_adjustment));
    volume.set_digits(0);
    // Show the value as a percentage so the boost range above 100% reads
    // naturally instead of as a bare multiplier.
    volume.set_format_value_func(move |_, value| format!("{:.0}%", value * 100.0));
    volume.set_hexpand(true);
    let params_for_volume = params.clone();
    let initial_volume_icon = volume_icon_name(f64::from(params.volume()));
    let low_volume = gtk::Image::from_icon_name(initial_volume_icon);
    let volume_icon = low_volume.clone();
    volume.connect_value_changed(move |scale| {
        let value = scale.value();
        params_for_volume.set_volume(value as f32);
        volume_icon.set_icon_name(Some(volume_icon_name(value)));
    });

    let volume_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    volume_row.append(&low_volume);
    volume_row.append(&volume);

    card.append(&top);
    card.append(&tempo_area);
    card.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    card.append(&volume_row);
    card
}

/// Picks the loudspeaker icon for the current volume, including the
/// over-amplified glyph once the slider passes unity gain.
fn volume_icon_name(value: f64) -> &'static str {
    if value <= 0.0 {
        "audio-volume-muted-symbolic"
    } else if value <= 0.5 {
        "audio-volume-low-symbolic"
    } else if value <= 1.0 {
        "audio-volume-high-symbolic"
    } else {
        "audio-volume-overamplified-symbolic"
    }
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
        // The dot is drawn as a CSS circle instead of a font glyph: a text
        // bullet sits on the baseline inside its line box, so its optical
        // center is lower than the centered button background and the playing
        // highlight looked uneven. currentColor keeps the state colors.
        let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        dot.add_css_class("dot");
        dot.set_halign(gtk::Align::Center);
        dot.set_valign(gtk::Align::Center);

        let button = gtk::Button::new();
        button.set_child(Some(&dot));
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
    updates: &Rc<UpdateUi>,
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
        build_settings_modal(window, engine, params, updates);
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
    updates: &Rc<UpdateUi>,
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

    let sample_labels = SoundBank::candidate_labels();
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

    let update_button = gtk::Button::builder()
        .label(settings_text(
            params.language(),
            SettingsText::CheckForUpdates,
        ))
        .css_classes(["flat", "caption", "about-button"])
        .halign(gtk::Align::Center)
        .margin_top(12)
        .build();
    content.append(&update_button);

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

    // Manual "Check for updates": each click gets a fresh task and result
    // drain so repeated checks keep working. The button stays disabled until
    // the terminal event arrives.
    let updates_for_check = updates.clone();
    let toast_for_check = toast.clone();
    let button_for_check = update_button.clone();
    update_button.connect_clicked(move |_| {
        run_manual_update_check(&updates_for_check, &toast_for_check, &button_for_check);
    });

    // Update the modal in place instead of rebuilding it, so the switch state
    // and any in-progress preview stay intact while the language changes.
    let dialog_for_language = dialog.clone();
    let header_title_for_language = header_title.clone();
    let language_title_for_language = language_title.clone();
    let sample_title_for_language = sample_title.clone();
    let play_button_for_language = play_button.clone();
    let first_beat_for_language = first_beat.clone();
    let about_button_for_language = about_button.clone();
    let update_button_for_language = update_button.clone();
    let updates_for_language = updates.clone();
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
        update_button_for_language
            .set_label(settings_text(language, SettingsText::CheckForUpdates));
        updates_for_language.sync_language();
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
    let result = SoundBank::candidate(position, sample_rate);

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

/// Initializes GTK exactly once for every UI test, no matter which test
/// thread gets there first: parallel tests race gtk::init for the default
/// main context and fail when they call it concurrently.
#[cfg(all(test, feature = "ui"))]
fn ensure_gtk() {
    static GTK_INIT: std::sync::Once = std::sync::Once::new();
    GTK_INIT.call_once(|| {
        gtk::init().expect("GTK should initialize in the test harness");
    });
}

#[cfg(all(test, feature = "ui"))]
#[test]
fn ui_widget_checks() {
    // GTK tests must all run on the thread that initialized GTK, so every
    // widget check lives in this single test instead of parallel ones.
    ensure_gtk();
    run_icon_resource_checks();
    run_tempo_selector_checks();
}

#[cfg(all(test, feature = "ui"))]
fn run_icon_resource_checks() {
    register_icon_resources();

    let theme = gtk::IconTheme::new();
    theme.add_resource_path(crate::app::style::ICON_RESOURCE_PATH);

    for name in ["takto", "takto-symbolic", "takto-header"] {
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

#[cfg(all(test, feature = "ui"))]
fn run_tempo_selector_checks() {
    let params = Arc::new(SharedParams::new());
    let app = gtk::Application::new(Some("io.github.takto.UiTest"), Default::default());
    let window = adw::ApplicationWindow::new(&app);
    let overlay = gtk::Overlay::new();
    window.set_content(Some(&overlay));
    let _card = build_tempo_card(&window, &overlay, &params);
    overlay.set_child(Some(&_card));

    let selector = find_tempo_selector(overlay.upcast_ref())
        .expect("the window overlay should contain the selector list");

    fn find_list(widget: &gtk::Widget) -> Option<gtk::ListBox> {
        if let Some(list) = widget.downcast_ref::<gtk::ListBox>() {
            return Some(list.clone());
        }
        let mut child = widget.first_child();
        while let Some(current) = child {
            if let Some(found) = find_list(&current) {
                return Some(found);
            }
            child = current.next_sibling();
        }
        None
    }

    let list = find_list(selector.upcast_ref()).expect("the selector should contain the list");
    let mut rows = 0usize;
    let mut child = list.first_child();
    while let Some(current) = child {
        if current.downcast_ref::<gtk::ListBoxRow>().is_some() {
            rows += 1;
        }
        child = current.next_sibling();
    }
    assert_eq!(
        rows,
        TEMPO_MARKINGS.len(),
        "the selector should list every tempo marking"
    );
    assert!(
        !selector.is_visible(),
        "the selector starts hidden inside the overlay"
    );
    assert!(
        find_tempo_button(overlay.upcast_ref()).is_some(),
        "the tempo marking button should exist"
    );
}
