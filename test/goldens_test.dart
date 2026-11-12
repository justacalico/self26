import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:self26/models.dart';
import 'package:self26/state.dart';

import 'helpers.dart';

void main() {
  setUpAll(loadNotoSans);

  Future<void> shot(WidgetTester tester, String name) async {
    await tester.pumpAndSettle();
    await expectLater(
        find.byType(MaterialApp), matchesGoldenFile('goldens/$name.png'));
  }

  testWidgets('shell with sidebar and posts grid', (tester) async {
    installFakeWindowActions();
    await pumpApp(tester, desktop: true);
    await shot(tester, 'shell_posts');
  });

  testWidgets('post detail', (tester) async {
    installFakeWindowActions();
    final state = await pumpApp(tester, desktop: true);
    await tester.pumpAndSettle();
    state.openPost(Post.fromV2(postJson(1)));
    await shot(tester, 'post_detail');
  });

  testWidgets('pools', (tester) async {
    installFakeWindowActions();
    final state = await pumpApp(tester, user: memberUser);
    state.select(NavDest.pools);
    await shot(tester, 'pools');
  });

  testWidgets('tags', (tester) async {
    installFakeWindowActions();
    final state = await pumpApp(tester);
    state.select(NavDest.tags);
    await shot(tester, 'tags');
  });

  testWidgets('comments', (tester) async {
    installFakeWindowActions();
    final state = await pumpApp(tester);
    state.select(NavDest.comments);
    await shot(tester, 'comments');
  });

  testWidgets('upload', (tester) async {
    installFakeWindowActions();
    final state = await pumpApp(tester, user: memberUser);
    state.select(NavDest.upload);
    await shot(tester, 'upload');
  });

  testWidgets('settings', (tester) async {
    installFakeWindowActions();
    final state = await pumpApp(tester);
    state.select(NavDest.settings);
    await shot(tester, 'settings');
  });

  testWidgets('login', (tester) async {
    installFakeWindowActions();
    await pumpApp(tester);
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.login));
    await shot(tester, 'login');
  });

  testWidgets('narrow drawer', (tester) async {
    installFakeWindowActions();
    await pumpApp(tester, size: const Size(500, 800));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.menu));
    await shot(tester, 'narrow_drawer');
  });

  testWidgets('dark theme', (tester) async {
    installFakeWindowActions();
    final state = await pumpApp(tester, desktop: true);
    await state.setThemeMode(AppThemeMode.dark);
    await shot(tester, 'dark_shell');
  });
}
