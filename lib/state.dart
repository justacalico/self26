import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:url_launcher/url_launcher.dart' as launcher;

import 'api.dart';
import 'models.dart';
import 'version_checker.dart';

/// Sidebar destinations, in display order.
enum NavDest { posts, favorites, pools, tags, comments, upload, admin, settings }

enum AppThemeMode { system, light, dark, oled }

/// Locales the UI ships, in selector order. `null` means "follow the system".
const appLocales = <Locale?>[
  null,
  Locale('en', 'GB'),
  Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'),
  Locale('ru'),
  Locale('ga'),
];

final appLocaleNames = <Locale?, String>{
  null: 'System',
  Locale('en', 'GB'): 'Traditional English',
  Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'): '简体中文',
  Locale('ru'): 'Русский',
  Locale('ga'): 'Gaeilge',
};

class AppState extends ChangeNotifier {
  Self26Api api;
  final SharedPreferences prefs;
  final VersionChecker versionChecker;
  final Future<String> Function() appVersion;
  final Future<void> Function(Uri url) openUrl;

  User? user;
  Locale? locale;
  AppThemeMode themeMode = AppThemeMode.system;
  AppUpdate? appUpdate;
  bool checkingUpdate = false;
  bool updateCheckFailed = false;
  NavDest nav = NavDest.posts;
  Post? selectedPost;
  Pool? selectedPool;
  String searchTags = '';
  bool serverReachable = true;

  Future<AppUpdate?>? _updateCheck;

  AppState({
    required this.api,
    required this.prefs,
    VersionChecker? versionChecker,
    Future<String> Function()? appVersion,
    Future<void> Function(Uri url)? openUrl,
    this.user,
  })  : versionChecker = versionChecker ?? VersionChecker(),
        appVersion = appVersion ?? (() async => (await PackageInfo.fromPlatform()).version),
        openUrl = openUrl ?? ((url) => launcher.launchUrl(url).then((_) {})) {
    _restore();
  }

  factory AppState.defaults(SharedPreferences prefs, {String? baseUrl}) =>
      AppState(api: Self26Api(baseUrl ?? defaultBaseUrl()), prefs: prefs);

  /// Where the app talks when no server is configured. On web the app is
  /// served by the backend itself, so same-origin is right.
  static String defaultBaseUrl() {
    if (kIsWeb) return Uri.base.origin;
    return 'http://localhost:8021';
  }

  void _restore() {
    locale = _localeFromTag(prefs.getString('locale'));
    themeMode = AppThemeMode
        .values[(prefs.getInt('themeMode') ?? 0) % AppThemeMode.values.length];
    final url = prefs.getString('serverUrl');
    final name = prefs.getString('loginName');
    final key = prefs.getString('apiKey');
    if (url != null && url.isNotEmpty && url != api.baseUrl) {
      api = Self26Api(url, client: api.client);
    }
    if (name != null && key != null) {
      api.loginName = name;
      api.apiKey = key;
    }
    notifyListeners();
  }

  Future<void> init() async {
    try {
      await api.health();
      serverReachable = true;
    } catch (_) {
      serverReachable = false;
    }
    if (api.apiKey != null && api.loginName != null) {
      try {
        user = await api.loginWithKey(api.loginName!, api.apiKey!);
      } catch (_) {
        user = null;
      }
    }
    notifyListeners();
  }

  Locale? _localeFromTag(String? tag) {
    switch (tag) {
      case 'en_GB':
        return const Locale('en', 'GB');
      case 'zh_Hans':
        return const Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans');
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

  Future<void> setThemeMode(AppThemeMode mode) async {
    themeMode = mode;
    await prefs.setInt('themeMode', mode.index);
    notifyListeners();
  }

  Future<void> setServerUrl(String url) async {
    final clean = url.trim();
    api = Self26Api(clean.isEmpty ? defaultBaseUrl() : clean);
    await prefs.setString('serverUrl', api.baseUrl);
    serverReachable = true;
    user = null;
    notifyListeners();
  }

  Future<User> signIn(String name, String password) async {
    final u = await api.login(name, password);
    user = u;
    if (u.apiKey != null) {
      await prefs.setString('loginName', name);
      await prefs.setString('apiKey', u.apiKey!);
    }
    notifyListeners();
    return u;
  }

  Future<User> signInWithKey(String name, String key) async {
    final u = await api.loginWithKey(name, key);
    user = u;
    await prefs.setString('loginName', name);
    await prefs.setString('apiKey', key);
    notifyListeners();
    return u;
  }

  Future<void> signOut() async {
    try {
      await api.logout();
    } catch (_) {
      // Local sign-out still proceeds when the server is unreachable.
    }
    user = null;
    api.apiKey = null;
    api.loginName = null;
    await prefs.remove('loginName');
    await prefs.remove('apiKey');
    notifyListeners();
  }

  void select(NavDest dest) {
    nav = dest;
    selectedPost = null;
    selectedPool = null;
    notifyListeners();
  }

  void openPost(Post post) {
    selectedPost = post;
    selectedPool = null;
    notifyListeners();
  }

  void openPool(Pool pool) {
    selectedPool = pool;
    selectedPost = null;
    notifyListeners();
  }

  void closeDetail() {
    selectedPost = null;
    selectedPool = null;
    notifyListeners();
  }

  void setSearch(String tags) {
    searchTags = tags;
    selectedPost = null;
    notifyListeners();
  }

  /// De-dupes concurrent checks; returns null when the check fails so the
  /// UI can show "check failed" feedback.
  Future<AppUpdate?> checkForUpdate() {
    final existing = _updateCheck;
    if (existing != null) return existing;
    checkingUpdate = true;
    updateCheckFailed = false;
    notifyListeners();
    return _updateCheck = _doCheck().whenComplete(() {
      _updateCheck = null;
      checkingUpdate = false;
      notifyListeners();
    });
  }

  Future<AppUpdate?> _doCheck() async {
    try {
      final update = await versionChecker.check(await appVersion());
      appUpdate = update;
      return update;
    } catch (_) {
      updateCheckFailed = true;
      return null;
    }
  }

  /// Basic-auth headers for <img> requests on platforms without cookies.
  Map<String, String> get imageHeaders {
    if (api.apiKey == null || api.loginName == null) return const {};
    final creds =
        base64.encode(utf8.encode('${api.loginName}:${api.apiKey}'));
    return {'Authorization': 'Basic $creds'};
  }

  ThemeMode get materialThemeMode => switch (themeMode) {
        AppThemeMode.system => ThemeMode.system,
        AppThemeMode.light => ThemeMode.light,
        AppThemeMode.dark || AppThemeMode.oled => ThemeMode.dark,
      };
}
