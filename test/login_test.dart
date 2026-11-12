import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:self26/pages/login_page.dart';
import 'package:self26/state.dart';

import 'helpers.dart';

void main() {
  Future<AppState> openLogin(
    WidgetTester tester, {
    Map<String, Object?> routes = const {},
  }) async {
    final state = await pumpApp(tester, client: FakeClient(routes));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.login));
    await tester.pumpAndSettle();
    expect(find.byType(LoginPage), findsOneWidget);
    return state;
  }

  testWidgets('password sign in succeeds and pops', (tester) async {
    final state = await openLogin(tester);
    final fields = find.byType(TextField);
    await tester.enterText(fields.at(0), 'alice');
    await tester.enterText(fields.at(1), 'pw');
    await tester.tap(find.widgetWithText(FilledButton, 'Sign in'));
    await tester.pumpAndSettle();
    expect(state.user?.name, 'alice');
    expect(find.byType(LoginPage), findsNothing);
  });

  testWidgets('password sign in can be submitted from the field',
      (tester) async {
    final state = await openLogin(tester);
    final fields = find.byType(TextField);
    await tester.enterText(fields.at(0), 'alice');
    await tester.enterText(fields.at(1), 'pw');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pumpAndSettle();
    expect(state.user?.name, 'alice');
  });

  testWidgets('bad credentials show the failure text', (tester) async {
    await openLogin(tester, routes: {'POST /session.json': 401});
    await tester.tap(find.widgetWithText(FilledButton, 'Sign in'));
    await tester.pumpAndSettle();
    expect(find.text('Sign in failed'), findsOneWidget);
    expect(find.byType(LoginPage), findsOneWidget);
  });

  testWidgets('api key sign in', (tester) async {
    final state = await openLogin(tester);
    final fields = find.byType(TextField);
    await tester.enterText(fields.at(2), 'alice');
    await tester.enterText(fields.at(3), 'key123');
    await tester.tap(find.widgetWithText(OutlinedButton, 'Use API key'));
    await tester.pumpAndSettle();
    expect(state.user?.name, 'alice');
    expect(find.byType(LoginPage), findsNothing);
  });

  testWidgets('api key failure shows the failure text', (tester) async {
    await openLogin(tester,
        routes: {'GET /users/alice.json': 401});
    final fields = find.byType(TextField);
    await tester.enterText(fields.at(2), 'alice');
    await tester.enterText(fields.at(3), 'bad');
    await tester.tap(find.widgetWithText(OutlinedButton, 'Use API key'));
    await tester.pumpAndSettle();
    expect(find.text('Sign in failed'), findsOneWidget);
  });

  testWidgets('busy state shows a spinner', (tester) async {
    final gate = Completer<Object?>();
    await openLogin(tester,
        routes: {'POST /session.json': (request) => gate.future});
    final fields = find.byType(TextField);
    await tester.enterText(fields.at(0), 'alice');
    await tester.enterText(fields.at(1), 'pw');
    await tester.tap(find.widgetWithText(FilledButton, 'Sign in'));
    await tester.pump();
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    gate.complete({'user': userJson()});
    await tester.pumpAndSettle();
  });
}
