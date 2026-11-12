import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:self26/pages/post_page.dart';
import 'package:self26/widgets.dart' as w;

import 'helpers.dart';

void main() {
  testWidgets('grid, search and quick filters', (tester) async {
    final state = await pumpApp(tester);
    await tester.pumpAndSettle();

    expect(find.byType(w.PostTile), findsNWidgets(2));

    await tester.enterText(find.byType(TextField), 'fox');
    await tester.testTextInput.receiveAction(TextInputAction.search);
    await tester.pumpAndSettle();
    expect(state.searchTags, 'fox');
    expect(find.byType(w.PostTile), findsNWidgets(2));

    for (final (label, query) in [
      ('Random', 'order:random'),
      ('Popular', 'order:score'),
      ('Hot', 'order:rank'),
    ]) {
      await tester.tap(find.widgetWithText(ActionChip, label));
      await tester.pumpAndSettle();
      expect(state.searchTags, query);
    }
  });

  testWidgets('clear button resets the search', (tester) async {
    final state = await pumpApp(tester);
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField), 'fox');
    await tester.tap(find.byIcon(Icons.clear));
    await tester.pumpAndSettle();
    expect(state.searchTags, '');
  });

  testWidgets('empty list shows the empty state', (tester) async {
    await pumpApp(tester,
        client: FakeClient({
          'GET /posts.json': {'posts': <Object?>[]},
        }));
    await tester.pumpAndSettle();
    expect(find.text('No posts yet'), findsOneWidget);
    expect(find.text('Upload something to get started.'), findsOneWidget);
  });

  testWidgets('load error shows retry which reloads', (tester) async {
    var calls = 0;
    await pumpApp(tester,
        client: FakeClient({
          'GET /posts.json': (request) =>
              calls++ == 0 ? 500 : {'posts': [postJson(1)]},
        }));
    await tester.pumpAndSettle();
    expect(find.text('Could not load Posts'), findsOneWidget);

    await tester.tap(find.text('Retry'));
    await tester.pumpAndSettle();
    expect(find.byType(w.PostTile), findsOneWidget);
    expect(calls, 2);
  });

  testWidgets('tapping a tile opens the detail page', (tester) async {
    final state = await pumpApp(tester);
    await tester.pumpAndSettle();

    await tester.tap(find.byType(w.PostTile).first);
    await tester.pumpAndSettle();
    expect(state.selectedPost?.id, 1);
    expect(find.byType(PostPage), findsOneWidget);
  });
}
