import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:self26/pages/login_page.dart';
import 'package:self26/shell.dart';
import 'package:self26/state.dart';

import 'helpers.dart';

void main() {
  testWidgets('desktop top bar and traffic lights', (tester) async {
    final actions = installFakeWindowActions();
    await pumpApp(tester, desktop: true);
    await tester.pumpAndSettle();

    expect(find.byType(DesktopTopBar), findsOneWidget);
    expect(find.byType(NavSidePanel), findsOneWidget);
    expect(find.byKey(const Key('nav_admin')), findsNothing);

    for (final k in ['window_close', 'window_minimize', 'window_maximize']) {
      final center = tester.getCenter(find.byKey(Key(k)));
      final gesture = await tester.startGesture(center);
      await tester.pump(const Duration(milliseconds: 50));
      await gesture.up();
      await tester.pump(const Duration(milliseconds: 500));
    }
    expect(actions.calls,
        ['close', 'minimize', 'toggleMaximize']);

    final topBar = find.byType(DesktopTopBar);
    final center = tester.getCenter(topBar);
    await tester.drag(topBar, const Offset(120, 0));
    await tester.tapAt(center);
    await tester.pump(const Duration(milliseconds: 60));
    await tester.tapAt(center);
    await tester.pump();
    expect(actions.calls,
        containsAllInOrder(['startDragging', 'toggleMaximize']));
    await tester.pump(const Duration(seconds: 2));
  });

  testWidgets('traffic light glyphs appear on hover', (tester) async {
    installFakeWindowActions();
    await pumpApp(tester, desktop: true);
    await tester.pumpAndSettle();

    final close = find.byKey(const Key('window_close'));
    final gesture =
        await tester.createGesture(kind: PointerDeviceKind.mouse);
    await gesture.addPointer(location: Offset.zero);
    addTearDown(gesture.removePointer);

    expect(find.byIcon(Icons.close), findsNothing);
    await gesture.moveTo(tester.getCenter(close));
    await tester.pump();
    expect(find.byIcon(Icons.close), findsOneWidget);
    await gesture.moveTo(Offset.zero);
    await tester.pump();
    expect(find.byIcon(Icons.close), findsNothing);
  });

  testWidgets('sidebar nav switches pages', (tester) async {
    installFakeWindowActions();
    final state = await pumpApp(tester, user: adminUser);
    await tester.pumpAndSettle();

    for (final dest in NavDest.values) {
      await tester.tap(find.byKey(Key('nav_${dest.name}')));
      await tester.pumpAndSettle();
      expect(state.nav, dest);
    }
    expect(find.byKey(const Key('nav_admin')), findsOneWidget);
  });

  testWidgets('account row signs in and out', (tester) async {
    installFakeWindowActions();
    final state = await pumpApp(tester);
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.login));
    await tester.pumpAndSettle();
    expect(state.nav, NavDest.settings);
    expect(find.byType(LoginPage), findsOneWidget);

    await tester.pageBack();
    await tester.pumpAndSettle();

    state.user = memberUser;
    state.notifyListeners();
    await tester.pumpAndSettle();
    expect(find.byIcon(Icons.account_circle_outlined), findsOneWidget);
    await tester.tap(find.byIcon(Icons.account_circle_outlined));
    await tester.pumpAndSettle();
    expect(state.user, isNull);
  });

  testWidgets('narrow layout uses a drawer', (tester) async {
    installFakeWindowActions();
    final state =
        await pumpApp(tester, size: const Size(500, 800));
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.menu), findsOneWidget);
    await tester.tap(find.byIcon(Icons.menu));
    await tester.pumpAndSettle();
    expect(find.byType(NavSidePanel), findsOneWidget);

    await tester.tap(find.byKey(const Key('nav_pools')));
    await tester.pumpAndSettle();
    expect(state.nav, NavDest.pools);
  });

  testWidgets('no desktop bar when not on desktop', (tester) async {
    installFakeWindowActions();
    await pumpApp(tester, desktop: false);
    await tester.pumpAndSettle();
    expect(find.byType(DesktopTopBar), findsNothing);
    expect(find.byType(NavSidePanel), findsOneWidget);
  });
}
