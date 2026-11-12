import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:self26/l10n/app_localizations.dart';
import 'package:self26/models.dart' as m;
import 'package:self26/pages/post_page.dart';
import 'package:self26/pages/posts_page.dart';
import 'package:self26/state.dart';

import 'helpers.dart';

void main() {
  Future<(AppState, FakeClient)> openPost(
    WidgetTester tester, {
    bool video = false,
    bool favorited = false,
    int vote = 0,
    m.User? user,
    Map<String, Object?> routes = const {},
  }) async {
    final client = FakeClient(routes);
    final state = await pumpApp(tester, client: client, user: user);
    await tester.pumpAndSettle();
    state.openPost(m.Post.fromV2(
        postJson(1, video: video, favorited: favorited, vote: vote)));
    await tester.pumpAndSettle();
    return (state, client);
  }

  testWidgets('detail renders metadata, tags and comments', (tester) async {
    await openPost(tester);
    expect(find.text('#1'), findsOneWidget);
    expect(find.text('3'), findsOneWidget);
    expect(
        find.textContaining('5 / 2', findRichText: true), findsWidgets);
    expect(find.text('cat ears'), findsOneWidget);
    expect(find.text('bob'), findsOneWidget);
    expect(find.text('nice post'), findsOneWidget);
    expect(find.textContaining('a description', findRichText: true),
        findsWidgets);
    expect(
        find.textContaining('#9', findRichText: true), findsWidgets);
    expect(find.textContaining('#4, #5', findRichText: true),
        findsWidgets);
    expect(
        find.textContaining('https://example.com/1', findRichText: true),
        findsWidgets);
    expect(find.byIcon(Icons.favorite_border), findsOneWidget);
  });

  testWidgets('video post shows a play placeholder', (tester) async {
    await openPost(tester, video: true);
    expect(find.byIcon(Icons.play_circle_outline), findsWidgets);
  });

  testWidgets('favorite and unfavorite', (tester) async {
    final (_, client) = await openPost(tester);
    await tester.tap(find.byIcon(Icons.favorite_border));
    await tester.pumpAndSettle();
    expect(
        client.requests
            .map((r) => '${r.method} ${r.url.path}')
            .contains('POST /favorites.json'),
        isTrue);

    final (_, client2) = await openPost(tester, favorited: true);
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.favorite));
    await tester.pumpAndSettle();
    expect(
        client2.requests
            .map((r) => '${r.method} ${r.url.path}')
            .contains('DELETE /favorites/1.json'),
        isTrue);
  });

  testWidgets('vote up and down toggle the score', (tester) async {
    http.Request voteRequest(FakeClient client) =>
        client.requests.whereType<http.Request>().firstWhere(
            (r) => r.url.path == '/posts/1/votes.json');

    final (_, client) = await openPost(tester);
    await tester.tap(find.byIcon(Icons.arrow_upward));
    await tester.pumpAndSettle();
    expect(voteRequest(client).bodyFields['score'], '1');

    final (_, client2) = await openPost(tester, vote: 1);
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.arrow_upward));
    await tester.pumpAndSettle();
    expect(voteRequest(client2).bodyFields['score'], '0');

    final (_, client3) = await openPost(tester, vote: -1);
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.arrow_downward));
    await tester.pumpAndSettle();
    expect(voteRequest(client3).bodyFields['score'], '0');

    final (_, client4) = await openPost(tester);
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.arrow_downward));
    await tester.pumpAndSettle();
    expect(voteRequest(client4).bodyFields['score'], '-1');
  });

  testWidgets('failed actions are swallowed', (tester) async {
    await openPost(tester, routes: {'POST /posts/1/votes.json': 500});
    await tester.tap(find.byIcon(Icons.arrow_upward));
    await tester.pumpAndSettle();
    expect(find.byType(PostPage), findsOneWidget);
  });

  testWidgets('delete dialog can cancel and confirm', (tester) async {
    final (state, client) = await openPost(tester);
    await tester.tap(find.byTooltip('Delete post'));
    await tester.pumpAndSettle();
    expect(find.text('Delete post'), findsWidgets);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(find.byType(PostPage), findsOneWidget);

    await tester.tap(find.byTooltip('Delete post'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Confirm'));
    await tester.pumpAndSettle();
    expect(state.selectedPost, isNull);
    expect(find.byType(PostsPage), findsOneWidget);
    expect(
        client.requests
            .map((r) => '${r.method} ${r.url.path}')
            .contains('DELETE /posts/1.json'),
        isTrue);
  });

  testWidgets('posting a comment refreshes the list', (tester) async {
    final (_, client) = await openPost(tester);
    await tester.ensureVisible(find.byType(TextField));
    await tester.enterText(find.byType(TextField), 'hello');
    await tester.tap(find.text('Post comment'));
    await tester.pumpAndSettle();
    expect(
        client.requests
            .map((r) => '${r.method} ${r.url.path}')
            .contains('POST /comments.json'),
        isTrue);
  });

  testWidgets('empty or failed comment submit keeps the draft',
      (tester) async {
    final (_, client) = await openPost(tester,
        routes: {'POST /comments.json': 500});
    await tester.ensureVisible(find.byType(TextField));
    await tester.tap(find.text('Post comment'));
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField), 'still here');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pumpAndSettle();
    expect(
        tester.widget<TextField>(find.byType(TextField)).controller?.text,
        'still here');
    expect(
        client.requests
            .map((r) => '${r.method} ${r.url.path}')
            .contains('POST /comments.json'),
        isTrue);
  });

  testWidgets('comment author can delete their comment', (tester) async {
    final (_, client) = await openPost(tester, user: memberUser);
    final delete = find.byTooltip('Delete');
    await tester.ensureVisible(delete);
    await tester.tap(delete);
    await tester.pumpAndSettle();
    expect(
        client.requests
            .map((r) => '${r.method} ${r.url.path}')
            .contains('DELETE /comments/1.json'),
        isTrue);
  });

  testWidgets('empty comment list shows placeholder', (tester) async {
    await openPost(tester, routes: {'GET /comments.json': <Object?>[]});
    expect(find.text('No comments yet'), findsOneWidget);
  });

  testWidgets('tag chip searches and navigates back', (tester) async {
    final (state, _) = await openPost(tester);
    await tester.tap(find.text('cat ears'));
    await tester.pumpAndSettle();
    expect(state.searchTags, 'cat_ears');
    expect(state.nav, NavDest.posts);
    expect(find.byType(PostsPage), findsOneWidget);
  });

  test('ratingName covers each rating', () {
    final l = lookupAppLocalizations(const Locale('en'));
    expect(ratingName(l, 'e'), l.ratingExplicit);
    expect(ratingName(l, 'q'), l.ratingQuestionable);
    expect(ratingName(l, 's'), l.ratingSafe);
    expect(ratingName(l, 'x'), l.ratingSafe);
  });

  testWidgets('unknown tag groups fall back to the general colour',
      (tester) async {
    final state = await pumpApp(tester);
    await tester.pumpAndSettle();
    final json = postJson(1);
    (json['tags'] as Map<String, Object?>)['mystery'] = ['odd_tag'];
    state.openPost(m.Post.fromV2(json));
    await tester.pumpAndSettle();
    expect(find.text('odd tag'), findsOneWidget);
  });
}
