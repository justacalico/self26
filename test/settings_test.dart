import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:self26/models.dart';
import 'package:self26/state.dart';
import 'package:self26/version_checker.dart';

import 'helpers.dart';

const releasesRoute = 'GET /api/v4/projects/HttpAnimations%2Fself26/releases';

void main() {
  Future<AppState> openSettings(
    WidgetTester tester, {
    FakeClient? client,
    VersionChecker? versionChecker,
    String appVersion = '1.0.0',
    void Function(Uri)? onOpen,
  }) async {
    final state = await pumpApp(
      tester,
      client: client,
      versionChecker: versionChecker,
      appVersion: appVersion,
      onOpen: onOpen,
    );
    state.select(NavDest.settings);
    await tester.pumpAndSettle();
    return state;
  }

  testWidgets('renders sections and the app version', (tester) async {
    await openSettings(tester);
    expect(find.text('Server URL'), findsOneWidget);
    expect(find.text('http://server.test'), findsOneWidget);
    expect(find.text('1.0.0'), findsOneWidget);
    expect(find.text('Traditional English'), findsOneWidget);
    expect(find.text('Русский'), findsOneWidget);
    expect(find.text('简体中文'), findsOneWidget);
    expect(find.text('Gaeilge'), findsOneWidget);
    expect(find.text('OLED'), findsOneWidget);
  });

  testWidgets('edits the server url through the dialog', (tester) async {
    final state = await openSettings(tester);
    await tester.tap(find.text('Server URL'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(state.api.baseUrl, 'http://server.test');

    await tester.tap(find.text('Server URL'));
    await tester.pumpAndSettle();
    await tester.enterText(
        find.byType(TextField), '  http://new.test  ');
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
    expect(state.api.baseUrl, 'http://new.test');
  });

  testWidgets('language radios update the locale', (tester) async {
    final state = await openSettings(tester);
    await tester.tap(
        find.widgetWithText(RadioListTile<Locale?>, 'Русский'));
    await tester.pumpAndSettle();
    expect(state.locale, const Locale('ru'));

    await tester.tap(find.widgetWithText(
        RadioListTile<Locale?>, 'Системный (System)'));
    await tester.pumpAndSettle();
    expect(state.locale, isNull);
  });

  testWidgets('theme radios update the theme', (tester) async {
    final state = await openSettings(tester);
    await tester.tap(
        find.widgetWithText(RadioListTile<AppThemeMode>, 'OLED'));
    await tester.pumpAndSettle();
    expect(state.themeMode, AppThemeMode.oled);

    await tester.tap(
        find.widgetWithText(RadioListTile<AppThemeMode>, 'Light'));
    await tester.pumpAndSettle();
    expect(state.themeMode, AppThemeMode.light);
  });

  testWidgets('update check shows the result and release link',
      (tester) async {
    final opened = <Uri>[];
    final state = await openSettings(
      tester,
      versionChecker: VersionChecker(
          client: FakeClient({
        releasesRoute: [
          {'tag_name': 'v9.9.9'},
        ],
      })),
      onOpen: opened.add,
    );
    await tester.scrollUntilVisible(find.text('Check for updates'), 250, scrollable: find.byType(Scrollable).last);
    await tester.tap(find.text('Check for updates'));
    await tester.pumpAndSettle();
    expect(find.text('Version 9.9.9 is available'), findsOneWidget);
    expect(state.appUpdate?.updateAvailable, isTrue);

    await tester.scrollUntilVisible(find.text('View release'), 250, scrollable: find.byType(Scrollable).last);
    await tester.tap(find.text('View release'));
    await tester.pumpAndSettle();
    expect(opened.single.toString(),
        'https://gitlab.com/HttpAnimations/self26/-/releases/v9.9.9');
  });

  testWidgets('failed update check and status variants', (tester) async {
    final state = await openSettings(
      tester,
      versionChecker:
          VersionChecker(client: FakeClient({releasesRoute: 500})),
    );
    await tester.scrollUntilVisible(find.text('Check for updates'), 250, scrollable: find.byType(Scrollable).last);
    await tester.tap(find.text('Check for updates'));
    await tester.pumpAndSettle();
    expect(find.text('Update check failed'), findsOneWidget);

    state.updateCheckFailed = false;
    state.appUpdate = const AppUpdate(latestVersion: '1.0.0');
    state.notifyListeners();
    await tester.pump();
    expect(find.text('self26 is up to date'), findsOneWidget);

    state.checkingUpdate = true;
    state.notifyListeners();
    await tester.pump();
    expect(find.text('Checking…'), findsOneWidget);
    state.checkingUpdate = false;
    state.notifyListeners();
    await tester.pump();
  });

  testWidgets('about links open through the injected opener',
      (tester) async {
    final opened = <Uri>[];
    await openSettings(tester, onOpen: opened.add);

    for (final label in [
      'All releases',
      'Source code',
      'Licence: AGPL-3.0',
    ]) {
      await tester.scrollUntilVisible(find.text(label), 250, scrollable: find.byType(Scrollable).last);
      await tester.tap(find.text(label));
      await tester.pump();
    }
    expect(opened.length, 3);
    expect(opened.first.toString(),
        'https://gitlab.com/HttpAnimations/self26/-/releases');
  });
}
