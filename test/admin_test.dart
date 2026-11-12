import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:self26/state.dart';

import 'helpers.dart';

void main() {
  testWidgets('non admin sees the lock state', (tester) async {
    final state = await pumpApp(tester);
    state.select(NavDest.admin);
    await tester.pumpAndSettle();
    expect(find.text('Administration'), findsOneWidget);
    expect(find.byIcon(Icons.lock_outline), findsOneWidget);
  });

  testWidgets('admin sees stats, flags and users', (tester) async {
    final client = FakeClient();
    final state =
        await pumpApp(tester, client: client, user: adminUser);
    state.select(NavDest.admin);
    await tester.pumpAndSettle();

    expect(find.text('Statistics'), findsOneWidget);
    expect(find.text('10'), findsOneWidget);
    expect(find.text('Posts'), findsWidgets);
    expect(find.text('1.9 GB'), findsOneWidget);

    expect(find.text('Flags'), findsOneWidget);
    expect(find.text('#1'), findsOneWidget);
    expect(find.text('spam'), findsOneWidget);
    await tester.tap(find.text('Resolve'));
    await tester.pumpAndSettle();
    expect(
        client.requests
            .map((r) => '${r.method} ${r.url.path}')
            .contains('POST /admin/api/flags/1/resolve.json'),
        isTrue);

    expect(find.text('Users'), findsWidgets);
    expect(find.text('alice'), findsOneWidget);
    expect(find.text('root'), findsOneWidget);
    expect(find.text('member'), findsOneWidget);
    expect(find.text('admin'), findsOneWidget);
  });

  testWidgets('empty flag list shows the placeholder', (tester) async {
    final state = await pumpApp(tester,
        user: adminUser,
        client: FakeClient({'GET /post_flags.json': <Object?>[]}));
    state.select(NavDest.admin);
    await tester.pumpAndSettle();
    expect(find.text('Nothing here yet'), findsOneWidget);
  });

  testWidgets('admin load errors name each section', (tester) async {
    final state = await pumpApp(tester,
        user: adminUser,
        client: FakeClient({
          'GET /admin/api/stats.json': 500,
          'GET /post_flags.json': 500,
          'GET /users.json': 500,
        }));
    state.select(NavDest.admin);
    await tester.pumpAndSettle();
    expect(find.text('Could not load Statistics'), findsOneWidget);
    expect(find.text('Could not load Flags'), findsOneWidget);
    expect(find.text('Could not load Users'), findsOneWidget);
  });
}
