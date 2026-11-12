import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:self26/models.dart';
import 'package:self26/pages/post_page.dart';
import 'package:self26/state.dart';
import 'package:self26/widgets.dart' as w;

import 'helpers.dart';

void main() {
  testWidgets('favorites grid and empty state', (tester) async {
    final state = await pumpApp(tester);
    state.select(NavDest.favorites);
    await tester.pumpAndSettle();
    expect(find.byType(w.PostTile), findsOneWidget);

    final empty = await pumpApp(tester,
        client: FakeClient({'GET /favorites.json': {'posts': <Object?>[]}}));
    empty.select(NavDest.favorites);
    await tester.pumpAndSettle();
    expect(find.text('No favourites yet'), findsOneWidget);
  });

  testWidgets('favorites load error names favourites', (tester) async {
    final state = await pumpApp(tester,
        client: FakeClient({'GET /favorites.json': 500}));
    state.select(NavDest.favorites);
    await tester.pumpAndSettle();
    expect(find.text('Could not load Favourites'), findsOneWidget);
  });

  testWidgets('pools list, detail and back', (tester) async {
    final state = await pumpApp(tester);
    state.select(NavDest.pools);
    await tester.pumpAndSettle();

    expect(find.text('cool pool'), findsOneWidget);
    expect(find.text('a series'), findsOneWidget);
    expect(find.text('3 posts'), findsOneWidget);
    expect(find.text('empty pool'), findsOneWidget);
    expect(find.text('New pool'), findsNothing);

    await tester.tap(find.text('cool pool'));
    await tester.pumpAndSettle();
    expect(state.selectedPool?.id, 2);
    expect(find.byType(w.PostTile), findsOneWidget);

    await tester.tap(find.byType(BackButton));
    await tester.pumpAndSettle();
    expect(state.selectedPool, isNull);
    expect(find.text('cool pool'), findsOneWidget);
  });

  testWidgets('pool detail empty state and empty pools list',
      (tester) async {
    final state = await pumpApp(tester,
        client: FakeClient({'GET /pools.json': <Object?>[]}));
    state.select(NavDest.pools);
    await tester.pumpAndSettle();
    expect(find.text('No pools yet'), findsOneWidget);

    state.openPool(Pool.fromJson(barePoolJson(3)));
    await tester.pumpAndSettle();
    expect(find.text('No posts yet'), findsOneWidget);
  });

  testWidgets('pools load error names pools', (tester) async {
    final state = await pumpApp(tester,
        client: FakeClient({'GET /pools.json': 500}));
    state.select(NavDest.pools);
    await tester.pumpAndSettle();
    expect(find.text('Could not load Pools'), findsOneWidget);
  });

  testWidgets('create pool dialog', (tester) async {
    final state = await pumpApp(tester, user: memberUser);
    state.select(NavDest.pools);
    await tester.pumpAndSettle();
    expect(find.text('New pool'), findsOneWidget);

    await tester.tap(find.text('New pool'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(state.selectedPool, isNull);

    await tester.tap(find.text('New pool'));
    await tester.pumpAndSettle();
    final fields = find.byType(TextField);
    await tester.enterText(fields.first, 'fresh pool');
    await tester.enterText(fields.last, 'desc');
    await tester.tap(find.text('Create pool'));
    await tester.pumpAndSettle();
    expect(state.selectedPool?.id, 2);
  });

  testWidgets('failed pool creation leaves the list', (tester) async {
    final state = await pumpApp(tester,
        user: memberUser,
        client: FakeClient({'POST /pools.json': 500}));
    state.select(NavDest.pools);
    await tester.pumpAndSettle();
    await tester.tap(find.text('New pool'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Create pool'));
    await tester.pumpAndSettle();
    expect(state.selectedPool, isNull);
    expect(find.text('cool pool'), findsOneWidget);
  });

  testWidgets('tags list, search and tap to search', (tester) async {
    final state = await pumpApp(tester);
    state.select(NavDest.tags);
    await tester.pumpAndSettle();

    expect(find.text('fox'), findsOneWidget);
    expect(find.text('movie'), findsOneWidget);
    expect(find.text('artist'), findsOneWidget);
    expect(find.text('copyright'), findsOneWidget);
    expect(find.text('42'), findsWidgets);

    await tester.enterText(find.byType(TextField), 'wolf');
    await tester.testTextInput.receiveAction(TextInputAction.search);
    await tester.pumpAndSettle();

    await tester.tap(find.text('fox'));
    await tester.pumpAndSettle();
    expect(state.searchTags, 'fox');
    expect(state.nav, NavDest.posts);
  });

  testWidgets('empty tags list', (tester) async {
    final state = await pumpApp(tester,
        client: FakeClient({'GET /tags.json': <Object?>[]}));
    state.select(NavDest.tags);
    await tester.pumpAndSettle();
    expect(find.text('Nothing here yet'), findsOneWidget);
  });

  testWidgets('tags load error names tags', (tester) async {
    final state = await pumpApp(tester,
        client: FakeClient({'GET /tags.json': 500}));
    state.select(NavDest.tags);
    await tester.pumpAndSettle();
    expect(find.text('Could not load Tags'), findsOneWidget);
  });

  testWidgets('comments list opens the post', (tester) async {
    final state = await pumpApp(tester);
    state.select(NavDest.comments);
    await tester.pumpAndSettle();

    expect(find.text('nice post'), findsOneWidget);
    expect(find.text('#1 · alice · 2'), findsOneWidget);

    await tester.tap(find.text('nice post'));
    await tester.pumpAndSettle();
    expect(state.selectedPost?.id, 1);
    expect(find.byType(PostPage), findsOneWidget);
  });

  testWidgets('empty comments list', (tester) async {
    final state = await pumpApp(tester,
        client: FakeClient({'GET /comments.json': <Object?>[]}));
    state.select(NavDest.comments);
    await tester.pumpAndSettle();
    expect(find.text('No comments yet'), findsOneWidget);
  });

  testWidgets('comments load error names comments', (tester) async {
    final state = await pumpApp(tester,
        client: FakeClient({'GET /comments.json': 500}));
    state.select(NavDest.comments);
    await tester.pumpAndSettle();
    expect(find.text('Could not load Comments'), findsOneWidget);
  });
}
