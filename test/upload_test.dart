import 'dart:async';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:self26/models.dart';
import 'package:self26/pages/post_page.dart';
import 'package:self26/pages/upload_page.dart';
import 'package:self26/state.dart';

import 'helpers.dart';

void main() {
  test('default picker hits the platform plugin', () async {
    expect(() => pickUploadFile(), throwsA(anything));
  });

  Future<AppState> openUpload(
    WidgetTester tester, {
    User? user = memberUser,
    Map<String, Object?> routes = const {},
  }) async {
    final state =
        await pumpApp(tester, user: user, client: FakeClient(routes));
    state.select(NavDest.upload);
    await tester.pumpAndSettle();
    return state;
  }

  void fakePick(FilePickerResult? result) {
    final original = pickUploadFile;
    pickUploadFile = () async => result;
    addTearDown(() => pickUploadFile = original);
  }

  testWidgets('prompts sign in when logged out', (tester) async {
    await openUpload(tester, user: null);
    expect(find.text('Sign in to upload'), findsOneWidget);
  });

  testWidgets('picking a file and uploading opens the new post',
      (tester) async {
    fakePick(FilePickerResult([
      PlatformFile(
          name: 'pic.png', size: 4, bytes: Uint8List.fromList([1, 2, 3, 4])),
    ]));
    final state = await openUpload(tester);

    expect(find.text('Upload'), findsWidgets);
    await tester.tap(find.text('Choose file'));
    await tester.pumpAndSettle();
    expect(find.text('pic.png · 0 KB'), findsOneWidget);

    final fields = find.byType(TextField);
    await tester.enterText(fields.at(0), 'fox cute');
    await tester.enterText(fields.at(1), 'https://src');
    await tester.enterText(fields.at(2), 'a description');
    await tester.tap(find.text('Questionable'));
    await tester.pump();

    await tester.tap(find.widgetWithText(FilledButton, 'Upload'));
    await tester.pumpAndSettle();
    expect(state.selectedPost?.id, 1);
    expect(find.byType(PostPage), findsOneWidget);
  });

  testWidgets('upload failure shows the error message', (tester) async {
    fakePick(FilePickerResult([
      PlatformFile(name: 'pic.png', size: 1, bytes: Uint8List(1)),
    ]));
    await openUpload(tester, routes: {'POST /uploads.json': 500});
    await tester.tap(find.text('Choose file'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Upload'));
    await tester.pumpAndSettle();
    expect(find.text('Upload failed'), findsOneWidget);
  });

  testWidgets('upload shows a spinner while in flight', (tester) async {
    final gate = Completer<Object?>();
    fakePick(FilePickerResult([
      PlatformFile(name: 'pic.png', size: 1, bytes: Uint8List(1)),
    ]));
    await openUpload(
        tester, routes: {'POST /uploads.json': (request) => gate.future});
    await tester.tap(find.text('Choose file'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Upload'));
    await tester.pump();
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    gate.complete({'post_id': 1});
    await tester.pumpAndSettle();
  });

  testWidgets('cancelled picker leaves the form untouched', (tester) async {
    fakePick(null);
    await openUpload(tester);
    await tester.tap(find.text('Choose file'));
    await tester.pumpAndSettle();
    expect(find.text('Choose file'), findsOneWidget);
  });
}
