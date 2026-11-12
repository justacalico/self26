import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:self26/app.dart';
import 'package:self26/models.dart';
import 'package:self26/pages/admin_page.dart';
import 'package:self26/pages/comments_page.dart';
import 'package:self26/pages/favorites_page.dart';
import 'package:self26/pages/pools_page.dart';
import 'package:self26/pages/post_page.dart';
import 'package:self26/pages/posts_page.dart';
import 'package:self26/pages/settings_page.dart';
import 'package:self26/pages/tags_page.dart';
import 'package:self26/pages/upload_page.dart';
import 'package:self26/state.dart';

import 'helpers.dart';

void main() {
  test('buildTheme variants', () {
    final light = buildTheme(Brightness.light);
    expect(light.colorScheme.brightness, Brightness.light);

    final dark = buildTheme(Brightness.dark);
    expect(dark.colorScheme.brightness, Brightness.dark);
    expect(dark.colorScheme.surface, isNot(Colors.black));

    final oled = buildTheme(Brightness.dark, oled: true);
    expect(oled.colorScheme.surface, Colors.black);
    expect(oled.colorScheme.surfaceContainerLow, const Color(0xFF0A0A0A));

    final oledLight = buildTheme(Brightness.light, oled: true);
    expect(oledLight.colorScheme.surface, isNot(Colors.black));
  });

  testWidgets('home switches pages for every nav destination',
      (tester) async {
    final state = await pumpApp(tester, user: adminUser);
    await tester.pumpAndSettle();

    final pages = <NavDest, Type>{
      NavDest.posts: PostsPage,
      NavDest.favorites: FavoritesPage,
      NavDest.pools: PoolsPage,
      NavDest.tags: TagsPage,
      NavDest.comments: CommentsPage,
      NavDest.upload: UploadPage,
      NavDest.admin: AdminPage,
      NavDest.settings: SettingsPage,
    };
    for (final entry in pages.entries) {
      state.select(entry.key);
      await tester.pumpAndSettle();
      expect(find.byType(entry.value), findsOneWidget,
          reason: entry.key.name);
    }
  });

  testWidgets('selectedPost shows the detail page', (tester) async {
    final state = await pumpApp(tester);
    await tester.pumpAndSettle();

    state.openPost(Post.fromV2(postJson(1)));
    await tester.pumpAndSettle();
    expect(find.byType(PostPage), findsOneWidget);

    state.closeDetail();
    await tester.pumpAndSettle();
    expect(find.byType(PostsPage), findsOneWidget);
  });

  testWidgets('locale switching rebuilds the app', (tester) async {
    tester.platformDispatcher.localeTestValue = const Locale('en');
    addTearDown(tester.platformDispatcher.clearLocaleTestValue);
    final state = await pumpApp(tester);
    await tester.pumpAndSettle();
    expect(find.text('Posts'), findsWidgets);

    await state.setLocale(const Locale('ru'));
    await tester.pumpAndSettle();
    expect(find.text('Посты'), findsWidgets);

    await state.setLocale(
        const Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'));
    await tester.pumpAndSettle();
    expect(find.text('帖子'), findsWidgets);

    await state.setLocale(null);
    await tester.pumpAndSettle();
    expect(find.text('Posts'), findsWidgets);
  });

  testWidgets('theme mode drives MaterialApp', (tester) async {
    final state = await pumpApp(tester);
    await tester.pumpAndSettle();

    MaterialApp app() => tester.widget(find.byType(MaterialApp));

    expect(app().themeMode, ThemeMode.system);
    await state.setThemeMode(AppThemeMode.light);
    await tester.pump();
    expect(app().themeMode, ThemeMode.light);
    await state.setThemeMode(AppThemeMode.dark);
    await tester.pump();
    expect(app().themeMode, ThemeMode.dark);
    await state.setThemeMode(AppThemeMode.oled);
    await tester.pump();
    expect(app().themeMode, ThemeMode.dark);
    expect(app().darkTheme!.colorScheme.surface, Colors.black);
    await state.setThemeMode(AppThemeMode.system);
    await tester.pump();
  });
}
