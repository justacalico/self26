import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

enum LandingThemeMode { system, light, dark, oled }

const landingLocales = <Locale?>[
  null,
  Locale('en', 'GB'),
  Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'),
  Locale('ru'),
  Locale('ga'),
];

final landingLocaleNames = <Locale?, String>{
  Locale('en', 'GB'): 'Traditional English',
  Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'): '简体中文',
  Locale('ru'): 'Русский',
  Locale('ga'): 'Gaeilge',
};

class LandingState extends ChangeNotifier {
  LandingState(this.prefs) {
    themeMode = LandingThemeMode.values[
        (prefs.getInt('themeMode') ?? 0) % LandingThemeMode.values.length];
    locale = _localeFromTag(prefs.getString('locale'));
  }

  final SharedPreferences prefs;

  LandingThemeMode themeMode = LandingThemeMode.system;
  Locale? locale;

  ThemeMode get materialThemeMode => switch (themeMode) {
        LandingThemeMode.system => ThemeMode.system,
        LandingThemeMode.light => ThemeMode.light,
        LandingThemeMode.dark || LandingThemeMode.oled => ThemeMode.dark,
      };

  Locale? _localeFromTag(String? tag) {
    switch (tag) {
      case 'en_GB':
        return const Locale('en', 'GB');
      case 'zh_Hans':
        return const Locale.fromSubtags(
            languageCode: 'zh', scriptCode: 'Hans');
      case 'ru':
        return const Locale('ru');
      case 'ga':
        return const Locale('ga');
      default:
        return null;
    }
  }

  String? _localeTag(Locale? l) {
    if (l == null) return null;
    if (l.languageCode == 'zh') return 'zh_Hans';
    if (l.countryCode != null) return '${l.languageCode}_${l.countryCode}';
    return l.languageCode;
  }

  Future<void> setLocale(Locale? l) async {
    locale = l;
    final tag = _localeTag(l);
    if (tag == null) {
      await prefs.remove('locale');
    } else {
      await prefs.setString('locale', tag);
    }
    notifyListeners();
  }

  Future<void> setThemeMode(LandingThemeMode mode) async {
    themeMode = mode;
    await prefs.setInt('themeMode', mode.index);
    notifyListeners();
  }
}
