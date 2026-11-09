# План публикации Metrognome на Flathub

## Цель

Сделать `io.github.metrognome.Metrognome` доступным на Flathub:

```
flatpak install flathub io.github.metrognome.Metrognome
```

Приложение появится в каталогах GNOME Software и KDE Discover.

## Текущее состояние

Инфраструктура Flatpak уже реализована (см. `packaging/flatpak/`, `DEVELOP.md`):

| Артефакт | Статус |
|---|---|
| `packaging/flatpak/io.github.metrognome.Metrognome.yml` — манифест (runtime `org.gnome.Platform 50`, vendored cargo-крейсы, офлайн-сборка) | готов |
| `packaging/flatpak/cargo-sources.json` (генерируется `scripts/generate-cargo-sources.py`) | готов |
| `packaging/io.github.metrognome.Metrognome.metainfo.xml` — AppStream (валидация `appstreamcli validate --pedantic` проходит) | готов |
| CI-сборка бандла на тег → `.flatpak` на GitHub Releases | готов |
| Локальная ручная установка бандла (`flatpak install --user ./metrognome_X_Y_Z_x86_64.flatpak`) | работает уже сейчас |

Публикация на Flathub — **не обязательный** шаг для установки, но даёт
каталоги центров приложений и автоматические обновления.

## Подготовительные изменения в репозитории

### 1. Манифест для Flathub: источник кода

Flathub не разрешает `type: dir` (локальные каталоги). Для отправки нужен
вариант манифеста с источником `type: git`:

```yaml
sources:
  - type: git
    url: https://github.com/ez404-36/metrognome.git
    tag: vX.Y.Z          # или commit: <sha>
    x-checker-data:
      type: git
      tag-pattern: ^v([\d.]+)$
```

Остальное (runtime, finish-args, cargo-sources, сборка) не меняется.
Удобно держать два манифеста:

- `io.github.metrognome.Metrognome.yml` — локальная сборка (dir, как сейчас);
- `io.github.metrognome.Metrognome.flathub.yml` — вариант для Flathub (git).

### 2. Иконки по app ID (рекомендация ревью Flathub)

Сейчас иконки ставятся под именем `metrognome*`, а конвенция Flatpak/Flathub —
имена по app ID (`io.github.metrognome.Metrognome`). Работать будет, но ревью
могут попросить переименовать.

Кросс-обрезное изменение, затрагивает:

- `src/app/window.rs` — `.application_icon("metrognome")`, `.icon_name("metrognome-header")`;
- desktop-файл `Icon=`;
- gresource-алиасы (`build.rs`);
- пути установки иконок в манифесте, deb и AppImage-пакетинге;
- ссылки на иконку в README.

Ветвь выбора: переименовать до подачи заявки (чтобы ревью прошли сразу)
или подать как есть и переименовать по замечанию.

### 3. Проверка манифеста линтером

```
flatpak run --command=flatpak-builder-lint org.flatpak.Builder \
    manifest packaging/flatpak/io.github.metrognome.Metrognome.flathub.yml
```

Устранить все ошибки до подачи PR.

## Подача заявки на Flathub (актуальный процесс)

1. **Fork** `flathub/flathub` (полный форк, не «copy master only»).
2. **Ветка** от `new-pr` (не от `master`).
3. Добавить манифест и, если нужно, дополнительные патчи/источники
   `flathub/io.github.metrognome.Metrognome.*`.
4. **PR против ветки `new-pr`** с заголовком
   `Add io.github.metrognome.Metrognome`.
5. Запросить тест-сборку комментарием **`bot, build`**; дождаться успешного
   билда бота.
6. Ревью волонтёрами. Возможные замечания — правки в PR. Причёсывать
   иконки/метаданные просят часто, это нормальная часть процесса.
7. После аппрува: создаётся репо `flathub/io.github.metrognome.Metrognome`
   и приходит инвайт (нужен включённый **2FA** на GitHub).
8. Обновления после публикации — обычные PR в этот репо
   (bump тега/коммита в манифесте, регенерация cargo-sources).

## Известные риски

| Риск | Вероятность | Митигирование |
|---|---|---|
| Отказ по «insufficient development history» — репо создано недавно, релизов мало | заметная; Flathub иногда отклоняет молодые проекты | отложить заявку, набрать историю релизов/коммитов, либо подать и честно аргументировать |
| Замечание ревью к именам иконок | высокая (конвенция app ID) | сделать переименование заранее — см. шаг 2 |
| Verified-бейдж не светится: app ID `io.github.metrognome.Metrognome` не матчится с GitHub-логином `ez404-36` | не блокер ревью | создать GitHub-организацию `metrognome`, тогда бейдж доступен |
| EOL-рантайм на момент ревью | низкая | runtime `org.gnome.Platform 50` поддерживается до выхода GNOME 52 (~март 2027); перед подачей проверить актуальность |

## Сопровождение после публикации

- **Cargo.lock изменился** → регенерировать
  `packaging/flatpak/cargo-sources.json` (`scripts/generate-cargo-sources.py`)
  и отправить PR во Flathub-репо.
- **Новый релиз** → bump `tag:` в манифесте, PR во Flathub-репо
  (или обновление в том же PR, если билдится автоматически через
  `x-checker-data` + flathub-бот).
- **Выход GNOME 52** → bump runtime в манифесте, PR в свой и Flathub-репо.
  Мёртвый рантайм на Flathub запрещён — за этим следят ревью, но лучше
  обновляться самому.

## Чек-лист подачи

- [ ] Решено: переименовывать иконки под app ID до подачи или нет
- [ ] Создан `io.github.metrognome.Metrognome.flathub.yml` (источник `type: git`, pin на тег)
- [ ] `flatpak-builder-lint manifest` — без ошибок
- [ ] Манифест и metainfo проверены локальной сборкой
- [ ] Fork `flathub/flathub`, ветка от `new-pr`
- [ ] PR «Add io.github.metrognome.Metrognome» против `new-pr`
- [ ] `bot, build` — сборка бота успешна
- [ ] Ревью пройдено, инвайт принят (2FA включён)
- [ ] Первый релиз опубликован, приложение видно в GNOME Software / Discover
