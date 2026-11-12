import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:self26/models.dart';
import 'package:self26/state.dart';
import 'package:self26/version_checker.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'helpers.dart';

const releasesRoute = 'GET /api/v4/projects/HttpAnimations%2Fself26/releases';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('restores defaults from empty prefs', () async {
    final state = await makeState();
    expect(state.locale, isNull);
    expect(state.themeMode, AppThemeMode.system);
    expect(state.api.baseUrl, 'http://server.test');
  });

  test('restores locale, theme, server url and credentials', () async {
    final state = await makeState(prefs: {
      'locale': 'ru',
      'themeMode': 2,
      'serverUrl': 'http://other.test',
      'loginName': 'alice',
      'apiKey': 'k',
    });
    expect(state.locale, const Locale('ru'));
    expect(state.themeMode, AppThemeMode.dark);
    expect(state.api.baseUrl, 'http://other.test');
    expect(state.api.loginName, 'alice');
    expect(state.api.apiKey, 'k');
  });

  test('restore keeps same server url and parses each locale tag', () async {
    for (final (tag, expected) in [
      ('en_GB', const Locale('en', 'GB')),
      ('zh_Hans',
          const Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans')),
      ('ru', const Locale('ru')),
      ('ga', const Locale('ga')),
      ('xx', null),
    ]) {
      final state = await makeState(
          baseUrl: 'http://kept.test',
          prefs: {'locale': tag, 'serverUrl': 'http://kept.test'});
      expect(state.locale, expected, reason: tag);
      expect(state.api.baseUrl, 'http://kept.test');
    }
  });

  test('themeMode index wraps around', () async {
    final state = await makeState(prefs: {'themeMode': 6});
    expect(state.themeMode, AppThemeMode.dark);
  });

  test('init checks health and restores the session', () async {
    final state = await makeState(prefs: {
      'loginName': 'alice',
      'apiKey': 'key123',
    });
    await state.init();
    expect(state.serverReachable, isTrue);
    expect(state.user?.name, 'alice');

    final offline = await makeState(
      client: FakeClient({'GET /health.json': 500}),
      prefs: {'loginName': 'alice', 'apiKey': 'key123'},
    );
    await offline.init();
    expect(offline.serverReachable, isFalse);

    final badCreds = await makeState(
      client: FakeClient({'GET /users/alice.json': 401}),
      prefs: {'loginName': 'alice', 'apiKey': 'bad'},
    );
    await badCreds.init();
    expect(badCreds.user, isNull);
  });

  test('setLocale persists and clears', () async {
    final state = await makeState();
    await state.setLocale(const Locale('ru'));
    expect(state.prefs.getString('locale'), 'ru');
    await state.setLocale(const Locale('en', 'GB'));
    expect(state.prefs.getString('locale'), 'en_GB');
    await state.setLocale(
        const Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'));
    expect(state.prefs.getString('locale'), 'zh_Hans');
    await state.setLocale(const Locale('ga'));
    expect(state.prefs.getString('locale'), 'ga');
    await state.setLocale(null);
    expect(state.prefs.getString('locale'), isNull);
    expect(state.locale, isNull);
  });

  test('setThemeMode persists', () async {
    final state = await makeState();
    await state.setThemeMode(AppThemeMode.oled);
    expect(state.prefs.getInt('themeMode'), AppThemeMode.oled.index);
    expect(state.themeMode, AppThemeMode.oled);
  });

  test('setServerUrl trims, falls back and resets the session', () async {
    final state = await makeState(user: memberUser);
    await state.setServerUrl('  http://new.test  ');
    expect(state.api.baseUrl, 'http://new.test');
    expect(state.prefs.getString('serverUrl'), 'http://new.test');
    expect(state.user, isNull);
    expect(state.serverReachable, isTrue);

    await state.setServerUrl('');
    expect(state.api.baseUrl, AppState.defaultBaseUrl());
  });

  test('signIn stores credentials and the user', () async {
    final state = await makeState();
    final user = await state.signIn('alice', 'pw');
    expect(user.name, 'alice');
    expect(state.user, user);
    expect(state.prefs.getString('loginName'), 'alice');
    expect(state.prefs.getString('apiKey'), 'key123');
  });

  test('signIn without an api key does not persist credentials', () async {
    final state = await makeState(
      client: FakeClient({
        'POST /session.json': {
          'user': {'id': 7, 'name': 'alice', 'level': 20}
        },
      }),
    );
    final user = await state.signIn('alice', 'pw');
    expect(user.apiKey, isNull);
    expect(state.prefs.getString('apiKey'), isNull);
  });

  test('signInWithKey persists credentials', () async {
    final state = await makeState();
    final user = await state.signInWithKey('alice', 'key123');
    expect(user.name, 'alice');
    expect(state.prefs.getString('loginName'), 'alice');
    expect(state.prefs.getString('apiKey'), 'key123');
  });

  test('signOut clears state even when logout fails', () async {
    final state = await makeState(user: memberUser);
    state.api.loginName = 'alice';
    state.api.apiKey = 'k';
    await state.prefs.setString('loginName', 'alice');
    await state.prefs.setString('apiKey', 'k');
    await state.signOut();
    expect(state.user, isNull);
    expect(state.api.apiKey, isNull);
    expect(state.prefs.getString('apiKey'), isNull);

    final failing = await makeState(
      client: FakeClient({'DELETE /session.json': 500}),
      user: memberUser,
    );
    await failing.signOut();
    expect(failing.user, isNull);
  });

  test('navigation helpers update state', () async {
    final state = await makeState();
    final post = Post.fromV2(postJson(1));
    final pool = Pool.fromJson(poolJson(2));

    state.select(NavDest.tags);
    expect(state.nav, NavDest.tags);

    state.openPost(post);
    expect(state.selectedPost, post);
    expect(state.selectedPool, isNull);

    state.openPool(pool);
    expect(state.selectedPool, pool);
    expect(state.selectedPost, isNull);

    state.select(NavDest.pools);
    expect(state.selectedPool, isNull);

    state.openPost(post);
    state.closeDetail();
    expect(state.selectedPost, isNull);

    state.setSearch('fox');
    expect(state.searchTags, 'fox');
    expect(state.selectedPost, isNull);
  });

  test('checkForUpdate dedupes and stores the result', () async {
    final state = await makeState(
      versionChecker: VersionChecker(
          client: FakeClient({
        releasesRoute: [
          {'tag_name': 'v2.0.0'},
        ],
      })),
    );
    final first = state.checkForUpdate();
    final second = state.checkForUpdate();
    expect(identical(first, second), isTrue);
    expect(state.checkingUpdate, isTrue);
    final update = await first;
    expect(update?.updateAvailable, isTrue);
    expect(state.appUpdate?.latestVersion, '2.0.0');
    expect(state.checkingUpdate, isFalse);

    final third = state.checkForUpdate();
    expect(identical(first, third), isFalse);
    await third;
  });

  test('checkForUpdate failure flags the state', () async {
    final state = await makeState(
      versionChecker:
          VersionChecker(client: FakeClient({releasesRoute: 500})),
    );
    final update = await state.checkForUpdate();
    expect(update, isNull);
    expect(state.updateCheckFailed, isTrue);
    expect(state.appUpdate, isNull);
  });

  test('imageHeaders only sends auth when credentials exist', () async {
    final state = await makeState();
    expect(state.imageHeaders, isEmpty);

    final authed = await makeState(prefs: {
      'loginName': 'alice',
      'apiKey': 'k',
    });
    expect(authed.imageHeaders['Authorization'], startsWith('Basic '));
  });

  test('materialThemeMode maps each app mode', () async {
    final state = await makeState();
    expect(state.materialThemeMode, ThemeMode.system);
    await state.setThemeMode(AppThemeMode.light);
    expect(state.materialThemeMode, ThemeMode.light);
    await state.setThemeMode(AppThemeMode.dark);
    expect(state.materialThemeMode, ThemeMode.dark);
    await state.setThemeMode(AppThemeMode.oled);
    expect(state.materialThemeMode, ThemeMode.dark);
  });

  test('defaultBaseUrl and defaults factory', () async {
    expect(AppState.defaultBaseUrl(), 'http://localhost:8021');
    SharedPreferences.setMockInitialValues({});
    final prefs = await SharedPreferences.getInstance();
    final state = AppState.defaults(prefs);
    expect(state.api.baseUrl, 'http://localhost:8021');
  });

  test('default appVersion and openUrl hit platform plugins', () async {
    final state = await makeState(injectDefaults: false);
    await expectLater(state.appVersion(), throwsA(anything));
    await expectLater(state.openUrl(Uri.parse('https://x.test')),
        throwsA(anything));
  });
}
