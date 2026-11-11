// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Russian (`ru`).
class AppLocalizationsRu extends AppLocalizations {
  AppLocalizationsRu([String locale = 'ru']) : super(locale);

  @override
  String get appTitle => 'self26';

  @override
  String get openApp => 'Открыть приложение';

  @override
  String get viewOnGitLab => 'Открыть на GitLab';

  @override
  String get pitch => 'Ваш собственный медиасервер, совместимый с e621.';

  @override
  String get heroSub => 'Собственный booru-хостинг для вашей коллекции.';

  @override
  String get themeLabel => 'Тема';

  @override
  String get themeSystem => 'Системная';

  @override
  String get themeLight => 'Светлая';

  @override
  String get themeDark => 'Тёмная';

  @override
  String get themeOled => 'OLED';

  @override
  String get languageLabel => 'Язык';

  @override
  String get langSystem => 'Системный';

  @override
  String get showcaseTitle => 'Как это выглядит';

  @override
  String get showcaseGridCaption => 'Просмотр по тегам, как на e621.';

  @override
  String get showcaseApiCaption => 'Поддержка JSON API e621 v2.';

  @override
  String get showcaseShellCaption =>
      'Одно приложение для веба, десктопа и мобильных.';

  @override
  String get featuresTitle => 'Возможности';

  @override
  String get featureApi => 'API, совместимый с e621 v2';

  @override
  String get featureSelfHost => 'Self-hosted: один бинарник на Rust и SQLite';

  @override
  String get featurePlatforms => 'Веб, десктоп и мобильные устройства';

  @override
  String get featureBooru => 'Пулы, теги, комментарии и избранное';

  @override
  String get downloadsTitle => 'Загрузки';

  @override
  String get downloadsEmpty => 'Релизов пока нет.';

  @override
  String get downloadsError => 'Не удалось загрузить релизы.';

  @override
  String get retry => 'Повторить';

  @override
  String get allReleases => 'Все релизы на GitLab';

  @override
  String get flatpakHint => 'На Linux? Установите Flatpak.';

  @override
  String get footerLicence => 'self26 - свободное ПО под лицензией AGPL-3.0.';

  @override
  String get sourceCode => 'Исходный код';
}
