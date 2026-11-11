import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:self26_landing/app.dart';
import 'package:self26_landing/landing_page.dart';
import 'package:self26_landing/state.dart';
import 'package:shared_preferences/shared_preferences.dart';

http.Client fakeReleasesClient({int status = 200, Object? body}) {
  return MockClient((req) async {
    if (status != 200) return http.Response('oops', status);
    return http.Response(jsonEncode(body ?? []), 200,
        headers: {'content-type': 'application/json'});
  });
}

const releaseJson = [
  {
    'name': 'v1.2.3',
    'tag_name': 'v1.2.3',
    'assets': {
      'links': [
        {'name': 'self26-linux-x86_64.tar.gz', 'url': 'https://example.com/a'},
        {'name': 'self26.apk', 'url': 'https://example.com/b'},
      ],
    },
  },
];

Future<LandingState> makeState() async {
  SharedPreferences.setMockInitialValues({});
  return LandingState(await SharedPreferences.getInstance());
}

void main() {
  testWidgets('landing page renders release assets', (tester) async {
    final state = await makeState();
    await tester.pumpWidget(LandingApp(
        state: state, client: fakeReleasesClient(body: releaseJson)));
    await tester.pumpAndSettle();

    expect(find.text('self26'), findsWidgets);
    expect(find.text('Your own e621-compatible media server.'),
        findsOneWidget);
    expect(find.text('self26-linux-x86_64.tar.gz'), findsOneWidget);
    expect(find.text('self26.apk'), findsOneWidget);
    expect(find.text('All releases on GitLab'), findsOneWidget);
  });

  testWidgets('shows empty state when no releases exist', (tester) async {
    final state = await makeState();
    await tester.pumpWidget(
        LandingApp(state: state, client: fakeReleasesClient(body: const [])));
    await tester.pumpAndSettle();

    expect(find.text('No releases published yet.'), findsWidgets);
  });

  testWidgets('shows error state and retries', (tester) async {
    final state = await makeState();
    var calls = 0;
    final client = MockClient((req) async {
      calls++;
      return http.Response('oops', 500);
    });
    await tester.pumpWidget(LandingApp(state: state, client: client));
    await tester.pumpAndSettle();

    expect(find.text('Could not load releases.'), findsOneWidget);
    await tester.tap(find.text('Retry'));
    await tester.pumpAndSettle();
    expect(calls, 2);
  });

  testWidgets('landing page golden', (tester) async {
    final loader = FontLoader('NotoSans')
      ..addFont(rootBundle.load('assets/fonts/NotoSans-Regular.ttf'))
      ..addFont(rootBundle.load('assets/fonts/NotoSans-Medium.ttf'))
      ..addFont(rootBundle.load('assets/fonts/NotoSans-Bold.ttf'));
    await loader.load();

    tester.view.physicalSize = const Size(1400, 1400);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    final state = await makeState();
    await tester.pumpWidget(LandingApp(
        state: state, client: fakeReleasesClient(body: releaseJson)));
    await tester.pumpAndSettle();

    await expectLater(
      find.byType(LandingPage),
      matchesGoldenFile('goldens/landing.png'),
    );
  });
}
